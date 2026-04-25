use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;

pub async fn get_rate_limit_stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    let stats = state.rate_limit_stats.read().await;
    let mut summary = serde_json::Map::new();

    for (flow_id, events) in stats.iter() {
        let total   = events.len();
        let blocked = events.iter().filter(|e| !e.allowed).count();
        let allowed = events.iter().filter(|e| e.allowed).count();
        summary.insert(flow_id.clone(), json!({
            "total_requests": total,
            "allowed":        allowed,
            "blocked":        blocked,
            "block_rate":     if total > 0 { blocked as f64 / total as f64 * 100.0 } else { 0.0 }
        }));
    }

    Json(json!({
        "flows":     summary,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn get_flow_rate_limit_stats(
    State(state): State<Arc<AppState>>,
    Path(flow_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let stats = state.rate_limit_stats.read().await;
    let events = stats.get(&flow_id)
        .ok_or_else(|| AppError::NotFound(format!("No rate limit stats for flow: {}", flow_id)))?;

    let total   = events.len();
    let blocked = events.iter().filter(|e| !e.allowed).count();
    let allowed = events.iter().filter(|e| e.allowed).count();

    let recent_events: Vec<_> = events.iter().rev().take(10).collect();

    let mut key_stats = std::collections::HashMap::new();
    for event in events {
        let entry = key_stats.entry(event.key.clone()).or_insert((0usize, 0usize));
        if event.allowed { entry.0 += 1; } else { entry.1 += 1; }
    }

    Ok(Json(json!({
        "flow_id": flow_id,
        "summary": {
            "total_requests": total,
            "allowed":        allowed,
            "blocked":        blocked,
            "block_rate":     if total > 0 { blocked as f64 / total as f64 * 100.0 } else { 0.0 }
        },
        "by_key": key_stats.iter().map(|(k, (a, b))| json!({
            "key":     k,
            "allowed": a,
            "blocked": b
        })).collect::<Vec<_>>(),
        "recent_events": recent_events,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
