//! Authentication middleware for the Data Plane.
//!
//! Two auth methods are accepted, checked in order:
//!
//! **Method 1 – Client-Credentials headers** (direct validation via NATS):
//!   X-Client-Id: cid_<uuid>
//!   X-Client-Secret: cs_<uuid>
//!   → Data-Plane sends a NATS request to `auth.validate.credentials`
//!   → Control-Plane checks the DB, bcrypt-verifies, replies valid/invalid.
//!
//! **Method 2 – Bearer JWT** (local verification, no network hop):
//!   Authorization: Bearer <signed-jwt>
//!   → Data-Plane decodes & verifies the JWT with the shared JWT_SECRET.
//!
//! Protected endpoints:
//!   POST /flows/:flow_id/execute
//!   GET  /api/trigger/:path
//!
//! All other endpoints (health, metrics, /flows list, /circuit-breakers)
//! remain public.

use std::sync::Arc;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
     body::Body,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde_json::json;
use common::{AuthMethod, AuthPrincipal, JwtClaims};

// ─── Shared config injected into the middleware ───────────────────────────────

pub struct AuthConfig {
    pub jwt_secret: String,
    pub nats:       async_nats::Client,
}

// ─── Middleware entry point ───────────────────────────────────────────────────

pub async fn auth_middleware(
    State(cfg): State<Arc<AuthConfig>>,
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();

    // Only guard flow execution and trigger endpoints
    let is_protected = (path.starts_with("/flows/") && path.ends_with("/execute"))
        || path.starts_with("/api/trigger/");

    if !is_protected {
        return next.run(request).await;
    }

    match authenticate(&cfg, &headers).await {
        Ok(principal) => {
            tracing::info!(
                "🔓 Authenticated [{}] client_id={} path={}",
                principal.auth_method, principal.client_id, path
            );
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(reason) => {
            tracing::warn!("🔒 Auth rejected for {}: {}", path, reason);
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error":  reason,
                    "hint":   "Supply X-Client-Id + X-Client-Secret headers, or Authorization: Bearer <jwt>",
                    "docs":   "POST /auth/token to exchange credentials for a JWT"
                })),
            )
                .into_response()
        }
    }
}

// ─── Core auth dispatcher ─────────────────────────────────────────────────────

async fn authenticate(
    cfg: &AuthConfig,
    headers: &HeaderMap,
) -> Result<AuthPrincipal, String> {
    // ── Method 1: X-Client-Id / X-Client-Secret ──────────────────────────────
    let client_id     = header_val(headers, "x-client-id");
    let client_secret = header_val(headers, "x-client-secret");

    if let (Some(id), Some(secret)) = (client_id, client_secret) {
        return validate_client_credentials(cfg, id, secret).await;
    }

    // ── Method 2: Authorization: Bearer <token> ───────────────────────────────
    if let Some(auth) = header_val(headers, "authorization") {
        if let Some(token) = auth.strip_prefix("Bearer ").or_else(|| auth.strip_prefix("bearer ")) {
            return validate_jwt(cfg, token.trim());
        }
    }

    Err("No credentials provided".into())
}

// ─── Method 1: Client-Credentials via NATS request-reply ─────────────────────

async fn validate_client_credentials(
    cfg: &AuthConfig,
    client_id: &str,
    client_secret: &str,
) -> Result<AuthPrincipal, String> {
    let payload = serde_json::to_vec(&json!({
        "client_id":     client_id,
        "client_secret": client_secret,
    }))
    .map_err(|e| e.to_string())?;

    // 3-second timeout — Control Plane is local on the same Docker network
    let reply = tokio::time::timeout(
        tokio::time::Duration::from_secs(3),
        cfg.nats.request("auth.validate.credentials", payload.into()),
    )
    .await
    .map_err(|_| "Auth service timed out".to_string())?
    .map_err(|e| format!("NATS error: {e}"))?;

    let resp: serde_json::Value = serde_json::from_slice(&reply.payload)
        .map_err(|_| "Malformed auth response".to_string())?;

    if resp["valid"].as_bool().unwrap_or(false) {
        Ok(AuthPrincipal {
            client_id:   resp["client_id"].as_str().unwrap_or(client_id).to_string(),
            client_name: resp["name"].as_str().unwrap_or("").to_string(),
            auth_method: AuthMethod::ClientCredentials,
        })
    } else {
        let reason = resp["reason"].as_str().unwrap_or("Invalid credentials").to_string();
        Err(reason)
    }
}

// ─── Method 2: JWT verification (local, no network) ──────────────────────────

fn validate_jwt(cfg: &AuthConfig, token: &str) -> Result<AuthPrincipal, String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &validation,
    )
    .map(|data| AuthPrincipal {
        client_id:   data.claims.sub,
        client_name: data.claims.client_name,
        auth_method: AuthMethod::JwtToken,
    })
    .map_err(|e| format!("Invalid token: {e}"))
}

// ─── Utility ─────────────────────────────────────────────────────────────────

fn header_val<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}
