use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use common::{ApiDefinition, ConfigUpdate, Endpoint};

use crate::error::AppError;
use crate::state::AppState;
use super::flow::publish_event;

#[derive(Deserialize)]
pub struct CreateApiRequest {
    pub name: String,
    pub version: String,
    pub base_path: String,
    pub endpoints: Vec<EndpointRequest>,
}

#[derive(Deserialize)]
pub struct EndpointRequest {
    pub path: String,
    pub method: String,
    pub flow_id: String,
}

pub async fn list_apis(State(state): State<Arc<AppState>>) -> Json<Value> {
    let apis = state.apis.read().await;
    Json(json!({"apis": *apis, "count": apis.len()}))
}

pub async fn create_api(
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

    sqlx::query("INSERT INTO api_definitions (id, name, version, base_path, config) VALUES ($1, $2, $3, $4, $5)")
        .bind(uuid::Uuid::parse_str(&api.id).unwrap())
        .bind(&api.name)
        .bind(&api.version)
        .bind(&api.base_path)
        .bind(serde_json::to_value(&api).unwrap())
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let mut apis = state.apis.write().await;
    apis.push(api.clone());
    drop(apis);

    let event = ConfigUpdate::ApiCreated { api: api.clone() };
    publish_event(&state.nats, &event).await?;

    tracing::info!("✅ API created: {}", api_id);
    Ok(Json(json!(api)))
}

pub async fn get_api(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let apis = state.apis.read().await;
    let api = apis.iter().find(|a| a.id == id)
        .ok_or_else(|| AppError::NotFound("API not found".to_string()))?;
    Ok(Json(json!(api)))
}
