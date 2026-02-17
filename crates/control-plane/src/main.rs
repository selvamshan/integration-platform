use anyhow::Result;
use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{State, Path, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sqlx::{PgPool, postgres::PgPoolOptions, Row};
use async_nats::Client as NatsClient;
use futures::StreamExt;
use jsonwebtoken::{encode, Header, EncodingKey};

use common::{
    ApiDefinition, 
    FlowDefinition, 
    Endpoint, 
    ConfigUpdate, 
    ConnectorDefinition, 
    TriggerDefinition, 
    Trigger,
    RateLimitEvent,
    JwtClaims,
};

type RedisConnection = redis::aio::ConnectionManager;

struct AppState {
    db: PgPool,
    nats: NatsClient,
    redis: RedisConnection,
    apis:             Arc<RwLock<Vec<ApiDefinition>>>,
    flows:            Arc<RwLock<Vec<FlowDefinition>>>,
    connectors:       Arc<RwLock<Vec<ConnectorDefinition>>>,
    triggers:         Arc<RwLock<Vec<TriggerDefinition>>>,
    rate_limit_stats: Arc<RwLock<HashMap<String, Vec<RateLimitEvent>>>>,
    jwt_secret:       String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "control_plane=debug,info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🎛️  Starting Control Plane with Connector/Trigger Registry");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://platform:platform123@postgres:5432/integration_platform".to_string());
    
    let db = PgPoolOptions::new().max_connections(10).connect(&database_url).await?;
    tracing::info!("✅ Database connected");

    // Connect to Redis
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://redis:6379".to_string());
    
    tracing::info!("Connecting to Redis at {}...", redis_url);
    let redis_client = redis::Client::open(redis_url)?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("✅ Redis connected");

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_string());
    let nats = async_nats::connect(&nats_url).await?;
    tracing::info!("✅ NATS connected");

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "integration-platform-secret-change-in-production".to_string());


    run_migrations(&db).await?;
    
    let state = Arc::new(AppState {
        db: db.clone(),
        nats,
        redis,
        apis: Arc::new(RwLock::new(Vec::new())),
        flows: Arc::new(RwLock::new(Vec::new())),
        connectors: Arc::new(RwLock::new(Vec::new())),
        triggers: Arc::new(RwLock::new(Vec::new())),
        rate_limit_stats: Arc::new(RwLock::new(HashMap::new())),
        jwt_secret,
    });

    load_flows_from_database(state.clone()).await?;
    initialize_builtin_registry(state.clone()).await?;

   // Start flow sync service - listens for Data Plane registration
    let sync_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = flow_sync_service(sync_state).await {
            tracing::error!("Flow sync service error: {}", e);
        }
    });

    // Start rate limit event listener
    let ratelimit_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = rate_limit_event_listener(ratelimit_state).await {
            tracing::error!("Rate limit event listener error: {}", e);
        }
    });

    // Background: validate client credentials for data-planes (NATS request-reply)
    let cred_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = credential_validation_service(cred_state).await {
            tracing::error!("Credential validation error: {}", e);
        }
    });

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        // API routes
        .route("/apis", get(list_apis).post(create_api))
        .route("/apis/:id", get(get_api))
        // Flow routes
        .route("/flows", get(list_flows).post(create_flow))
        .route("/flows/:id", get(get_flow).put(update_flow).delete(delete_flow))
        // Connector registry routes (for UI palette)
        .route("/connectors", get(list_connectors))
        .route("/connectors/:id", get(get_connector))
        // Trigger registry routes (for UI palette)
        .route("/triggers", get(list_triggers))
        .route("/triggers/:id", get(get_trigger))
        // ── Rate-limit stats ──────────────────────────────────────────────
        .route("/rate-limits", get(get_rate_limit_stats))
        .route("/rate-limits/:flow_id", get(get_flow_rate_limit_stats))
        // ── Auth: client management & token issuance ──────────────────────
        .route("/auth/clients",           post(create_client).get(list_clients))
        .route("/auth/clients/:client_id",get(get_client).delete(delete_client).patch(toggle_client))
        .route("/auth/token",             post(issue_token))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8081));
    tracing::info!("🌐 Control Plane listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn load_flows_from_database(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📥 Loading flows from database into memory...");
    
    let rows = sqlx::query("SELECT config FROM flow_definitions")
        .fetch_all(&state.db)
        .await?;
    
    let mut flows = state.flows.write().await;
    
    for row in rows {
        let config: serde_json::Value = row.try_get("config")?;
        if let Ok(flow) = serde_json::from_value::<FlowDefinition>(config) {
            tracing::info!("  ➕ Loaded: {} ({})", flow.name, flow.id);
            flows.push(flow);
        }
    }
    
    tracing::info!("✅ Loaded {} flows into memory", flows.len());
    Ok(())
}

