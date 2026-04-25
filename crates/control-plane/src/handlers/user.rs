use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct InviteUserBody {
    pub email: String,
    pub role: String,
}

pub async fn invite_user(
    State(state): State<Arc<AppState>>,
    current_user: Option<Extension<common::User>>,
    Json(body): Json<InviteUserBody>,
) -> Result<Json<Value>, AppError> {
    let current_user = current_user
        .ok_or_else(|| AppError::Unauthorized("Authentication required (enable RBAC)".into()))?
        .0;

    if !current_user.is_admin() {
        return Err(AppError::Unauthorized("Admin role required".into()));
    }

    let role = common::UserRole::from_str(&body.role)
        .ok_or_else(|| AppError::Internal(format!("Invalid role: {}", body.role)))?;

    let user_id = state.oidc.invite_user(&body.email, &role).await
        .map_err(|e| AppError::Internal(format!("Keycloak invitation failed: {}", e)))?;

    let invitation_id    = uuid::Uuid::new_v4().to_string();
    let invitation_token = uuid::Uuid::new_v4().to_string();
    let expires_at       = chrono::Utc::now() + chrono::Duration::days(7);

    sqlx::query!(
        "INSERT INTO user_invitations (id, email, role, invited_by, invited_at, expires_at, token, accepted)
         VALUES ($1, $2, $3, $4, NOW(), $5, $6, FALSE)",
        invitation_id, body.email, role.as_str(), current_user.id, expires_at, invitation_token
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("✅ User invited: {} as {} by {}", body.email, role.as_str(), current_user.username);

    Ok(Json(json!({
        "invitation_id":    invitation_id,
        "email":            body.email,
        "role":             role.as_str(),
        "keycloak_user_id": user_id,
        "expires_at":       expires_at,
        "status":           "invitation_sent"
    })))
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let users = state.oidc.list_users().await
        .map_err(|e| AppError::Internal(format!("Failed to list users: {}", e)))?;
    Ok(Json(json!({ "users": users, "count": users.len() })))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    state.oidc.delete_user(&user_id).await
        .map_err(|e| AppError::Internal(format!("Failed to delete user: {}", e)))?;
    tracing::info!("🗑️  User deleted: {}", user_id);
    Ok(Json(json!({ "deleted": user_id, "status": "success" })))
}

pub async fn get_current_user(
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Value>, AppError> {
    tracing::info!("Get current user");
    let user = request.extensions().get::<common::User>()
        .ok_or_else(|| AppError::Internal("No user context".into()))?;
    Ok(Json(json!({
        "id":       user.id,
        "username": user.username,
        "email":    user.email,
        "name":     user.name,
        "roles":    user.roles.iter().map(|r| r.as_str()).collect::<Vec<_>>()
    })))
}
