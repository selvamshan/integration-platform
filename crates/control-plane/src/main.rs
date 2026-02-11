use anyhow::Result;
use axum::{
    Router,
    routing::{get, post, delete},
    extract::{State, Path, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sqlx::{PgPool, postgres::PgPoolOptions};
use async_nats::Client as NatsClient;

use common::{ApiDefinition, FlowDefinition, Endpoint, ConfigUpdate};

struct AppState {
    db: PgPool,
    nats: NatsClient,
    apis: Arc<RwLock<Vec<ApiDefinition>>>,
    flows: Arc<RwLock<Vec<FlowDefinition>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "control_plane=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🎛️  Starting Control Plane with Event Distribution");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://platform:platform123@postgres:5432/integration_platform".to_string());
    
    tracing::info!("Connecting to database...");
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;
    
    tracing::info!("✅ Database connected");

    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://nats:4222".to_string());
    
    tracing::info!("Connecting to NATS at {}...", nats_url);
    let nats = async_nats::connect(&nats_url).await?;
    tracing::info!("✅ NATS connected");

    run_migrations(&db).await?;

    let state = Arc::new(AppState {
        db,
        nats,
        apis: Arc::new(RwLock::new(Vec::new())),
        flows: Arc::new(RwLock::new(Vec::new())),
    });

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/apis", get(list_apis).post(create_api))
        .route("/apis/:id", get(get_api))
        .route("/flows", get(list_flows).post(create_flow))
        .route("/flows/:id", get(get_flow).delete(delete_flow))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8081));
    tracing::info!("🌐 Control Plane listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn run_migrations(db: &PgPool) -> Result<()> {
    tracing::info!("Running database migrations...");
    
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL UNIQUE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    "#).execute(db).await?;
    
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS api_definitions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            version VARCHAR(50) NOT NULL,
            base_path VARCHAR(255) NOT NULL,
            config JSONB NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(name, version)
        )
    "#).execute(db).await?;
    
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS flow_definitions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            config JSONB NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    "#).execute(db).await?;
    
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(db).await?;
    
    if count.0 == 0 {
        tracing::info!("Inserting sample data...");
        sqlx::query(r#"
            INSERT INTO users (name, email) VALUES
            ('Alice Johnson', 'alice@example.com'),
            ('Bob Smith', 'bob@example.com'),
            ('Charlie Brown', 'charlie@example.com'),
            ('Diana Prince', 'diana@example.com'),
            ('Eve Wilson', 'eve@example.com')
        "#).execute(db).await?;
        tracing::info!("✅ Sample data inserted");
    }
    
    tracing::info!("✅ Migrations completed");
    Ok(())
}

async fn root() -> &'static str {
    "Control Plane - Integration Platform with Event Distribution"
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "control-plane",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

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

async fn create_api(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApiRequest>,
) -> Result<Json<Value>, AppError> {
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
    
    sqlx::query(r#"INSERT INTO api_definitions (id, name, version, base_path, config) VALUES ($1, $2, $3, $4, $5)"#)
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
    
    tracing::info!("✅ API created and published: {}", api_id);
    Ok(Json(json!(api)))
}

async fn get_api(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let apis = state.apis.read().await;
    let api = apis.iter().find(|a| a.id == id).ok_or_else(|| AppError::NotFound("API not found".to_string()))?;
    Ok(Json(json!(api)))
}

async fn list_flows(State(state): State<Arc<AppState>>) -> Json<Value> {
    let flows = state.flows.read().await;
    Json(json!({"flows": *flows, "count": flows.len()}))
}

async fn create_flow(State(state): State<Arc<AppState>>, Json(flow): Json<FlowDefinition>) -> Result<Json<Value>, AppError> {
    tracing::info!("📡 Creating flow: {}", flow.name);
    
    sqlx::query(r#"INSERT INTO flow_definitions (name, config) VALUES ($1, $2)"#)
        .bind(&flow.name)
        .bind(serde_json::to_value(&flow).unwrap())
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    flows.push(flow.clone());
    drop(flows);
    
    let event = ConfigUpdate::FlowCreated { flow: flow.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow created and published: {}", flow.id);
    Ok(Json(json!(flow)))
}

async fn get_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let flows = state.flows.read().await;
    let flow = flows.iter().find(|f| f.id == id).ok_or_else(|| AppError::NotFound("Flow not found".to_string()))?;
    Ok(Json(json!(flow)))
}

async fn delete_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    tracing::info!("🗑️  Deleting flow: {}", id);
    
    sqlx::query("DELETE FROM flow_definitions WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    flows.retain(|f| f.id != id);
    drop(flows);
    
    let event = ConfigUpdate::FlowDeleted { flow_id: id.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow deleted and event published: {}", id);
    Ok(Json(json!({"deleted": true, "flow_id": id})))
}

async fn publish_event(nats: &NatsClient, event: &ConfigUpdate) -> Result<(), AppError> {
    let subject = event.subject();
    let payload = serde_json::to_vec(event).map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;
    nats.publish(subject, payload.into()).await.map_err(|e| AppError::Internal(format!("NATS publish error: {}", e)))?;
    tracing::debug!("📤 Published event to {}", subject);
    Ok(())
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
        (status, Json(json!({"error": message}))).into_response()
    }
}
