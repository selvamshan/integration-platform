use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Json, Query},
    response::{IntoResponse, Response},
    http::{StatusCode, Request, HeaderMap},
    middleware::{self, Next},
    body::Body,
};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use async_nats::Client as NatsClient;
use futures::StreamExt;
use redis::AsyncCommands;
use prometheus::{
    Encoder, 
    TextEncoder, 
    Counter, 
    Histogram, 
    IntGauge, 
    IntGaugeVec,
    Registry, 
    HistogramOpts, 
    Opts,
};
use lazy_static::lazy_static;

use common::{
    Message, 
    FlowDefinition, 
    ConfigUpdate, 
    RateLimitPolicy, 
    RateLimitKeyType, 
    RateLimitEvent,
    CircuitBreakerPolicy, 
    CircuitState,   
};
use integration_runtime::FlowExecutor;
//use integration_runtime::connectors::{http::HttpConnector, postgres::PostgresConnector};

mod retry;
mod auth;
mod connector_registry;
mod scheduler;
//use retry::with_retry;
use auth::{auth_middleware, AuthConfig};
use connector_registry::{ConnectorRegistry, CryptoService};
use scheduler::FlowScheduler;

type RedisConnection = redis::aio::ConnectionManager;

// Circuit breaker state tracking
#[derive(Debug, Clone)]
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: u64,
    opened_at: u64,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: 0,
            opened_at: 0,
        }
    }
}

// Prometheus metrics
lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    
    static ref HTTP_REQUESTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("http_requests_total", "Total number of HTTP requests")
    ).unwrap();
    
    static ref HTTP_REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration in seconds")
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    ).unwrap();
    
    static ref FLOW_EXECUTIONS_TOTAL: Counter = Counter::with_opts(
        Opts::new("flow_executions_total", "Total number of flow executions")
    ).unwrap();
    
    static ref FLOW_EXECUTIONS_SUCCESS: Counter = Counter::with_opts(
        Opts::new("flow_executions_success_total", "Total number of successful flow executions")
    ).unwrap();
    
    static ref FLOW_EXECUTIONS_FAILED: Counter = Counter::with_opts(
        Opts::new("flow_executions_failed_total", "Total number of failed flow executions")
    ).unwrap();
    
    static ref FLOW_EXECUTION_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new("flow_execution_duration_seconds", "Flow execution duration in seconds")
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0])
    ).unwrap();
    
    static ref RATE_LIMIT_CHECKS_TOTAL: Counter = Counter::with_opts(
        Opts::new("rate_limit_checks_total", "Total number of rate limit checks")
    ).unwrap();
    
    static ref RATE_LIMIT_BLOCKED_TOTAL: Counter = Counter::with_opts(
        Opts::new("rate_limit_blocked_total", "Total number of blocked requests due to rate limiting")
    ).unwrap();
    
    static ref RATE_LIMIT_ALLOWED_TOTAL: Counter = Counter::with_opts(
        Opts::new("rate_limit_allowed_total", "Total number of allowed requests after rate limit check")
    ).unwrap();
    
    static ref FLOWS_LOADED: IntGauge = IntGauge::with_opts(
        Opts::new("flows_loaded", "Number of flows currently loaded")
    ).unwrap();
    
    static ref REDIS_OPERATIONS_TOTAL: Counter = Counter::with_opts(
        Opts::new("redis_operations_total", "Total number of Redis operations")
    ).unwrap();
    
    static ref REDIS_ERRORS_TOTAL: Counter = Counter::with_opts(
        Opts::new("redis_errors_total", "Total number of Redis errors")
    ).unwrap();
    
    // Circuit breaker metrics
    static ref CIRCUIT_BREAKER_STATE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("circuit_breaker_state", "Circuit breaker state by flow (0=closed, 1=open, 2=half_open)"),
        &["flow_id"]
    ).unwrap();
    
    static ref CIRCUIT_BREAKER_OPENS_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_opens_total", "Total number of circuit breaker opens")
    ).unwrap();
    
    static ref CIRCUIT_BREAKER_CLOSES_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_closes_total", "Total number of circuit breaker closes")
    ).unwrap();
    
    static ref CIRCUIT_BREAKER_HALF_OPENS_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_half_opens_total", "Total number of circuit breaker half-opens")
    ).unwrap();
    
    static ref CIRCUIT_BREAKER_REJECTED_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_rejected_total", "Total number of requests rejected by circuit breaker")
    ).unwrap();

    // Retry metrics
    static ref RETRY_ATTEMPTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("retry_attempts_total", "Total number of retry attempts")
    ).unwrap();
    
    static ref RETRY_SUCCESS_TOTAL: Counter = Counter::with_opts(
        Opts::new("retry_success_total", "Total number of successful retries")
    ).unwrap();
    
    static ref RETRY_EXHAUSTED_TOTAL: Counter = Counter::with_opts(
        Opts::new("retry_exhausted_total", "Total number of exhausted retries")
    ).unwrap();
}

