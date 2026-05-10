use anyhow::Result;
use axum::{
    Router,
    routing::{delete, get, post},
    http::{header, HeaderValue, Method},
    middleware,
};
use serde_json::{json, Value};
use axum::Json;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sqlx::postgres::PgPoolOptions;

mod state;
mod error;
mod crypto;
mod keycloak;
mod oidc;
mod rbac;
mod transformers;
mod handlers;
mod audit;
mod db_migrations;
mod services;
mod startup;

use crypto::CryptoService;
use oidc::OidcAuth;
use rbac::{permission_middleware, rbac_middleware};
use audit::AuditLogger;
use state::AppState;
use db_migrations::run_migrations;

use handlers::api::{list_apis, create_api, get_api};
use handlers::connector::{list_connectors, get_connector, list_triggers, get_trigger};
use handlers::connector_instance::{
    create_connector_instance, list_connector_instances, get_connector_instance,
    list_connector_instances_by_type, test_connector_instance,
    delete_connector_instance, update_connector_instance,
};
use handlers::auth::{create_client, list_clients, get_client, delete_client, toggle_client, issue_token};
use handlers::user::{invite_user, list_users, delete_user, get_current_user};
use handlers::rate_limit::{get_rate_limit_stats, get_flow_rate_limit_stats};
use handlers::flow::{list_flows, test_flow, get_flow, create_flow, delete_flow, update_flow};
use handlers::project::{list_projects, create_project, get_project, delete_project, list_project_flows};

use services::flow_sync::flow_sync_service;
use services::connector_sync::connector_instance_sync_service;
use services::rate_limit_listener::rate_limit_event_listener;
use services::credential_validation::credential_validation_service;

use startup::{
    load_flows_from_database, load_apis_from_database,
    load_connector_instances, initialize_builtin_registry,
};

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

    let crypto = Arc::new(CryptoService::new()?);
    tracing::info!("✅ Encryption service initialized");

    let oidc = Arc::new(OidcAuth::from_env());

    run_migrations(&db).await?;

    let state = Arc::new(AppState {
        db: db.clone(),
        nats,
        redis,
        apis:                Arc::new(RwLock::new(Vec::new())),
        flows:               Arc::new(RwLock::new(Vec::new())),
        connectors:          Arc::new(RwLock::new(Vec::new())),
        triggers:            Arc::new(RwLock::new(Vec::new())),
        connector_instances: Arc::new(RwLock::new(Vec::new())),
        rate_limit_stats:    Arc::new(RwLock::new(HashMap::new())),
        jwt_secret,
        crypto,
        oidc: oidc.clone(),
        audit_logger: Arc::new(AuditLogger::new(db.clone())),
    });

    load_flows_from_database(state.clone()).await?;
    load_apis_from_database(state.clone()).await?;
    load_connector_instances(&state).await?;
    initialize_builtin_registry(state.clone()).await?;

    let sync_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = flow_sync_service(sync_state).await {
            tracing::error!("Flow sync service error: {}", e);
        }
    });

    let connector_sync_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = connector_instance_sync_service(connector_sync_state).await {
            tracing::error!("Connector instance sync error: {}", e);
        }
    });

    let ratelimit_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = rate_limit_event_listener(ratelimit_state).await {
            tracing::error!("Rate limit event listener error: {}", e);
        }
    });

    let cred_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = credential_validation_service(cred_state).await {
            tracing::error!("Credential validation error: {}", e);
        }
    });

    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let cors = CorsLayer::new()
        .allow_origin(frontend_url.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::X_CONTENT_TYPE_OPTIONS,
        ])
        .allow_credentials(true);

    let app = Router::new()
        .route("/",       get(root))
        .route("/health", get(health_check))
        // API routes
        .route("/apis",     get(list_apis).post(create_api))
        .route("/apis/:id", get(get_api))
        // Project routes
        .route("/projects",             get(list_projects).post(create_project))
        .route("/projects/:id",         get(get_project).delete(delete_project))
        .route("/projects/:id/flows",   get(list_project_flows))
        // Flow routes
        .route("/flows",     get(list_flows).post(create_flow))
        .route("/flows/:id", get(get_flow).put(update_flow).delete(delete_flow))
        .route("/flows/test", post(test_flow))
        // Transformer routes
        .route("/transformers",              get(transformers::list_transformers))
        .route("/transformers/:id",          get(transformers::get_transformer))
        .route("/transformers/capabilities", get(transformers::get_transformer_capabilities))
        // Connector registry routes
        .route("/connectors",     get(list_connectors))
        .route("/connectors/:id", get(get_connector))
        // Trigger registry routes
        .route("/triggers",     get(list_triggers))
        .route("/triggers/:id", get(get_trigger))
        // Rate-limit stats
        .route("/rate-limits",          get(get_rate_limit_stats))
        .route("/rate-limits/:flow_id", get(get_flow_rate_limit_stats))
        // Auth: client management & token issuance
        .route("/auth/clients",              post(create_client).get(list_clients))
        .route("/auth/clients/:client_id",   get(get_client).delete(delete_client).patch(toggle_client))
        .route("/auth/token",                post(issue_token))
        // Connector instances
        .route("/connector-instances",                          post(create_connector_instance).get(list_connector_instances))
        .route("/connector-instances/test",                     post(test_connector_instance))
        .route("/connector-instances/:id",                      get(get_connector_instance).put(update_connector_instance).delete(delete_connector_instance))
        .route("/connector-instances/type/:connector_type",     get(list_connector_instances_by_type))
        // User management
        .route("/users/invite",   post(invite_user))
        .route("/users",          get(list_users))
        .route("/users/me",       get(get_current_user))
        .route("/users/:user_id", delete(delete_user))
        // Audit logs
        .route("/audit-logs",                         get(handlers::audit::list_audit_logs))
        .route("/flows/:id/audit-logs",               get(handlers::audit::get_flow_audit_logs))
        .route("/connector-instances/:id/audit-logs", get(handlers::audit::get_connector_audit_logs))
        // Middleware
        .layer(middleware::from_fn(permission_middleware))
        .layer(middleware::from_fn_with_state(oidc.clone(), rbac_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::new())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8081));
    tracing::info!("🌐 Control Plane listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> &'static str {
    "Control Plane - Integration Platform with Registry"
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status":    "healthy",
        "service":   "control-plane",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
