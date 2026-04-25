use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;

pub async fn list_connectors(State(state): State<Arc<AppState>>) -> Json<Value> {
    let connectors = state.connectors.read().await;
    Json(json!({"connectors": *connectors, "count": connectors.len()}))
}

pub async fn get_connector(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let connectors = state.connectors.read().await;
    let connector = connectors.iter().find(|c| c.id == id)
        .ok_or_else(|| AppError::NotFound("Connector not found".to_string()))?;
    Ok(Json(json!(connector)))
}

pub async fn list_triggers(State(state): State<Arc<AppState>>) -> Json<Value> {
    let triggers = state.triggers.read().await;
    Json(json!({"triggers": *triggers, "count": triggers.len()}))
}

pub async fn get_trigger(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let triggers = state.triggers.read().await;
    let trigger = triggers.iter().find(|t| t.id == id)
        .ok_or_else(|| AppError::NotFound("Trigger not found".to_string()))?;
    Ok(Json(json!(trigger)))
}
