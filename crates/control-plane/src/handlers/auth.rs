use axum::extract::{Path, State};
use axum::Json;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use common::JwtClaims;

use crate::error::AppError;
use crate::state::AppState;

const TOKEN_EXPIRY_SECS: i64 = 3600;

#[derive(Deserialize)]
pub struct CreateClientBody {
    pub name: String,
    pub expires_in_days: Option<i64>,
}

#[derive(Deserialize)]
pub struct ToggleBody {
    pub active: bool,
}

pub async fn create_client(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateClientBody>,
) -> Result<Json<Value>, AppError> {
    let client_id  = format!("cid_{}", uuid::Uuid::new_v4().simple());
    let raw_secret = format!("cs_{}", uuid::Uuid::new_v4().simple());
    let hash = bcrypt::hash(&raw_secret, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let expires_at: Option<chrono::DateTime<chrono::Utc>> =
        body.expires_in_days.map(|d| chrono::Utc::now() + chrono::Duration::days(d));

    sqlx::query!(
        "INSERT INTO client_credentials (client_id, client_secret_hash, name, active, expires_at)
         VALUES ($1, $2, $3, TRUE, $4)",
        client_id, hash, body.name, expires_at
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("🔑 Created client: {} ({})", body.name, client_id);

    Ok(Json(json!({
        "client_id":     client_id,
        "client_secret": raw_secret,
        "name":          body.name,
        "active":        true,
        "expires_at":    expires_at,
        "warning":       "Store client_secret now — it will not be shown again"
    })))
}

pub async fn list_clients(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query!(
        "SELECT client_id, name, active, created_at, expires_at
         FROM client_credentials ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let clients: Vec<Value> = rows.iter().map(|r| json!({
        "client_id":  r.client_id,
        "name":       r.name,
        "active":     r.active,
        "created_at": r.created_at,
        "expires_at": r.expires_at,
    })).collect();

    Ok(Json(json!({ "clients": clients, "count": clients.len() })))
}

pub async fn get_client(
    State(state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query!(
        "SELECT client_id, name, active, created_at, expires_at
         FROM client_credentials WHERE client_id = $1", client_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Client not found: {}", client_id)))?;

    Ok(Json(json!({
        "client_id":  row.client_id,
        "name":       row.name,
        "active":     row.active,
        "created_at": row.created_at,
        "expires_at": row.expires_at,
    })))
}

pub async fn delete_client(
    State(state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let res = sqlx::query!(
        "DELETE FROM client_credentials WHERE client_id = $1", client_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Client not found: {}", client_id)));
    }
    tracing::info!("🗑️  Deleted client: {}", client_id);
    Ok(Json(json!({ "deleted": client_id })))
}

pub async fn toggle_client(
    State(state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
    Json(body): Json<ToggleBody>,
) -> Result<Json<Value>, AppError> {
    let res = sqlx::query!(
        "UPDATE client_credentials SET active = $1 WHERE client_id = $2",
        body.active, client_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Client not found: {}", client_id)));
    }
    let status = if body.active { "activated" } else { "deactivated" };
    tracing::info!("🔄 Client {} {}", client_id, status);
    Ok(Json(json!({ "client_id": client_id, "active": body.active })))
}

pub async fn issue_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<common::TokenRequest>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query!(
        "SELECT client_id, client_secret_hash, name, active, expires_at
         FROM client_credentials WHERE client_id = $1", body.client_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .ok_or_else(|| AppError::Unauthorized("Invalid credentials".into()))?;

    if !row.active {
        return Err(AppError::Unauthorized("Client is deactivated".into()));
    }

    if let Some(exp) = row.expires_at {
        if exp < chrono::Utc::now() {
            return Err(AppError::Unauthorized("Credential has expired".into()));
        }
    }

    let ok = bcrypt::verify(&body.client_secret, &row.client_secret_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !ok {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    let now = chrono::Utc::now().timestamp();
    let claims = JwtClaims {
        sub:         row.client_id.clone(),
        client_name: row.name.clone(),
        iat:         now,
        exp:         now + TOKEN_EXPIRY_SECS,
        jti:         uuid::Uuid::new_v4().to_string(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ).map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("🎫 Token issued for: {} ({})", row.name, row.client_id);

    Ok(Json(json!({
        "access_token": token,
        "token_type":   "Bearer",
        "expires_in":   TOKEN_EXPIRY_SECS,
        "client_id":    row.client_id,
    })))
}
