use axum::{
    extract::{State, Path, Json, Query, Extension},
};
use std::sync::Arc;
use serde::Deserialize;
use serde_json::{json, Value};
use common::{User, Permission};

use crate::error::AppError;
use crate::AppState;

/// GET /audit-logs
pub async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<Value>, AppError> {
    if !user.can(&Permission::ReadAuditLogs) {
        return Err(AppError::Unauthorized("Insufficient permissions to view audit logs".to_string()));
    }

    // Developers may only filter by entity or user, not retrieve all logs
    if !user.is_admin() && params.entity_id.is_none() && params.user_id.is_none() {
        return Err(AppError::Unauthorized("Developers must filter by entity_id or user_id".to_string()));
    }

    let logs = if let Some(entity_id) = params.entity_id {
        state.audit_logger.get_logs_for_entity(
            &params.entity_type.unwrap_or("flow".to_string()),
            &entity_id,
            params.limit.unwrap_or(100),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?
    } else if let Some(user_id) = params.user_id {
        state.audit_logger.get_logs_for_user(
            &user_id,
            params.limit.unwrap_or(100),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?
    } else {
        state.audit_logger.get_recent_logs(
            params.limit.unwrap_or(100),
        ).await.map_err(|e| AppError::Internal(e.to_string()))?
    };

    let count = logs.len();
    Ok(Json(json!({
        "logs": logs,
        "count": count
    })))
}

/// GET /flows/:id/audit-logs
pub async fn get_flow_audit_logs(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.can(&Permission::ReadAuditLogs) {
        return Err(AppError::Unauthorized("Insufficient permissions to view audit logs".to_string()));
    }

    let logs = state.audit_logger.get_logs_for_entity(
        "flow",
        &id,
        100,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    let count = logs.len();
    Ok(Json(json!({
        "flow_id": id,
        "logs": logs,
        "count": count
    })))
}

/// GET /connector-instances/:id/audit-logs
pub async fn get_connector_audit_logs(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.can(&Permission::ReadAuditLogs) {
        return Err(AppError::Unauthorized("Insufficient permissions to view audit logs".to_string()));
    }

    let logs = state.audit_logger.get_logs_for_entity(
        "connector_instance",
        &id,
        100,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    let count = logs.len();
    Ok(Json(json!({
        "connector_id": id,
        "logs": logs,
        "count": count
    })))
}

#[derive(Deserialize)]
pub struct AuditQueryParams {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub user_id: Option<String>,
    pub limit: Option<i64>,
}