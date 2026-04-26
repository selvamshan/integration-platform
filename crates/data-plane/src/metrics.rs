use std::time::Instant;

use axum::{body::Body, http::{Request, StatusCode}, middleware::Next, response::{IntoResponse, Response}};
use lazy_static::lazy_static;
use prometheus::{
    Counter, Encoder, Histogram, HistogramOpts, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref HTTP_REQUESTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("http_requests_total", "Total number of HTTP requests")
    ).unwrap();

    pub static ref HTTP_REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration in seconds")
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    ).unwrap();

    pub static ref FLOW_EXECUTIONS_TOTAL: Counter = Counter::with_opts(
        Opts::new("flow_executions_total", "Total number of flow executions")
    ).unwrap();

    pub static ref FLOW_EXECUTIONS_SUCCESS: Counter = Counter::with_opts(
        Opts::new("flow_executions_success_total", "Total number of successful flow executions")
    ).unwrap();

    pub static ref FLOW_EXECUTIONS_FAILED: Counter = Counter::with_opts(
        Opts::new("flow_executions_failed_total", "Total number of failed flow executions")
    ).unwrap();

    pub static ref FLOW_EXECUTION_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new("flow_execution_duration_seconds", "Flow execution duration in seconds")
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0])
    ).unwrap();

    pub static ref RATE_LIMIT_CHECKS_TOTAL: Counter = Counter::with_opts(
        Opts::new("rate_limit_checks_total", "Total number of rate limit checks")
    ).unwrap();

    pub static ref RATE_LIMIT_BLOCKED_TOTAL: Counter = Counter::with_opts(
        Opts::new("rate_limit_blocked_total", "Total number of blocked requests due to rate limiting")
    ).unwrap();

    pub static ref RATE_LIMIT_ALLOWED_TOTAL: Counter = Counter::with_opts(
        Opts::new("rate_limit_allowed_total", "Total number of allowed requests after rate limit check")
    ).unwrap();

    pub static ref FLOWS_LOADED: IntGauge = IntGauge::with_opts(
        Opts::new("flows_loaded", "Number of flows currently loaded")
    ).unwrap();

    pub static ref REDIS_OPERATIONS_TOTAL: Counter = Counter::with_opts(
        Opts::new("redis_operations_total", "Total number of Redis operations")
    ).unwrap();

    pub static ref REDIS_ERRORS_TOTAL: Counter = Counter::with_opts(
        Opts::new("redis_errors_total", "Total number of Redis errors")
    ).unwrap();

    pub static ref CIRCUIT_BREAKER_STATE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("circuit_breaker_state", "Circuit breaker state by flow (0=closed, 1=open, 2=half_open)"),
        &["flow_id"]
    ).unwrap();

    pub static ref CIRCUIT_BREAKER_OPENS_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_opens_total", "Total number of circuit breaker opens")
    ).unwrap();

    pub static ref CIRCUIT_BREAKER_CLOSES_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_closes_total", "Total number of circuit breaker closes")
    ).unwrap();

    pub static ref CIRCUIT_BREAKER_HALF_OPENS_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_half_opens_total", "Total number of circuit breaker half-opens")
    ).unwrap();

    pub static ref CIRCUIT_BREAKER_REJECTED_TOTAL: Counter = Counter::with_opts(
        Opts::new("circuit_breaker_rejected_total", "Total number of requests rejected by circuit breaker")
    ).unwrap();

    pub static ref RETRY_ATTEMPTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("retry_attempts_total", "Total number of retry attempts")
    ).unwrap();

    pub static ref RETRY_SUCCESS_TOTAL: Counter = Counter::with_opts(
        Opts::new("retry_success_total", "Total number of successful retries")
    ).unwrap();

    pub static ref RETRY_EXHAUSTED_TOTAL: Counter = Counter::with_opts(
        Opts::new("retry_exhausted_total", "Total number of exhausted retries")
    ).unwrap();
}

pub fn register_metrics() {
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

pub async fn metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let path = request.uri().path().to_string();

    HTTP_REQUESTS_TOTAL.inc();
    let response = next.run(request).await;

    let duration = start.elapsed().as_secs_f64();
    HTTP_REQUEST_DURATION.observe(duration);
    tracing::debug!("Request to {} took {:.3}s", path, duration);

    response
}

pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics".to_string());
    }

    match String::from_utf8(buffer) {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(e) => {
            tracing::error!("Failed to convert metrics to string: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to convert metrics".to_string())
        }
    }
}