// Flow Sync Service - listens for Data Plane registration and pushes all flows
async fn flow_sync_service(state: Arc<AppState>) -> Result<()> {
    tracing::info!("🔄 Starting Flow Sync Service...");
    
    // Subscribe to data plane registration requests
    let mut subscriber = state.nats.subscribe("dataplane.register").await?;
    
    tracing::info!("✅ Flow Sync Service listening for Data Plane registrations");
    
    while let Some(message) = subscriber.next().await {
        let node_id = String::from_utf8_lossy(&message.payload).to_string();
        
        tracing::info!("📡 Data Plane registered: {}", node_id);
        tracing::info!("📤 Pushing all flows to Data Plane: {}", node_id);
        
        // Get all flows
        let flows = state.flows.read().await;
        let flow_count = flows.len();
        
        // Push each flow to the Data Plane
        for flow in flows.iter() {
            let event = ConfigUpdate::FlowCreated { flow: flow.clone() };
            if let Err(e) = publish_event(&state.nats, &event).await {
                tracing::error!("Failed to push flow {}: {}", flow.id, e);
            } else {
                tracing::debug!("  ✅ Pushed: {} ({})", flow.name, flow.id);
            }
            
            // Small delay to avoid overwhelming the Data Plane
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        
        tracing::info!("✅ Successfully pushed {} flows to Data Plane: {}", flow_count, node_id);
    }
    
    Ok(())
}

async fn run_migrations(db: &PgPool) -> Result<()> {
    tracing::info!("Running database migrations...");
    
    sqlx::query("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name VARCHAR(255) NOT NULL, email VARCHAR(255) NOT NULL UNIQUE, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS api_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL, version VARCHAR(50) NOT NULL, base_path VARCHAR(255) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, UNIQUE(name, version))").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS flow_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS connector_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL UNIQUE, connector_type VARCHAR(100) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS trigger_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL UNIQUE, trigger_type VARCHAR(100) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
     // ── Auth table ────────────────────────────────────────────────────────────
    sqlx::query("CREATE TABLE IF NOT EXISTS client_credentials (
        client_id          VARCHAR(64)  PRIMARY KEY,
        client_secret_hash TEXT         NOT NULL,
        name               VARCHAR(255) NOT NULL,
        active             BOOLEAN      NOT NULL DEFAULT TRUE,
        created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
        expires_at         TIMESTAMPTZ
    )").execute(db).await?;
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(db).await?;
    if count.0 == 0 {
        sqlx::query("INSERT INTO users (name, email) VALUES ('Alice Johnson', 'alice@example.com'), ('Bob Smith', 'bob@example.com'), ('Charlie Brown', 'charlie@example.com'), ('Diana Prince', 'diana@example.com'), ('Eve Wilson', 'eve@example.com')").execute(db).await?;
        tracing::info!("✅ Sample data inserted");
    }
    
    tracing::info!("✅ Migrations completed");
    Ok(())
}

async fn initialize_builtin_registry(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📋 Initializing built-in connectors and triggers...");
    
    // Register HTTP connector
    let http_connector = ConnectorDefinition {
        id: "http-connector".to_string(),
        name: "HTTP/REST".to_string(),
        connector_type: "http".to_string(),
        description: "Make HTTP GET/POST requests to external APIs".to_string(),
        icon: Some("🌐".to_string()),
        operations: vec![
            common::ConnectorOperation {
                name: "get".to_string(),
                description: "Make HTTP GET request".to_string(),
                parameters: vec![
                    common::OperationParameter {
                        name: "url".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Target URL".to_string(),
                        default_value: None,
                    },
                ],
            },
            common::ConnectorOperation {
                name: "post".to_string(),
                description: "Make HTTP POST request".to_string(),
                parameters: vec![
                    common::OperationParameter {
                        name: "url".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Target URL".to_string(),
                        default_value: None,
                    },
                    common::OperationParameter {
                        name: "body".to_string(),
                        param_type: "object".to_string(),
                        required: false,
                        description: "Request body (JSON)".to_string(),
                        default_value: Some(json!({})),
                    },
                ],
            },
        ],
        config_schema: json!({"type": "object", "properties": {}}),
        enabled: true,
    };
    
    save_connector(&state, http_connector).await?;
    
    // Register PostgreSQL connector
    let postgres_connector = ConnectorDefinition {
        id: "postgres-connector".to_string(),
        name: "PostgreSQL".to_string(),
        connector_type: "postgres".to_string(),
        description: "Execute SQL queries on PostgreSQL database".to_string(),
        icon: Some("🐘".to_string()),
        operations: vec![
            common::ConnectorOperation {
                name: "query".to_string(),
                description: "Execute SELECT query".to_string(),
                parameters: vec![
                    common::OperationParameter {
                        name: "sql".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "SQL SELECT statement".to_string(),
                        default_value: None,
                    },
                ],
            },
            common::ConnectorOperation {
                name: "execute".to_string(),
                description: "Execute INSERT/UPDATE/DELETE".to_string(),
                parameters: vec![
                    common::OperationParameter {
                        name: "sql".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "SQL statement".to_string(),
                        default_value: None,
                    },
                ],
            },
        ],
        config_schema: json!({"type": "object", "properties": {"connection_string": {"type": "string"}}}),
        enabled: true,
    };
    
    save_connector(&state, postgres_connector).await?;
    
    // Register HTTP trigger
    let http_trigger = TriggerDefinition {
        id: "http-trigger".to_string(),
        name: "HTTP Request".to_string(),
        trigger_type: "http".to_string(),
        description: "Trigger flow on HTTP GET/POST request".to_string(),
        icon: Some("🌐".to_string()),
        config_schema: json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "URL path"},
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"]}
            },
            "required": ["path", "method"]
        }),
        enabled: true,
    };
    
    save_trigger(&state, http_trigger).await?;
    
    // Register Schedule trigger
    let schedule_trigger = TriggerDefinition {
        id: "schedule-trigger".to_string(),
        name: "Schedule".to_string(),
        trigger_type: "schedule".to_string(),
        description: "Trigger flow on schedule (cron)".to_string(),
        icon: Some("⏰".to_string()),
        config_schema: json!({
            "type": "object",
            "properties": {
                "cron": {"type": "string", "description": "Cron expression"}
            },
            "required": ["cron"]
        }),
        enabled: true,
    };
    
    save_trigger(&state, schedule_trigger).await?;
    
    tracing::info!("✅ Built-in registry initialized");
    Ok(())
}