fn register_metrics() {
    REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_REQUEST_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(FLOW_EXECUTIONS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(FLOW_EXECUTIONS_SUCCESS.clone())).unwrap();
    REGISTRY.register(Box::new(FLOW_EXECUTIONS_FAILED.clone())).unwrap();
    REGISTRY.register(Box::new(FLOW_EXECUTION_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(RATE_LIMIT_CHECKS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(RATE_LIMIT_BLOCKED_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(RATE_LIMIT_ALLOWED_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(FLOWS_LOADED.clone())).unwrap();
    REGISTRY.register(Box::new(REDIS_OPERATIONS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(REDIS_ERRORS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(CIRCUIT_BREAKER_STATE.clone())).unwrap();
    REGISTRY.register(Box::new(CIRCUIT_BREAKER_OPENS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(CIRCUIT_BREAKER_CLOSES_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(CIRCUIT_BREAKER_HALF_OPENS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(CIRCUIT_BREAKER_REJECTED_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(RETRY_ATTEMPTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(RETRY_SUCCESS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(RETRY_EXHAUSTED_TOTAL.clone())).unwrap();
}

struct AppState {
    executor: Arc<RwLock<FlowExecutor>>,
    flows: Arc<RwLock<std::collections::HashMap<String, FlowDefinition>>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreakerState>>>,
    connector_registry: Arc<ConnectorRegistry>,
    nats: NatsClient,
    redis: RedisConnection,
    node_id: String,
    jwt_secret: String,
    scheduler:Arc<FlowScheduler>,
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

    tracing::info!("🚀 Starting Data Plane with Metrics & Rate Limiting");

    // Register Prometheus metrics
    register_metrics();
    tracing::info!("✅ Metrics registered");

    // Generate unique node ID
    let node_id = format!("data-plane-{}", uuid::Uuid::new_v4());
    tracing::info!("📋 Node ID: {}", node_id);
    
     // Connect to Redis
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://redis:6379".to_string());
    
    tracing::info!("Connecting to Redis at {}...", redis_url);
    let redis_client = redis::Client::open(redis_url)?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("✅ Redis connected");


    // Connect to NATS
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://nats:4222".to_string());
    
    tracing::info!("Connecting to NATS at {}...", nats_url);
    let nats = async_nats::connect(&nats_url).await?;
    tracing::info!("✅ NATS connected");

    // Initialize flow executor
    let executor = FlowExecutor::new();
    
    let scheduler = Arc::new(FlowScheduler::new("UTC").await?);
    scheduler.start().await?;
    tracing::info!("✅ Flow scheduler started");

   tracing::info!("✅ HTTP connector initialized (DB connectors registered on-demand)");

   // Initialize crypto service for decrypting connector passwords
    let crypto = Arc::new(CryptoService::new()?);
    let connector_registry = Arc::new(ConnectorRegistry::new(crypto));
    tracing::info!("✅ Connector registry initialized");


    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "integration-platform-secret-change-in-production".to_string());

    // Create application state
    let state = Arc::new(AppState {
        executor: Arc::new(RwLock::new(executor)),
        flows: Arc::new(RwLock::new(std::collections::HashMap::new())),
        circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
        connector_registry: connector_registry.clone(),
        nats: nats.clone(),
        redis,
        node_id: node_id.clone(),
        jwt_secret: jwt_secret.clone(),
        scheduler,
    });

    // Load and schedule flows
    load_scheduled_flows(&state).await?;

    // Start NATS event listener
    let listener_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_config_updates(listener_state).await {
            tracing::error!("Config listener error: {}", e);
        }
    });

    // Listen for connector instance updates from Control Plane
    let connector_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_connector_instances(connector_state).await {
            tracing::error!("Connector instance listener error: {}", e);
        }
    });

        // Wait a moment for subscription to be ready
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Register with Control Plane to receive all flows
    register_with_control_plane(state.clone()).await?;

    // Auth config – shared by the auth middleware
    let auth_cfg = Arc::new(AuthConfig {
        jwt_secret,
        nats: nats.clone(),
    });

    // Protected routes: require authentication
    let protected = Router::new()
        .route("/flows/:flow_id/execute", post(execute_flow))
        .route("/api/trigger/*path", get(trigger_flow).post(trigger_flow).put(trigger_flow).delete(trigger_flow))
        .layer(middleware::from_fn_with_state(state.clone(), circuit_breaker_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(middleware::from_fn_with_state(auth_cfg, auth_middleware));

    // Public routes: no authentication required
    let public = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/circuit-breakers", get(circuit_breaker_status))
        .route("/flows", get(list_flows));

    let scheduler = state.scheduler.clone();

    let app = public
        .merge(protected)
        .layer(middleware::from_fn(metrics_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🌐 Data Plane listening on {}", addr);
    tracing::info!("🔐 Auth: Client-Credentials + JWT Bearer supported");
    tracing::info!("📊 Metrics: http://{}:{}/metrics", addr.ip(), addr.port());
    tracing::info!("🔌 Circuit Breakers: http://{}:{}/circuit-breakers", addr.ip(), addr.port());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    // Cleanup on shutdown
    scheduler.shutdown().await?;

    Ok(())
}



fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// Circuit breaker middleware
async fn circuit_breaker_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    
    // Extract flow ID
    let flow_id = if path.starts_with("/flows/") && path.ends_with("/execute") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 {
            Some(parts[2].to_string())
        } else {
            None
        }
    } else if path.starts_with("/api/trigger/") {
        // HTTP trigger: /api/trigger/{path}
        let trigger_path = path.strip_prefix("/api/trigger/").unwrap_or("");
        let flows = state.flows.read().await;
        
        // Find flow matching this trigger path
        flows.values()
            .find(|f| {
                if let common::Trigger::Http { path: trigger_path_def, method } = &f.trigger {
                    method == "GET" && trigger_path_def.contains(trigger_path)
                } else {
                    false
                }
            })
            .map(|f| f.id.clone())
    } else {
        None
    };

    if let Some(ref flow_id_str) = flow_id {
        let cb_policy = {
            let flows = state.flows.read().await;

            flows
                .get(flow_id_str)
                .and_then(|flow| flow.circuit_breaker.clone())
        }; // flows dropped here automatically

        if let Some(cb_policy) = cb_policy {
            // Now safe — no borrow of flows exists

            let mut circuit_breakers = state.circuit_breakers.write().await;
            let cb_state = circuit_breakers
                .entry(flow_id_str.clone())
                .or_insert_with(CircuitBreakerState::new);

            let now = current_timestamp();

            match cb_state.state {
                CircuitState::Open => {
                    if now - cb_state.opened_at >= cb_policy.timeout_seconds {
                        cb_state.state = CircuitState::HalfOpen;
                        cb_state.success_count = 0;

                        CIRCUIT_BREAKER_HALF_OPENS_TOTAL.inc();
                        CIRCUIT_BREAKER_STATE
                            .with_label_values(&[&flow_id_str.clone()])
                            .set(2);

                        tracing::info!(
                            "🔄 Circuit breaker HALF-OPEN for flow: {}",
                            flow_id_str.clone()
                        );
                    } else {
                        CIRCUIT_BREAKER_REJECTED_TOTAL.inc();

                        tracing::warn!(
                            "🔌 Circuit breaker OPEN - rejecting request for flow: {}",
                            flow_id_str
                        );

                        return (
                            StatusCode::SERVICE_UNAVAILABLE,
                            Json(json!({
                                "error": "Circuit breaker is open - service temporarily unavailable",
                                "flow_id": flow_id_str,
                                "state": "open",
                                "retry_after_seconds":
                                    cb_policy.timeout_seconds - (now - cb_state.opened_at)
                            })),
                        )
                            .into_response();
                    }
                }
                _ => {}
            }
        }
    }


    next.run(request).await
}

async fn update_circuit_breaker_on_success(
    state: Arc<AppState>,
    flow_id: String,
    policy: CircuitBreakerPolicy,
) {
    tokio::spawn(async move {
        let mut circuit_breakers = state.circuit_breakers.write().await;

        let cb_state = circuit_breakers
            .entry(flow_id.clone())
            .or_insert_with(CircuitBreakerState::new);

        match cb_state.state {
            CircuitState::HalfOpen => {
                cb_state.success_count += 1;

                if cb_state.success_count >= policy.success_threshold {
                    cb_state.state = CircuitState::Closed;
                    cb_state.failure_count = 0;
                    cb_state.success_count = 0;

                    CIRCUIT_BREAKER_CLOSES_TOTAL.inc();
                    CIRCUIT_BREAKER_STATE
                        .with_label_values(&[&flow_id])
                        .set(0);

                    tracing::info!(
                        "✅ Circuit breaker CLOSED for flow: {}",
                        flow_id
                    );
                }
            }

            CircuitState::Closed => {
                cb_state.failure_count = 0;
            }

            _ => {}
        }
    });
}


async fn update_circuit_breaker_on_failure(
    state: Arc<AppState>,
    flow_id: String,
    policy: CircuitBreakerPolicy,
) {
    tokio::spawn(async move {
        let mut circuit_breakers = state.circuit_breakers.write().await;

        let cb_state = circuit_breakers
            .entry(flow_id.clone())
            .or_insert_with(CircuitBreakerState::new);

        let now = current_timestamp();

        match cb_state.state {
            CircuitState::Closed => {
                cb_state.failure_count += 1;
                cb_state.last_failure_time = now;

                if cb_state.failure_count >= policy.failure_threshold {
                    cb_state.state = CircuitState::Open;
                    cb_state.opened_at = now;

                    CIRCUIT_BREAKER_OPENS_TOTAL.inc();
                    CIRCUIT_BREAKER_STATE
                        .with_label_values(&[&flow_id])
                        .set(1);

                    tracing::error!(
                        "🔴 Circuit breaker OPEN for flow: {} (failures: {})",
                        flow_id,
                        cb_state.failure_count
                    );
                }
            }

            CircuitState::HalfOpen => {
                cb_state.state = CircuitState::Open;
                cb_state.opened_at = now;
                cb_state.success_count = 0;

                CIRCUIT_BREAKER_OPENS_TOTAL.inc();
                CIRCUIT_BREAKER_STATE
                    .with_label_values(&[&flow_id])
                    .set(1);

                tracing::error!(
                    "🔴 Circuit breaker re-OPEN for flow: {} (failed in half-open)",
                    flow_id
                );
            }

            _ => {}
        }
    });
}

// Circuit breaker status endpoint
async fn circuit_breaker_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let circuit_breakers = state.circuit_breakers.read().await;
    let flows = state.flows.read().await;
    
    let mut status = Vec::new();
    
    for (flow_id, cb_state) in circuit_breakers.iter() {
        let policy = flows.get(flow_id).and_then(|f| f.circuit_breaker.as_ref());
        
        status.push(json!({
            "flow_id": flow_id,
            "state": match cb_state.state {
                CircuitState::Closed => "closed",
                CircuitState::Open => "open",
                CircuitState::HalfOpen => "half_open",
            },
            "failure_count": cb_state.failure_count,
            "success_count": cb_state.success_count,
            "policy": policy
        }));
    }
    
    Json(json!({
        "circuit_breakers": status,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Metrics middleware
async fn metrics_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = request.uri().path().to_string();
    
    HTTP_REQUESTS_TOTAL.inc();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed().as_secs_f64();
    HTTP_REQUEST_DURATION.observe(duration);
    
    tracing::debug!("Request to {} took {:.3}s", path, duration);
    
    response
}

// Metrics endpoint
async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics".to_string(),
        );
    }
    
    match String::from_utf8(buffer) {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(e) => {
            tracing::error!("Failed to convert metrics to string: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to convert metrics".to_string(),
            )
        }
    }
}


// Rate limiting middleware
async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Extract client IP from headers (X-Forwarded-For or X-Real-IP) or use default
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();
    
    // Extract flow ID from path
    let path = request.uri().path();
    let flow_id = if path.starts_with("/flows/") && path.ends_with("/execute") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 {
            Some(parts[2].to_string())
        } else {
            None
        }
    } else if path.starts_with("/api/trigger/") {
        // For trigger endpoints, try to find matching flow
        let trigger_path = path.strip_prefix("/api/trigger/").unwrap_or("");
        let flows = state.flows.read().await;
        flows.values()
            .find(|f| {
                if let common::Trigger::Http { path: trigger_path_def, method } = &f.trigger {
                    method == "GET" && trigger_path_def.contains(trigger_path)
                } else {
                    false
                }
            })
            .map(|f| f.id.clone())
    } else {
        None
    };

    // If we have a flow ID, check rate limit
    if let Some(ref flow_id_str) = flow_id {
        let flows = state.flows.read().await;
        if let Some(flow) = flows.get(flow_id_str) {
            if let Some(rate_limit) = &flow.rate_limit {
                // Generate rate limit key based on policy
                let key = generate_rate_limit_key(flow_id_str, rate_limit, &client_ip);
                
                // Check rate limit
                match check_rate_limit(&state, flow_id_str, &key, rate_limit).await {
                    Ok(allowed) => {
                        if !allowed {
                            tracing::warn!("🚫 Rate limit exceeded for flow {} (key: {})", flow_id_str, key);
                            
                            let message = rate_limit.message.as_ref()
                                .map(|m| m.clone())
                                .unwrap_or_else(|| format!("Rate limit exceeded: {} requests per {} seconds", 
                                    rate_limit.max_requests, rate_limit.window_seconds));
                            
                            return (
                                StatusCode::TOO_MANY_REQUESTS,
                                Json(json!({
                                    "error": message,
                                    "flow_id": flow_id_str,
                                    "limit": rate_limit.max_requests,
                                    "window_seconds": rate_limit.window_seconds
                                }))
                            ).into_response();
                        } else {
                            tracing::debug!("✅ Rate limit check passed for flow {} (key: {})", flow_id_str, key);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Rate limit check error: {}", e);
                        // On error, allow the request (fail open)
                    }
                }
            }
        }
    }

    next.run(request).await
}

fn generate_rate_limit_key(flow_id: &str, policy: &RateLimitPolicy, client_ip: &str) -> String {
    match policy.key_type {
        RateLimitKeyType::Global => format!("ratelimit:global:{}", flow_id),
        RateLimitKeyType::PerIp => format!("ratelimit:ip:{}:{}", client_ip, flow_id),
        RateLimitKeyType::PerFlow => format!("ratelimit:flow:{}", flow_id),
        RateLimitKeyType::PerUser => {
            // TODO For now, use IP as user identifier (in real impl, extract from auth token)
            format!("ratelimit:user:{}:{}", client_ip, flow_id)
        }
    }
}

async fn check_rate_limit(
    state: &AppState,
    flow_id: &str,
    key: &str,
    policy: &RateLimitPolicy,
) -> Result<bool> {
    let mut redis = state.redis.clone();
    
    // Use Redis INCR with EXPIRE for sliding window
    let count: u32 = redis.incr(key, 1).await?;
   // let count: u32 = redis.incr::<_, _, u32>(key, 1).await?;
    
    // Set expiry on first request
    if count == 1 {
        //redis.expire(key, policy.window_seconds as i64).await?;
        let _: () = redis
            .expire::<_, ()>(key, policy.window_seconds as i64)
            .await?;
    }
    
    let allowed = count <= policy.max_requests;
    
    // Send rate limit event to Control Plane
    let event = RateLimitEvent {
        flow_id: flow_id.to_string(),
        key: key.to_string(),
        timestamp: chrono::Utc::now(),
        allowed,
        current_count: count,
        limit: policy.max_requests,
    };
    
    // Publish to Control Plane (fire and forget)
    let nats = state.nats.clone();
    tokio::spawn(async move {
        let payload = serde_json::to_vec(&event).unwrap();
        let _ = nats.publish("ratelimit.event", payload.into()).await;
    });
    
    Ok(allowed)
}

async fn register_with_control_plane(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📡 Registering with Control Plane...");   
    // Send registration message to Control Plane   
    state.nats.publish(
        "dataplane.register",
        state.node_id.clone().into_bytes().into()
    ).await?;
    
    tracing::info!("✅ Registration sent to Control Plane");
    tracing::info!("⏳ Waiting for flows to be pushed from Control Plane...");
    
    // Wait a bit for flows to arrive
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let flow_count = state.flows.read().await.len();
    tracing::info!("✅ Received {} flows from Control Plane", flow_count);
    
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
                
                if let Err(e) = handle_config_update(state.clone(), event).await {
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

async fn handle_config_update(state: Arc<AppState>, event: ConfigUpdate) -> Result<()> {
    match event {
        ConfigUpdate::FlowCreated { flow } | ConfigUpdate::FlowUpdated { flow } => {
            let is_update = state.flows.read().await.contains_key(&flow.id);
            tracing::info!("{} flow: {} ({})", if is_update { "🔄 Updating" } else { "➕ Adding" }, flow.name, flow.id);

            // Schedule if it's a schedule-triggered flow
            if let common::Trigger::Schedule { cron } = &flow.trigger {
                let state_clone = state.clone();
                let executor = move |flow_id: String, context: Value| {
                    let s = state_clone.clone();
                    tokio::spawn(async move {
                        execute_flow_inner(&s, &flow_id, context).await
                    })
                };
                state.scheduler.schedule_flow(
                    flow.id.clone(),
                    flow.name.clone(),
                    cron,
                    executor,
                ).await?;
            }

            state.flows.write().await.insert(flow.id.clone(), flow);
            tracing::info!("✅ Flow registered in data plane");
        }

        ConfigUpdate::FlowDeleted { flow_id } => {
            tracing::info!("➖ Removing flow: {}", flow_id);
            // Unschedule if it was a scheduled flow
            let _ = state.scheduler.unschedule_flow(&flow_id).await;
            state.flows.write().await.remove(&flow_id);
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

async fn list_flows(State(state): State<Arc<AppState>>) -> Json<Value> {
    let flows = state.flows.read().await;
    let flow_list: Vec<&FlowDefinition> = flows.values().collect();
    Json(json!({
        "flows": flow_list,
        "count": flows.len(),
        "node_id": state.node_id
    }))
}

// Retry logic with exponential backoff
async fn execute_with_retry<F, Fut>(
    retry_policy: &common::RetryPolicy,
    flow_id: &str,
    mut operation: F,
) -> Result<Message, common::Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Message, common::Error>>,
{
    let mut attempt = 0;
    let mut delay_ms = retry_policy.initial_delay_ms;
    
    loop {
        attempt += 1;
        
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    RETRY_SUCCESS_TOTAL.inc();
                    tracing::info!("✅ Flow {} succeeded on attempt {}/{}", 
                        flow_id, attempt, retry_policy.max_attempts);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt >= retry_policy.max_attempts {
                    RETRY_EXHAUSTED_TOTAL.inc();
                    tracing::error!("❌ Flow {} failed after {} attempts: {}", 
                        flow_id, attempt, e);
                    return Err(e);
                }
                
                RETRY_ATTEMPTS_TOTAL.inc();
                
                // Calculate delay with exponential backoff
                let actual_delay = if retry_policy.jitter {
                    // Add jitter: 50-100% of calculated delay
                    let jitter_factor = 0.5 + (rand::random::<f64>() * 0.5);
                    (delay_ms as f64 * jitter_factor) as u64
                } else {
                    delay_ms
                };
                
                tracing::warn!("🔄 Flow {} failed on attempt {}/{}, retrying in {}ms: {}", 
                    flow_id, attempt, retry_policy.max_attempts, actual_delay, e);
                
                tokio::time::sleep(tokio::time::Duration::from_millis(actual_delay)).await;
                
                // Calculate next delay with backoff multiplier
                delay_ms = ((delay_ms as f64) * retry_policy.backoff_multiplier) as u64;
                delay_ms = delay_ms.min(retry_policy.max_delay_ms);
            }
        }
    }
}


async fn execute_flow_inner(state: &Arc<AppState>, flow_id: &str, payload: Value) -> Result<Value> {
    tracing::info!("📨 Executing flow: {}", flow_id);

    FLOW_EXECUTIONS_TOTAL.inc();
    let start = Instant::now();

    let flow = {
        let flows = state.flows.read().await;
        flows.get(flow_id).cloned()
    };

    let flow = flow.ok_or_else(|| anyhow::anyhow!("Flow not found: {}", flow_id))?;

    // Connect flow connectors dynamically before execution
    connect_flow_connectors(state, &flow).await
        .map_err(|e| anyhow::anyhow!("Connector setup failed: {}", e))?;

    let cb_policy = flow.circuit_breaker.clone();
    let retry_policy = flow.retry.clone();

    let input = Message::new(payload);

    // Execute with retry if policy exists
    let result = if let Some(ref policy) = retry_policy {
        let executor = state.executor.clone();
        let flow_clone = flow.clone();
        let input_clone = input.clone();
        execute_with_retry(policy, flow_id, move || {
            let executor = executor.clone();
            let flow = flow_clone.clone();
            let input = input_clone.clone();
            async move {
                let executor = executor.read().await;
                executor.execute_flow(&flow, input).await
            }
        }).await
    } else {
        let executor = state.executor.read().await;
        executor.execute_flow(&flow, input).await
    };

    let duration = start.elapsed().as_secs_f64();
    FLOW_EXECUTION_DURATION.observe(duration);

    match result {
        Ok(output) => {
            FLOW_EXECUTIONS_SUCCESS.inc();

            if let Some(policy) = cb_policy {
                update_circuit_breaker_on_success(state.clone(), flow_id.to_string(), policy).await;
            }

            tracing::info!("✅ Flow {} completed in {:.3}s", flow_id, duration);

            Ok(json!({
                "flow_id": flow_id,
                "flow_name": flow.name,
                "status": "completed",
                "result": output.payload,
                "timestamp": output.timestamp,
                "duration_seconds": duration,
                "node_id": state.node_id
            }))
        }
        Err(e) => {
            FLOW_EXECUTIONS_FAILED.inc();

            if let Some(policy) = cb_policy {
                update_circuit_breaker_on_failure(state.clone(), flow_id.to_string(), policy).await;
            }

            tracing::error!("❌ Flow {} failed after {:.3}s: {}", flow_id, duration, e);
            Err(e.into())
        }
    }
}

async fn execute_flow(
    State(state): State<Arc<AppState>>,
    Path(flow_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    execute_flow_inner(&state, &flow_id, payload)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Match a parameterized pattern like `/users/:userId` against an actual path like `users/1`.
/// Returns `Some(HashMap)` of extracted params on match, `None` otherwise.
fn match_path_pattern(pattern: &str, actual: &str) -> Option<HashMap<String, String>> {
    let pattern = pattern.trim_start_matches('/');
    let actual = actual.trim_start_matches('/');
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let actual_parts: Vec<&str> = actual.split('/').collect();
    if pattern_parts.len() != actual_parts.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (pp, ap) in pattern_parts.iter().zip(actual_parts.iter()) {
        if let Some(param_name) = pp.strip_prefix(':') {
            params.insert(param_name.to_string(), ap.to_string());
        } else if pp != ap {
            return None;
        }
    }
    Some(params)
}

async fn trigger_flow(
    State(state): State<Arc<AppState>>,
    method: axum::http::Method,
    Path(path): Path<String>,
    Query(query_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Result<Json<Value>, AppError> {
    let method_str = method.as_str();
    tracing::info!("🎯 HTTP Trigger: {} /{}", method_str, path);

    // Find flow matching both path AND method (supports :param patterns)
    let (flow, path_params) = {
        let flows = state.flows.read().await;
        let mut matched = None;
        for f in flows.values() {
            if let common::Trigger::Http { path: trigger_path, method: trigger_method } = &f.trigger {
                if trigger_method.to_uppercase() != method_str.to_uppercase() {
                    continue;
                }
                if let Some(params) = match_path_pattern(trigger_path, &path) {
                    matched = Some((f.clone(), params));
                    break;
                }
            }
        }
        match matched {
            Some(pair) => pair,
            None => return Err(AppError::NotFound(format!(
                "No flow registered for {} /{}",
                method_str, path
            ))),
        }
    };

    let flow_id = flow.id.clone();
    
    // Connect flow connectors dynamically before execution
    connect_flow_connectors(&state, &flow).await
        .map_err(|e| AppError::Internal(format!("Connector setup failed: {}", e)))?;
    
    let cb_policy = flow.circuit_breaker.clone();
    let retry_policy = flow.retry.clone();
    
    // Build input message structured as {{ trigger.query_params.X }}, etc.
    let query_params_obj: serde_json::Map<String, Value> = query_params
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();

    let headers_obj: serde_json::Map<String, Value> = headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str().ok().map(|s| (k.as_str().to_lowercase(), Value::String(s.to_string())))
        })
        .collect();

    let body_data = body.map(|Json(b)| b).unwrap_or(Value::Null);

    let path_params_obj: serde_json::Map<String, Value> = path_params
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();

    let payload = json!({
        "trigger": {
            "type": "http",
            "path": format!("/{}", path),
            "method": method_str,
            "query_params": query_params_obj,
            "path_params": path_params_obj,
            "headers": headers_obj,
            "body": body_data
        }
    });

    let input = Message::new(payload);
    
    FLOW_EXECUTIONS_TOTAL.inc();
    let start = Instant::now();
    
    // Execute with retry if policy exists
    let result = if let Some(ref policy) = retry_policy {
        let executor = state.executor.clone();
        let flow_clone = flow.clone();
        let input_clone = input.clone();
        
        execute_with_retry(policy, &flow_id, move || {
            let executor = executor.clone();
            let flow = flow_clone.clone();
            let input = input_clone.clone();
            
            async move {
                let executor = executor.read().await;
                executor.execute_flow(&flow, input).await
            }
        }).await
    } else {
        let executor = state.executor.read().await;
        executor.execute_flow(&flow, input).await
    };
    
    let duration = start.elapsed().as_secs_f64();
    FLOW_EXECUTION_DURATION.observe(duration);
    
    match result {
        Ok(output) => {
            FLOW_EXECUTIONS_SUCCESS.inc();
            
            // Update circuit breaker on success
            if let Some(policy) = cb_policy {
                update_circuit_breaker_on_success(state.clone(), flow_id.clone(), policy).await;
            }
            
            tracing::info!("✅ Trigger flow {} completed in {:.3}s", flow_id, duration);
            Ok(Json(output.payload))
        }
        Err(e) => {
            FLOW_EXECUTIONS_FAILED.inc();
            
            // Update circuit breaker on failure
            if let Some(policy) = cb_policy {
                update_circuit_breaker_on_failure(state.clone(), flow_id.clone(), policy).await;
            }
            
            tracing::error!("❌ Trigger flow {} failed after {:.3}s: {}", flow_id, duration, e);
            Err(AppError::Internal(e.to_string()))
        }
    }
}

async fn load_scheduled_flows(state: &Arc<AppState>) -> Result<()> {
    // Load flows from database
    let flows: Vec<FlowDefinition> = state.flows.read().await.values().cloned().collect();

    for flow in flows {
        if let common::Trigger::Schedule { cron } = &flow.trigger {
            let state_clone = state.clone();
            let executor = move |flow_id: String, context: Value| {
                let state = state_clone.clone();
                tokio::spawn(async move {
                    execute_flow_inner(&state, &flow_id, context).await
                })
            };

            state.scheduler.schedule_flow(
                flow.id.clone(),
                flow.name.clone(),
                cron,
                executor,
            ).await?;
        }
    }
    
    Ok(())
}

// Helper function for backward compatibility
fn _create_default_flow(path: &str) -> FlowDefinition {
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
                    "sql": "SELECT * FROM users LIMIT 1"
                }),
            },
            FlowStep::Log {
                name: "response".to_string(),
                message: "Returning data to client".to_string(),
            },
        ],
        rate_limit: Some(RateLimitPolicy {
            max_requests: 100,
            window_seconds: 60,
            key_type: RateLimitKeyType::PerIp,
            message: None,
        }),
        circuit_breaker: None,
        retry: None,
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


// ─── Connector Instance Listener ─────────────────────────────────────────────

async fn listen_for_connector_instances(state: Arc<AppState>) -> Result<()> {
    tracing::info!("🔌 Listening for connector instance events...");
    
    let mut created = state.nats.subscribe("connector.instance.created").await?;
    let mut updated = state.nats.subscribe("connector.instance.updated").await?;
    let mut deleted = state.nats.subscribe("connector.instance.deleted").await?;
    
    loop {
        tokio::select! {
            Some(msg) = created.next() => {
                if let Ok(event) = serde_json::from_slice::<common::ConnectorInstanceEvent>(&msg.payload) {
                    if let common::ConnectorInstanceEvent::Created { instance } = event {
                        tracing::info!("📥 Connector instance created: {}", instance.id);
                        state.connector_registry.register(instance).await;
                    }
                }
            }
            Some(msg) = updated.next() => {
                if let Ok(event) = serde_json::from_slice::<common::ConnectorInstanceEvent>(&msg.payload) {
                    if let common::ConnectorInstanceEvent::Updated { instance } = event {
                        tracing::info!("📥 Connector instance updated: {}", instance.id);
                        state.connector_registry.register(instance).await;
                    }
                }
            }
            Some(msg) = deleted.next() => {
                if let Ok(event) = serde_json::from_slice::<common::ConnectorInstanceEvent>(&msg.payload) {
                    if let common::ConnectorInstanceEvent::Deleted { id } = event {
                        tracing::info!("📥 Connector instance deleted: {}", id);
                        state.connector_registry.unregister(&id).await;
                    }
                }
            }
        }
    }
}

// ─── Dynamic Connector Connection Helper ─────────────────────────────────────

/// Extract unique connector IDs from flow steps and connect them
async fn connect_flow_connectors(
    state: &AppState,
    flow: &FlowDefinition,
) -> Result<()> {
    use common::FlowStep;
    
    let mut connector_ids = std::collections::HashSet::new();
    
    for step in &flow.steps {
        if let FlowStep::Call { connector, .. } = step {
            connector_ids.insert(connector.clone());
        }
    }
    
    if connector_ids.is_empty() {
        return Ok(());
    }
    
    // Get a mutable executor to register connectors
    let mut executor = state.executor.write().await;
    
    for connector_id in connector_ids {
        // Skip 'http' - already registered at startup
        if connector_id == "http" {
            continue;
        }
        
        // Dynamically connect this connector
        state.connector_registry
            .connect_for_flow(&connector_id, &mut *executor)
            .await
            .map_err(|e| {
                tracing::error!("Failed to connect {}: {}", connector_id, e);
                e
            })?;
    }
    
    Ok(())
}
