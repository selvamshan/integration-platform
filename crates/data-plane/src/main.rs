use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use async_nats::Client as NatsClient;
use futures::StreamExt;

use common::{ConfigUpdate, FlowDefinition, Message, Connector};
use integration_runtime::FlowExecutor;
use integration_runtime::connectors::{http::HttpConnector, postgres::PostgresConnector};

struct AppState {
    executor: Arc<RwLock<FlowExecutor>>,
    flows: Arc<RwLock<std::collections::HashMap<String, FlowDefinition>>>,
    nats: NatsClient,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "data_plane=debug,integration_runtime=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting Data Plane with Event Subscription");

    // Connect to NATS
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://nats:4222".to_string());
    
    tracing::info!("Connecting to NATS at {}...", nats_url);
    let nats = async_nats::connect(&nats_url).await?;
    tracing::info!("✅ NATS connected");

    // Initialize flow executor
    let mut executor = FlowExecutor::new();
    
    // Register HTTP connector
    let mut http_connector = HttpConnector::new();
    http_connector.connect().await?;
    //http_connector.connect().await?;
    executor.register_connector("http".to_string(), Box::new(http_connector));
    
    // Register PostgreSQL connector
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://platform:platform123@postgres:5432/integration_platform".to_string());
    
    let mut postgres_connector = PostgresConnector::new(db_url);
    postgres_connector.connect().await?;
    executor.register_connector("postgres".to_string(), Box::new(postgres_connector));
    
    tracing::info!("✅ Connectors initialized");

    // Create application state
    let state = Arc::new(AppState {
        executor: Arc::new(RwLock::new(executor)),
        flows: Arc::new(RwLock::new(std::collections::HashMap::new())),
        nats: nats.clone(),
    });

    // Start NATS event listener
    let listener_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_config_updates(listener_state).await {
            tracing::error!("Config listener error: {}", e);
        }
    });

    // Build router
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/flows/:flow_id/execute", post(execute_flow))
        .route("/api/trigger/:path", get(trigger_flow))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🌐 Data Plane listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn listen_for_config_updates(state: Arc<AppState>) -> Result<()> {
    tracing::info!("🎧 Starting config event listener...");
    
    // Subscribe to all config events
    let mut subscriber = state.nats.subscribe("config.>").await?;
    
    tracing::info!("✅ Subscribed to config.* events");
    
    while let Some(message) = subscriber.next().await {
        let subject = message.subject.as_str();
        
        match serde_json::from_slice::<ConfigUpdate>(&message.payload) {
            Ok(event) => {
                tracing::info!("📥 Received event from {}: {:?}", subject, event);
                
                if let Err(e) = handle_config_update(&state, event).await {
                    tracing::error!("Failed to handle config update: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to deserialize config update: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn handle_config_update(state: &AppState, event: ConfigUpdate) -> Result<()> {
    match event {
        ConfigUpdate::FlowCreated { flow } => {
            tracing::info!("➕ Adding flow: {} ({})", flow.name, flow.id);
            let mut flows = state.flows.write().await;
            flows.insert(flow.id.clone(), flow);
            tracing::info!("✅ Flow registered in data plane");
        }
        
        ConfigUpdate::FlowUpdated { flow } => {
            tracing::info!("🔄 Updating flow: {} ({})", flow.name, flow.id);
            let mut flows = state.flows.write().await;
            flows.insert(flow.id.clone(), flow);
            tracing::info!("✅ Flow updated in data plane");
        }
        
        ConfigUpdate::FlowDeleted { flow_id } => {
            tracing::info!("➖ Removing flow: {}", flow_id);
            let mut flows = state.flows.write().await;
            flows.remove(&flow_id);
            tracing::info!("✅ Flow removed from data plane");
        }
        
        ConfigUpdate::ApiCreated { api } => {
            tracing::info!("📋 API registered: {} v{}", api.name, api.version);
        }
        
        _ => {
            tracing::debug!("Received other config event");
        }
    }
    
    Ok(())
}

async fn root() -> &'static str {
    "Data Plane - Integration Platform with Event Subscription"
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "data-plane",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn execute_flow(
    State(state): State<Arc<AppState>>,
    Path(flow_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    tracing::info!("📨 Received request to execute flow: {}", flow_id);
    
    // Get flow from distributed state
    let flow = {
        let flows = state.flows.read().await;
        flows.get(&flow_id).cloned()
    };
    
    let flow = flow.ok_or_else(|| AppError::NotFound(format!("Flow not found: {}. Create it in Control Plane first.", flow_id)))?;
    
    // Execute flow
    let input = Message::new(payload);
    
    let executor = state.executor.read().await;
    let result = executor.execute_flow(&flow, input).await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok(Json(json!({
        "flow_id": flow_id,
        "flow_name": flow.name,
        "status": "completed",
        "result": result.payload,
        "timestamp": result.timestamp
    })))
}

async fn trigger_flow(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<Value>, AppError> {
    tracing::info!("🎯 HTTP Trigger: GET /{}", path);
    
    // Look for a flow with matching HTTP trigger
    let flow = {
        let flows = state.flows.read().await;
        flows.values()
            .find(|f| {
                if let common::Trigger::Http { path: trigger_path, method } = &f.trigger {
                    method == "GET" && trigger_path.contains(&path)
                } else {
                    false
                }
            })
            .cloned()
    };
    
    let flow = if let Some(f) = flow {
        f
    } else {
        // If no flow found, create a default one (backward compatibility)
        tracing::warn!("No flow found for /{}, using default flow", path);
        create_default_flow(&path)
    };
    
    // Execute flow
    let input = Message::new(json!({
        "trigger": "http",
        "path": format!("/{}", path),
        "method": "GET"
    }));
    
    let executor = state.executor.read().await;
    let result = executor.execute_flow(&flow, input).await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok(Json(result.payload))
}

// Helper function for backward compatibility
fn create_default_flow(path: &str) -> FlowDefinition {
    use common::{Trigger, FlowStep};
    
    FlowDefinition {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("Default HTTP Trigger: {}", path),
        trigger: Trigger::Http {
            path: format!("/{}", path),
            method: "GET".to_string(),
        },
        steps: vec![
            FlowStep::Log {
                name: "trigger".to_string(),
                message: format!("HTTP GET triggered on /{}", path),
            },
            FlowStep::Call {
                name: "fetch_data".to_string(),
                connector: "postgres".to_string(),
                operation: "query".to_string(),
                params: json!({
                    "sql": "SELECT * FROM users LIMIT 10"
                }),
            },
            FlowStep::Log {
                name: "response".to_string(),
                message: "Returning data to client".to_string(),
            },
        ],
    }
}

enum AppError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