async fn save_connector(state: &AppState, connector: ConnectorDefinition) -> Result<()> { 
    sqlx::query("INSERT INTO connector_definitions (name, connector_type, config) VALUES ($1, $2, $3) ON CONFLICT (name) DO NOTHING")
        .bind(&connector.name)
        .bind(&connector.connector_type)
        .bind(serde_json::to_value(&connector)?)
        .execute(&state.db)
        .await?;
    
    let mut connectors = state.connectors.write().await;
    if !connectors.iter().any(|c| c.id == connector.id) {
        connectors.push(connector.clone());
    }
    Ok(())
}

async fn save_trigger(state: &AppState, trigger: TriggerDefinition) -> Result<()> {

    sqlx::query("INSERT INTO trigger_definitions (name, trigger_type, config) VALUES ($1, $2, $3) ON CONFLICT (name) DO NOTHING")
        .bind(&trigger.name)
        .bind(&trigger.trigger_type)
        .bind(serde_json::to_value(&trigger)?)
        .execute(&state.db)
        .await?;
    
    let mut triggers = state.triggers.write().await;
    if !triggers.iter().any(|t| t.id == trigger.id) {
        triggers.push(trigger.clone());
    }
    Ok(())
}

async fn root() -> &'static str {
    "Control Plane - Integration Platform with Registry"
}

