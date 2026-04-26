use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};

use common::{FlowDefinition, Message};

use crate::circuit_breaker::{update_circuit_breaker_on_failure, update_circuit_breaker_on_success};
use crate::error::AppError;
use crate::executor::{connect_flow_connectors, execute_flow_inner, execute_with_retry};
use crate::metrics::{
    FLOW_EXECUTION_DURATION, FLOW_EXECUTIONS_FAILED, FLOW_EXECUTIONS_SUCCESS,
    FLOW_EXECUTIONS_TOTAL,
};
use crate::state::AppState;

pub async fn root() -> &'static str {
    "Data Plane - Integration Platform with Event Subscription"
}

pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status":    "healthy",
        "service":   "data-plane",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn list_flows(State(state): State<Arc<AppState>>) -> Json<Value> {
    let flows = state.flows.read().await;
    let flow_list: Vec<&FlowDefinition> = flows.values().collect();
    Json(json!({
        "flows":   flow_list,
        "count":   flows.len(),
        "node_id": state.node_id
    }))
}

pub async fn execute_flow(
    State(state): State<Arc<AppState>>,
    Path(flow_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    execute_flow_inner(&state, &flow_id, payload)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Match a parameterized pattern like `/users/:userId` against an actual path.
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
        if let Some(name) = pp.strip_prefix(':') {
            params.insert(name.to_string(), ap.to_string());
        } else if pp != ap {
            return None;
        }
    }
    Some(params)
}

pub async fn trigger_flow(
    State(state): State<Arc<AppState>>,
    method: axum::http::Method,
    Path(path): Path<String>,
    Query(query_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Result<Json<Value>, AppError> {
    let method_str = method.as_str();
    tracing::info!("🎯 HTTP Trigger: {} /{}", method_str, path);

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
                "No flow registered for {} /{}", method_str, path
            ))),
        }
    };

    let flow_id = flow.id.clone();

    connect_flow_connectors(&state, &flow).await
        .map_err(|e| AppError::Internal(format!("Connector setup failed: {}", e)))?;

    let cb_policy   = flow.circuit_breaker.clone();
    let retry_policy = flow.retry.clone();

    let query_params_obj: serde_json::Map<String, Value> = query_params
        .into_iter().map(|(k, v)| (k, Value::String(v))).collect();

    let headers_obj: serde_json::Map<String, Value> = headers
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.as_str().to_lowercase(), Value::String(s.to_string()))))
        .collect();

    let body_data = body.map(|Json(b)| b).unwrap_or(Value::Null);

    let path_params_obj: serde_json::Map<String, Value> = path_params
        .into_iter().map(|(k, v)| (k, Value::String(v))).collect();

    let payload = json!({
        "trigger": {
            "type":         "http",
            "path":         format!("/{}", path),
            "method":       method_str,
            "query_params": query_params_obj,
            "path_params":  path_params_obj,
            "headers":      headers_obj,
            "body":         body_data
        }
    });

    let input = Message::new(payload);
    FLOW_EXECUTIONS_TOTAL.inc();
    let start = Instant::now();

    let result = if let Some(ref policy) = retry_policy {
        let executor    = state.executor.clone();
        let flow_clone  = flow.clone();
        let input_clone = input.clone();
        execute_with_retry(policy, &flow_id, move || {
            let executor = executor.clone();
            let flow     = flow_clone.clone();
            let input    = input_clone.clone();
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
                update_circuit_breaker_on_success(state.clone(), flow_id.clone(), policy).await;
            }
            tracing::info!("✅ Trigger flow {} completed in {:.3}s", flow_id, duration);
            Ok(Json(output.payload))
        }
        Err(e) => {
            FLOW_EXECUTIONS_FAILED.inc();
            if let Some(policy) = cb_policy {
                update_circuit_breaker_on_failure(state.clone(), flow_id.clone(), policy).await;
            }
            tracing::error!("❌ Trigger flow {} failed after {:.3}s: {}", flow_id, duration, e);
            Err(AppError::Internal(e.to_string()))
        }
    }
}
