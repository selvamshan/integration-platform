use std::sync::Arc;
use axum::extract::{State, Path, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::error::AppError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

pub async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT p.id::text, p.name, p.description, COUNT(f.id) AS flow_count
         FROM projects p
         LEFT JOIN flow_definitions f ON f.project_id = p.id
         GROUP BY p.id, p.name, p.description
         ORDER BY p.created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let projects: Vec<Value> = rows.iter().map(|r| {
        json!({
            "id":          r.try_get::<String, _>("id").unwrap_or_default(),
            "name":        r.try_get::<String, _>("name").unwrap_or_default(),
            "description": r.try_get::<Option<String>, _>("description").ok().flatten(),
            "flow_count":  r.try_get::<i64, _>("flow_count").unwrap_or(0),
        })
    }).collect();

    Ok(Json(json!({ "projects": projects, "count": projects.len() })))
}

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<Json<Value>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("Project name is required".to_string()));
    }

    let id = Uuid::new_v4();

    sqlx::query("INSERT INTO projects (id, name, description) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    tracing::info!("✅ Project created: {} ({})", req.name, id);

    Ok(Json(json!({
        "id":          id.to_string(),
        "name":        req.name,
        "description": req.description,
        "flow_count":  0,
    })))
}

pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pid = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid project ID".to_string()))?;

    let row = sqlx::query(
        "SELECT p.id::text, p.name, p.description, COUNT(f.id) AS flow_count
         FROM projects p
         LEFT JOIN flow_definitions f ON f.project_id = p.id
         WHERE p.id = $1
         GROUP BY p.id, p.name, p.description"
    )
    .bind(pid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    Ok(Json(json!({
        "id":          row.try_get::<String, _>("id").unwrap_or_default(),
        "name":        row.try_get::<String, _>("name").unwrap_or_default(),
        "description": row.try_get::<Option<String>, _>("description").ok().flatten(),
        "flow_count":  row.try_get::<i64, _>("flow_count").unwrap_or(0),
    })))
}

pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pid = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid project ID".to_string()))?;

    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(pid)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    tracing::info!("🗑️  Project deleted: {}", id);

    Ok(Json(json!({ "deleted": true, "id": id })))
}

pub async fn list_project_flows(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pid = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid project ID".to_string()))?;

    let exists = sqlx::query("SELECT 1 FROM projects WHERE id = $1")
        .bind(pid)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    if exists.is_none() {
        return Err(AppError::NotFound("Project not found".to_string()));
    }

    let flows = state.flows.read().await;
    let project_flows: Vec<_> = flows.iter()
        .filter(|f| f.project_id.as_deref() == Some(id.as_str()))
        .collect();

    Ok(Json(json!({ "flows": project_flows, "count": project_flows.len() })))
}