async fn health_check() -> Json<Value> {
    Json(json!({"status": "healthy", "service": "control-plane", "timestamp": chrono::Utc::now().to_rfc3339()}))
}

// Connector Registry endpoints
async fn list_connectors(State(state): State<Arc<AppState>>) -> Json<Value> {
    let connectors = state.connectors.read().await;
    Json(json!({"connectors": *connectors, "count": connectors.len()}))
}

async fn get_connector(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let connectors = state.connectors.read().await;
    let connector = connectors.iter().find(|c| c.id == id).ok_or_else(|| AppError::NotFound("Connector not found".to_string()))?;
    Ok(Json(json!(connector)))
}

// Trigger Registry endpoints
async fn list_triggers(State(state): State<Arc<AppState>>) -> Json<Value> {
    let triggers = state.triggers.read().await;
    Json(json!({"triggers": *triggers, "count": triggers.len()}))
}

async fn get_trigger(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let triggers = state.triggers.read().await;
    let trigger = triggers.iter().find(|t| t.id == id).ok_or_else(|| AppError::NotFound("Trigger not found".to_string()))?;
    Ok(Json(json!(trigger)))
}

// API endpoints
async fn list_apis(State(state): State<Arc<AppState>>) -> Json<Value> {
    let apis = state.apis.read().await;
    Json(json!({"apis": *apis, "count": apis.len()}))
}

#[derive(Deserialize)]
struct CreateApiRequest {
    name: String,
    version: String,
    base_path: String,
    endpoints: Vec<EndpointRequest>,
}

#[derive(Deserialize)]
struct EndpointRequest {
    path: String,
    method: String,
    flow_id: String,
}

