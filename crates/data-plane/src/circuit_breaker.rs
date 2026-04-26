use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use common::{CircuitBreakerPolicy, CircuitState};

use crate::metrics::{
    CIRCUIT_BREAKER_CLOSES_TOTAL, CIRCUIT_BREAKER_HALF_OPENS_TOTAL,
    CIRCUIT_BREAKER_OPENS_TOTAL, CIRCUIT_BREAKER_REJECTED_TOTAL, CIRCUIT_BREAKER_STATE,
};
use crate::state::{AppState, CircuitBreakerState};

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn circuit_breaker_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();

    let flow_id = if path.starts_with("/flows/") && path.ends_with("/execute") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 { Some(parts[2].to_string()) } else { None }
    } else if path.starts_with("/api/trigger/") {
        let trigger_path = path.strip_prefix("/api/trigger/").unwrap_or("");
        let flows = state.flows.read().await;
        flows.values()
            .find(|f| {
                if let common::Trigger::Http { path: p, method } = &f.trigger {
                    method == "GET" && p.contains(trigger_path)
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
            flows.get(flow_id_str).and_then(|f| f.circuit_breaker.clone())
        };

        if let Some(cb_policy) = cb_policy {
            let mut circuit_breakers = state.circuit_breakers.write().await;
            let cb_state = circuit_breakers
                .entry(flow_id_str.clone())
                .or_insert_with(CircuitBreakerState::new);

            let now = current_timestamp();

            if let CircuitState::Open = cb_state.state {
                if now - cb_state.opened_at >= cb_policy.timeout_seconds {
                    cb_state.state = CircuitState::HalfOpen;
                    cb_state.success_count = 0;
                    CIRCUIT_BREAKER_HALF_OPENS_TOTAL.inc();
                    CIRCUIT_BREAKER_STATE.with_label_values(&[flow_id_str]).set(2);
                    tracing::info!("🔄 Circuit breaker HALF-OPEN for flow: {}", flow_id_str);
                } else {
                    CIRCUIT_BREAKER_REJECTED_TOTAL.inc();
                    tracing::warn!("🔌 Circuit breaker OPEN - rejecting request for flow: {}", flow_id_str);
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(json!({
                            "error": "Circuit breaker is open - service temporarily unavailable",
                            "flow_id": flow_id_str,
                            "state": "open",
                            "retry_after_seconds": cb_policy.timeout_seconds - (now - cb_state.opened_at)
                        })),
                    ).into_response();
                }
            }
        }
    }

    next.run(request).await
}

pub async fn update_circuit_breaker_on_success(
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
                    CIRCUIT_BREAKER_STATE.with_label_values(&[&flow_id]).set(0);
                    tracing::info!("✅ Circuit breaker CLOSED for flow: {}", flow_id);
                }
            }
            CircuitState::Closed => {
                cb_state.failure_count = 0;
            }
            _ => {}
        }
    });
}

pub async fn update_circuit_breaker_on_failure(
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
                    CIRCUIT_BREAKER_STATE.with_label_values(&[&flow_id]).set(1);
                    tracing::error!("🔴 Circuit breaker OPEN for flow: {} (failures: {})", flow_id, cb_state.failure_count);
                }
            }
            CircuitState::HalfOpen => {
                cb_state.state = CircuitState::Open;
                cb_state.opened_at = now;
                cb_state.success_count = 0;
                CIRCUIT_BREAKER_OPENS_TOTAL.inc();
                CIRCUIT_BREAKER_STATE.with_label_values(&[&flow_id]).set(1);
                tracing::error!("🔴 Circuit breaker re-OPEN for flow: {} (failed in half-open)", flow_id);
            }
            _ => {}
        }
    });
}

pub async fn circuit_breaker_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let circuit_breakers = state.circuit_breakers.read().await;
    let flows = state.flows.read().await;

    let status: Vec<serde_json::Value> = circuit_breakers.iter().map(|(flow_id, cb_state)| {
        let policy = flows.get(flow_id).and_then(|f| f.circuit_breaker.as_ref());
        json!({
            "flow_id":       flow_id,
            "state": match cb_state.state {
                CircuitState::Closed   => "closed",
                CircuitState::Open     => "open",
                CircuitState::HalfOpen => "half_open",
            },
            "failure_count": cb_state.failure_count,
            "success_count": cb_state.success_count,
            "policy":        policy
        })
    }).collect();

    Json(json!({
        "circuit_breakers": status,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
