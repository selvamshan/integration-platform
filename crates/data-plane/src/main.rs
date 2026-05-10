use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
    http::{header, HeaderName, HeaderValue, Method},
    middleware,
};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use integration_runtime::FlowExecutor;

mod auth;
mod circuit_breaker;
mod connector_registry;
mod error;
mod executor;
mod handlers;
mod metrics;
mod rate_limit;
mod retry;
mod scheduler;
mod services;
mod startup;
mod state;

use auth::{auth_middleware, AuthConfig};
use circuit_breaker::{circuit_breaker_middleware, circuit_breaker_status};
use connector_registry::{ConnectorRegistry, CryptoService};
use handlers::{execute_flow, health_check, list_flows, root, trigger_flow};
use metrics::{metrics_handler, metrics_middleware, register_metrics};
use rate_limit::rate_limit_middleware;
use scheduler::FlowScheduler;
use services::config_listener::listen_for_config_updates;
use services::connector_listener::listen_for_connector_instances;
use startup::{load_scheduled_flows, register_with_control_plane};
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "data_plane=debug,integration_runtime=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting Data Plane with Metrics & Rate Limiting");

    register_metrics();
    tracing::info!("✅ Metrics registered");

    let node_id = format!("data-plane-{}", uuid::Uuid::new_v4());
    tracing::info!("📋 Node ID: {}", node_id);

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://redis:6379".to_string());
    tracing::info!("Connecting to Redis at {}...", redis_url);
    let redis_client = redis::Client::open(redis_url)?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("✅ Redis connected");

    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://nats:4222".to_string());
    tracing::info!("Connecting to NATS at {}...", nats_url);
    let nats = async_nats::connect(&nats_url).await?;
    tracing::info!("✅ NATS connected");

    let executor = FlowExecutor::new();

    let scheduler = Arc::new(FlowScheduler::new("UTC").await?);

    // Only one instance should run scheduled flows. Acquire a Redis lock with a
    // 30-second TTL and renew it in the background; if this node loses the lock
    // (e.g. after a crash + restart) another instance will take over.
    let is_scheduler_leader = {
        let mut conn = redis.clone();
        let acquired: bool = redis::cmd("SET")
            .arg("scheduler-leader")
            .arg(&node_id)
            .arg("NX")
            .arg("PX")
            .arg(30_000u64)
            .query_async(&mut conn)
            .await
            .unwrap_or(false);
        if acquired {
            let mut renew_conn = redis.clone();
            let renew_id = node_id.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
                loop {
                    interval.tick().await;
                    let result: redis::RedisResult<()> = redis::cmd("PEXPIRE")
                        .arg("scheduler-leader")
                        .arg(30_000u64)
                        .query_async(&mut renew_conn)
                        .await;
                    if result.is_err() {
                        tracing::warn!("Scheduler leader lock renewal failed for {}", renew_id);
                    }
                }
            });
        }
        acquired
    };

    if is_scheduler_leader {
        scheduler.start().await?;
        tracing::info!("✅ Flow scheduler started (leader: {})", node_id);
    } else {
        tracing::info!("⏭️  Flow scheduler skipped (not leader)");
    }

    tracing::info!("✅ HTTP connector initialized (DB connectors registered on-demand)");

    let crypto = Arc::new(CryptoService::new()?);
    let connector_registry = Arc::new(ConnectorRegistry::new(crypto));
    tracing::info!("✅ Connector registry initialized");

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "integration-platform-secret-change-in-production".to_string());

    let state = Arc::new(AppState {
        executor:           Arc::new(RwLock::new(executor)),
        flows:              Arc::new(RwLock::new(HashMap::new())),
        circuit_breakers:   Arc::new(RwLock::new(HashMap::new())),
        connector_registry: connector_registry.clone(),
        nats:               nats.clone(),
        redis,
        node_id:            node_id.clone(),
        jwt_secret:         jwt_secret.clone(),
        scheduler,
    });

    load_scheduled_flows(&state).await?;

    let listener_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_config_updates(listener_state).await {
            tracing::error!("Config listener error: {}", e);
        }
    });

    let connector_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_connector_instances(connector_state).await {
            tracing::error!("Connector instance listener error: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    register_with_control_plane(state.clone()).await?;

    let auth_cfg = Arc::new(AuthConfig {
        jwt_secret,
        nats: nats.clone(),
    });

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderName::from_static("x-client-id"),
            HeaderName::from_static("x-client-secret"),
        ])
        .allow_credentials(true);

    let protected = Router::new()
        .route("/flows/:flow_id/execute", post(execute_flow))
        .route("/api/trigger/*path", get(trigger_flow).post(trigger_flow).put(trigger_flow).delete(trigger_flow))
        .layer(middleware::from_fn_with_state(state.clone(), circuit_breaker_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(middleware::from_fn_with_state(auth_cfg, auth_middleware));

    let public = Router::new()
        .route("/",                get(root))
        .route("/health",          get(health_check))
        .route("/metrics",         get(metrics_handler))
        .route("/circuit-breakers", get(circuit_breaker_status))
        .route("/flows",           get(list_flows));

    let scheduler_handle = state.scheduler.clone();

    let app = public
        .merge(protected)
        .layer(middleware::from_fn(metrics_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🌐 Data Plane listening on {}", addr);
    tracing::info!("🔐 Auth: Client-Credentials + JWT Bearer supported");
    tracing::info!("📊 Metrics: http://{}:{}/metrics", addr.ip(), addr.port());
    tracing::info!("🔌 Circuit Breakers: http://{}:{}/circuit-breakers", addr.ip(), addr.port());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    scheduler_handle.shutdown().await?;
    Ok(())
}