async fn create_api(State(state): State<Arc<AppState>>, Json(req): Json<CreateApiRequest>) -> Result<Json<Value>, AppError> {
    tracing::info!("📡 Creating API: {} v{}", req.name, req.version);
    
    let api_id = uuid::Uuid::new_v4().to_string();
    let api = ApiDefinition {
        id: api_id.clone(),
        name: req.name,
        version: req.version,
        base_path: req.base_path,
        endpoints: req.endpoints.into_iter().map(|e| Endpoint {
            path: e.path,
            method: e.method,
            flow_id: e.flow_id,
        }).collect(),
    };
    
    sqlx::query("INSERT INTO api_definitions (id, name, version, base_path, config) VALUES ($1, $2, $3, $4, $5)")
        .bind(uuid::Uuid::parse_str(&api.id).unwrap())
        .bind(&api.name)
        .bind(&api.version)
        .bind(&api.base_path)
        .bind(serde_json::to_value(&api).unwrap())
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut apis = state.apis.write().await;
    apis.push(api.clone());
    drop(apis);
    
    let event = ConfigUpdate::ApiCreated { api: api.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ API created: {}", api_id);
    Ok(Json(json!(api)))
}

async fn get_api(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let apis = state.apis.read().await;
    let api = apis.iter().find(|a| a.id == id).ok_or_else(|| AppError::NotFound("API not found".to_string()))?;
    Ok(Json(json!(api)))
}

// Flow endpoints
async fn list_flows(State(state): State<Arc<AppState>>) -> Json<Value> {
    let flows = state.flows.read().await;
    Json(json!({"flows": *flows, "count": flows.len()}))
}

async fn create_flow(State(state): State<Arc<AppState>>, Json(flow): Json<FlowDefinition>) -> Result<Json<Value>, AppError> {
    tracing::info!("📡 Creating flow: {}", flow.name);
    
    sqlx::query("INSERT INTO flow_definitions (name, config) VALUES ($1, $2)")
        .bind(&flow.name)
        .bind(serde_json::to_value(&flow).unwrap())
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    flows.push(flow.clone());
    drop(flows);
    
    // Auto-create or update API definition for HTTP triggers
    if let Trigger::Http { path, method } = &flow.trigger {
        auto_update_api_definition(&state, &flow, path, method).await?;
    }
    
    let event = ConfigUpdate::FlowCreated { flow: flow.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow created and API auto-updated: {}", flow.id);
    Ok(Json(json!(flow)))
}

async fn update_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(flow): Json<FlowDefinition>) -> Result<Json<Value>, AppError> {
    tracing::info!("🔄 Updating flow: {}", id);
    
    if flow.id != id {
        return Err(AppError::Internal("Flow ID mismatch".to_string()));
    }
    
    sqlx::query("UPDATE flow_definitions SET name = $1, config = $2 WHERE id = $3")
        .bind(&flow.name)
        .bind(serde_json::to_value(&flow).unwrap())
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    if let Some(existing) = flows.iter_mut().find(|f| f.id == id) {
        *existing = flow.clone();
    }
    drop(flows);
    
    // Auto-update API definition
    if let Trigger::Http { path, method } = &flow.trigger {
        auto_update_api_definition(&state, &flow, path, method).await?;
    }
    
    let event = ConfigUpdate::FlowUpdated { flow: flow.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow updated and API auto-updated: {}", id);
    Ok(Json(json!(flow)))
}

async fn get_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let flows = state.flows.read().await;
    let flow = flows.iter().find(|f| f.id == id).ok_or_else(|| AppError::NotFound("Flow not found".to_string()))?;
    Ok(Json(json!(flow)))
}

async fn delete_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    tracing::info!("🗑️  Deleting flow: {}", id);
    
    // Get flow before deleting to update API
    let flow = {
        let flows = state.flows.read().await;
        flows.iter().find(|f| f.id == id).cloned()
    };
    
    sqlx::query("DELETE FROM flow_definitions WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    flows.retain(|f| f.id != id);
    drop(flows);
    
    // Remove from API definition
    if let Some(flow) = flow {
        if let Trigger::Http { path, .. } = &flow.trigger {
            remove_from_api_definition(&state, &id, path).await?;
        }
    }
    
    let event = ConfigUpdate::FlowDeleted { flow_id: id.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow deleted and API updated: {}", id);
    Ok(Json(json!({"deleted": true, "flow_id": id})))
}

// Auto-update API definition when flow changes
async fn auto_update_api_definition(state: &AppState, flow: &FlowDefinition, path: &str, method: &str) -> Result<(), AppError> {
    tracing::info!("🔄 Auto-updating API definition for flow: {}", flow.id);
    
    let api_name = flow.name.clone();
    let api_version = "1.0";
    
    let mut apis = state.apis.write().await;
    
    // Find or create auto-generated API
    if let Some(api) = apis.iter_mut().find(|a| a.name == api_name && a.version == api_version) {
        // Update existing endpoint or add new
        if let Some(endpoint) = api.endpoints.iter_mut().find(|e| e.path == path && e.method == method) {
            endpoint.flow_id = flow.id.clone();
        } else {
            api.endpoints.push(Endpoint {
                path: path.to_string(),
                method: method.to_string(),
                flow_id: flow.id.clone(),
            });
        }
        
        // Save to DB
        sqlx::query("UPDATE api_definitions SET config = $1 WHERE id = $2")
            .bind(serde_json::to_value(&*api).unwrap())
            .bind(uuid::Uuid::parse_str(&api.id).unwrap())
            .execute(&state.db)
            .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
        
        tracing::info!("✅ Updated existing API definition");
    } else {
        // Create new auto-generated API
        let api_id = uuid::Uuid::new_v4().to_string();
        let new_api = ApiDefinition {
            id: api_id.clone(),
            name: api_name.to_string(),
            version: api_version.to_string(),
            base_path: "/api".to_string(),
            endpoints: vec![Endpoint {
                path: path.to_string(),
                method: method.to_string(),
                flow_id: flow.id.clone(),
            }],
        };

        tracing::info!("api_definitions id {}", uuid::Uuid::parse_str(&new_api.id).unwrap());
        
        sqlx::query("INSERT INTO api_definitions (id, name, version, base_path, config) VALUES ($1, $2, $3, $4, $5)")
            .bind(uuid::Uuid::parse_str(&new_api.id).unwrap())
            .bind(&new_api.name)
            .bind(&new_api.version)
            .bind(&new_api.base_path)
            .bind(serde_json::to_value(&new_api).unwrap())
            .execute(&state.db)
            .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
        
        apis.push(new_api.clone());
        
        tracing::info!("✅ Created new API definition");
    }
    
    Ok(())
}

async fn remove_from_api_definition(state: &AppState, flow_id: &str, path: &str) -> Result<(), AppError> {
    let mut apis = state.apis.write().await;
    
    for api in apis.iter_mut() {
        api.endpoints.retain(|e| e.flow_id != flow_id);
        
        // Update in DB
        sqlx::query("UPDATE api_definitions SET config = $1 WHERE id = $2")
            .bind(serde_json::to_value(&*api).unwrap())
            .bind(uuid::Uuid::parse_str(&api.id).unwrap())
            .execute(&state.db)
            .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    }
    
    Ok(())
}

async fn publish_event(nats: &NatsClient, event: &ConfigUpdate) -> Result<(), AppError> {
    let subject = event.subject();
    let payload = serde_json::to_vec(event).map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;
    nats.publish(subject, payload.into()).await.map_err(|e| AppError::Internal(format!("NATS publish error: {}", e)))?;
    tracing::debug!("📤 Published event to {}", subject);
    Ok(())
}

// Rate limit event listener
async fn rate_limit_event_listener(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📊 Starting Rate Limit Event Listener...");
    
    let mut subscriber = state.nats.subscribe("ratelimit.event").await?;
    
    tracing::info!("✅ Subscribed to ratelimit.event");
    
    while let Some(message) = subscriber.next().await {
        match serde_json::from_slice::<RateLimitEvent>(&message.payload) {
            Ok(event) => {
                if !event.allowed {
                    tracing::warn!(
                        "🚫 Rate limit exceeded: flow={}, key={}, count={}/{}", 
                        event.flow_id, event.key, event.current_count, event.limit
                    );
                } else {
                    tracing::debug!(
                        "✅ Rate limit check: flow={}, count={}/{}", 
                        event.flow_id, event.current_count, event.limit
                    );
                }
                
                // Store event for statistics (keep last 1000 per flow)
                let mut stats = state.rate_limit_stats.write().await;
                let flow_events = stats.entry(event.flow_id.clone()).or_insert_with(Vec::new);
                flow_events.push(event);
                
                // Keep only last 1000 events per flow
                if flow_events.len() > 1000 {
                    flow_events.drain(0..flow_events.len() - 1000);
                }
            }
            Err(e) => {
                tracing::error!("Failed to deserialize rate limit event: {}", e);
            }
        }
    }
    
    Ok(())
}

// Get all rate limit statistics
async fn get_rate_limit_stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    let stats = state.rate_limit_stats.read().await;
    
    let mut summary = serde_json::Map::new();
    
    for (flow_id, events) in stats.iter() {
        let total = events.len();
        let blocked = events.iter().filter(|e| !e.allowed).count();
        let allowed = events.iter().filter(|e| e.allowed).count();
        
        summary.insert(flow_id.clone(), json!({
            "total_requests": total,
            "allowed": allowed,
            "blocked": blocked,
            "block_rate": if total > 0 { (blocked as f64 / total as f64 * 100.0) } else { 0.0 }
        }));
    }
    
    Json(json!({
        "flows": summary,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Get rate limit statistics for specific flow
async fn get_flow_rate_limit_stats(
    State(state): State<Arc<AppState>>,
    Path(flow_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let stats = state.rate_limit_stats.read().await;
    
    let events = stats.get(&flow_id)
        .ok_or_else(|| AppError::NotFound(format!("No rate limit stats for flow: {}", flow_id)))?;
    
    let total = events.len();
    let blocked = events.iter().filter(|e| !e.allowed).count();
    let allowed = events.iter().filter(|e| e.allowed).count();
    
    // Get last 10 events
    let recent_events: Vec<&RateLimitEvent> = events.iter()
        .rev()
        .take(10)
        .collect();
    
    // Group by key to see which keys are hitting limits
    let mut key_stats = std::collections::HashMap::new();
    for event in events {
        let entry = key_stats.entry(event.key.clone()).or_insert((0, 0));
        if event.allowed {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }
    
    Ok(Json(json!({
        "flow_id": flow_id,
        "summary": {
            "total_requests": total,
            "allowed": allowed,
            "blocked": blocked,
            "block_rate": if total > 0 { (blocked as f64 / total as f64 * 100.0) } else { 0.0 }
        },
        "by_key": key_stats.iter().map(|(k, (a, b))| {
            json!({
                "key": k,
                "allowed": a,
                "blocked": b
            })
        }).collect::<Vec<_>>(),
        "recent_events": recent_events,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

// ─── Error type ──────────────────────────────────────────────────────────────
enum AppError {
    NotFound(String),
    Internal(String),
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
        };
        (status, Json(json!({"error": message}))).into_response()
    }
}


impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Internal(msg) => write!(f, "{msg}"),
            AppError::NotFound(msg) =>  write!(f, "{msg}"),
            AppError::Unauthorized(msg) => write!(f, "{msg}"),
        }
    }
}

// ─── Credential validation service (NATS request-reply) ──────────────────────

async fn credential_validation_service(state: Arc<AppState>) -> Result<()> {
    let mut subscriber = state.nats.subscribe("auth.validate.credentials").await?;
    tracing::info!("🔐 Credential validation service started");

    while let Some(msg) = subscriber.next().await {
        let reply = match msg.reply { Some(r) => r, None => continue };

        let response = match do_validate_credentials(&state, &msg.payload).await {
            Ok(p) => {
                tracing::debug!("✅ Credential OK: {}", p.client_id);
                json!({ "valid": true, "client_id": p.client_id, "name": p.client_name })
            }
            Err(reason) => {
                tracing::warn!("❌ Credential rejected: {}", reason);
                json!({ "valid": false, "reason": reason })
            }
        };

        let _ = state.nats.publish(reply, serde_json::to_vec(&response).unwrap_or_default().into()).await;
    }
    Ok(())
}

async fn do_validate_credentials(
    state: &AppState,
    payload: &[u8],
) -> std::result::Result<common::AuthPrincipal, String> {
    #[derive(Deserialize)]
    struct Req { client_id: String, client_secret: String }

    let req: Req = serde_json::from_slice(payload).map_err(|_| "Malformed request".to_string())?;

    let row = sqlx::query!(
        "SELECT client_id, client_secret_hash, name, active, expires_at
         FROM client_credentials WHERE client_id = $1", req.client_id
    )
    .fetch_optional(&state.db).await.map_err(|e| e.to_string())?
    .ok_or_else(|| "Client not found".to_string())?;

    if !row.active { return Err("Client is deactivated".into()); }
    if let Some(exp) = row.expires_at {
        if exp < chrono::Utc::now() { return Err("Credential expired".into()); }
    }
    if !bcrypt::verify(&req.client_secret, &row.client_secret_hash).map_err(|e| e.to_string())? {
        return Err("Invalid secret".into());
    }

    Ok(common::AuthPrincipal {
        client_id:   row.client_id,
        client_name: row.name,
        auth_method: common::AuthMethod::ClientCredentials,
    })
}

// ─── Auth: client CRUD ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateClientBody {
    name: String,
    expires_in_days: Option<i64>,
}

/// POST /auth/clients — create credentials; plain secret returned once
async fn create_client(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateClientBody>,
) -> Result<Json<Value>, AppError> {
    let client_id  = format!("cid_{}", uuid::Uuid::new_v4().simple());
    let raw_secret = format!("cs_{}", uuid::Uuid::new_v4().simple());
    let hash = bcrypt::hash(&raw_secret, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let expires_at: Option<chrono::DateTime<chrono::Utc>> =
        body.expires_in_days.map(|d| chrono::Utc::now() + chrono::Duration::days(d));

    sqlx::query!(
        "INSERT INTO client_credentials (client_id, client_secret_hash, name, active, expires_at)
         VALUES ($1, $2, $3, TRUE, $4)",
        client_id, hash, body.name, expires_at
    )
    .execute(&state.db).await.map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("🔑 Created client: {} ({})", body.name, client_id);

    Ok(Json(json!({
        "client_id":     client_id,
        "client_secret": raw_secret,   // shown ONCE — store securely
        "name":          body.name,
        "active":        true,
        "expires_at":    expires_at,
        "warning":       "Store client_secret now — it will not be shown again"
    })))
}

/// GET /auth/clients
async fn list_clients(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query!(
        "SELECT client_id, name, active, created_at, expires_at
         FROM client_credentials ORDER BY created_at DESC"
    )
    .fetch_all(&state.db).await.map_err(|e| AppError::Internal(e.to_string()))?;

    let clients: Vec<Value> = rows.iter().map(|r| json!({
        "client_id":  r.client_id,
        "name":       r.name,
        "active":     r.active,
        "created_at": r.created_at,
        "expires_at": r.expires_at,
    })).collect();

    Ok(Json(json!({ "clients": clients, "count": clients.len() })))
}

/// GET /auth/clients/:client_id
async fn get_client(
    State(state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query!(
        "SELECT client_id, name, active, created_at, expires_at
         FROM client_credentials WHERE client_id = $1", client_id
    )
    .fetch_optional(&state.db).await.map_err(|e| AppError::Internal(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Client not found: {}", client_id)))?;

    Ok(Json(json!({
        "client_id":  row.client_id,
        "name":       row.name,
        "active":     row.active,
        "created_at": row.created_at,
        "expires_at": row.expires_at,
    })))
}

/// DELETE /auth/clients/:client_id
async fn delete_client(
    State(state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let res = sqlx::query!(
        "DELETE FROM client_credentials WHERE client_id = $1", client_id
    )
    .execute(&state.db).await.map_err(|e| AppError::Internal(e.to_string()))?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Client not found: {}", client_id)));
    }
    tracing::info!("🗑️  Deleted client: {}", client_id);
    Ok(Json(json!({ "deleted": client_id })))
}

/// PATCH /auth/clients/:client_id — activate or deactivate
#[derive(Deserialize)]
struct ToggleBody { active: bool }

async fn toggle_client(
    State(state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
    Json(body): Json<ToggleBody>,
) -> Result<Json<Value>, AppError> {
    let res = sqlx::query!(
        "UPDATE client_credentials SET active = $1 WHERE client_id = $2",
        body.active, client_id
    )
    .execute(&state.db).await.map_err(|e| AppError::Internal(e.to_string()))?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Client not found: {}", client_id)));
    }
    let status = if body.active { "activated" } else { "deactivated" };
    tracing::info!("🔄 Client {} {}", client_id, status);
    Ok(Json(json!({ "client_id": client_id, "active": body.active })))
}

// ─── Auth: token issuance ─────────────────────────────────────────────────────

const TOKEN_EXPIRY_SECS: i64 = 3600; // 1 hour

/// POST /auth/token — exchange client_id + secret for a signed JWT
async fn issue_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<common::TokenRequest>,
) -> Result<Json<Value>, AppError> {
    // 1. Fetch record
    let row = sqlx::query!(
        "SELECT client_id, client_secret_hash, name, active, expires_at
         FROM client_credentials WHERE client_id = $1", body.client_id
    )
    .fetch_optional(&state.db).await.map_err(|e| AppError::Internal(e.to_string()))?
    .ok_or_else(|| AppError::Unauthorized("Invalid credentials".into()))?;

    // 2. Active check
    if !row.active {
        return Err(AppError::Unauthorized("Client is deactivated".into()));
    }

    // 3. Expiry check
    if let Some(exp) = row.expires_at {
        if exp < chrono::Utc::now() {
            return Err(AppError::Unauthorized("Credential has expired".into()));
        }
    }

    // 4. Secret verification
    let ok = bcrypt::verify(&body.client_secret, &row.client_secret_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !ok {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    // 5. Sign JWT
    let now = chrono::Utc::now().timestamp();
    let claims = JwtClaims {
        sub:         row.client_id.clone(),
        client_name: row.name.clone(),
        iat:         now,
        exp:         now + TOKEN_EXPIRY_SECS,
        jti:         uuid::Uuid::new_v4().to_string(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ).map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("🎫 Token issued for: {} ({})", row.name, row.client_id);

    Ok(Json(json!({
        "access_token": token,
        "token_type":   "Bearer",
        "expires_in":   TOKEN_EXPIRY_SECS,
        "client_id":    row.client_id,
    })))
}