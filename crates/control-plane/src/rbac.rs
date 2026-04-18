//! RBAC middleware — validates OIDC JWT (Keycloak / Auth0 / Okta) and checks permissions.

use std::sync::Arc;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use common::{User, Permission};
use crate::oidc::OidcAuth;

/// Validates the Bearer token with the configured OIDC provider and injects a `User`
/// extension into the request for downstream handlers.
pub async fn rbac_middleware(
    State(oidc): State<Arc<OidcAuth>>,
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    tracing::info!("RBAC middleware — provider: {}", oidc.provider_name());

    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Missing or invalid Authorization header",
                    "hint":  "Use: Authorization: Bearer <token>"
                })),
            ).into_response();
        }
    };

    let user = match oidc.validate_token(token).await {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("Token validation failed ({}): {}", oidc.provider_name(), e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error":    "Invalid or expired token",
                    "provider": oidc.provider_name(),
                    "details":  e.to_string()
                })),
            ).into_response();
        }
    };

    tracing::info!("🔓 Authenticated: {} (roles: {:?})", user.username, user.roles);
    request.extensions_mut().insert(user);
    next.run(request).await
}

/// Maps HTTP method + path to a required `Permission`.
/// Returns `None` for public endpoints.
pub fn extract_permission(path: &str, method: &str) -> Option<Permission> {
    match (method, path) {
        // Flows
        ("GET",    p) if p.starts_with("/flows")           => Some(Permission::ReadFlows),
        ("POST",   "/flows")                               => Some(Permission::WriteFlows),
        ("PUT",    p) if p.starts_with("/flows/")          => Some(Permission::WriteFlows),
        ("DELETE", p) if p.starts_with("/flows/")          => Some(Permission::DeleteFlows),

        // Connectors
        ("GET",    p) if p.starts_with("/connector-instances") => Some(Permission::ReadConnectors),
        ("POST",   "/connector-instances")                 => Some(Permission::WriteConnectors),
        ("PUT",    p) if p.starts_with("/connector-instances/") => Some(Permission::WriteConnectors),
        ("DELETE", p) if p.starts_with("/connector-instances/") => Some(Permission::DeleteConnectors),

        // APIs
        ("GET",    p) if p.starts_with("/apis")            => Some(Permission::ReadApis),
        ("POST",   "/apis")                                => Some(Permission::WriteApis),
        ("DELETE", p) if p.starts_with("/apis/")           => Some(Permission::DeleteApis),

        // Auth clients
        ("GET",    p) if p.starts_with("/auth/clients")    => Some(Permission::ReadClients),
        ("POST",   "/auth/clients")                        => Some(Permission::WriteClients),
        ("DELETE", p) if p.starts_with("/auth/clients/")   => Some(Permission::DeleteClients),
        ("PATCH",  p) if p.starts_with("/auth/clients/")   => Some(Permission::WriteClients),

        // Monitoring
        ("GET", "/metrics")                                => Some(Permission::ReadMetrics),
        ("GET", "/rate-limit-stats")                       => Some(Permission::ReadRateLimits),

        // User management
        ("GET",    "/users")                               => Some(Permission::ManageUsers),
        ("POST",   "/users/invite")                        => Some(Permission::InviteUsers),
        ("DELETE", p) if p.starts_with("/users/")          => Some(Permission::ManageUsers),

        // Audit logs
        ("GET", "/audit-logs")                             => Some(Permission::ReadAuditLogs),
        ("GET", p) if p.ends_with("/audit-logs")           => Some(Permission::ReadAuditLogs),

        // Public
        ("GET",  "/health")      => None,
        ("POST", "/auth/token")  => None,
        ("GET",  "/users/me")    => None,

        _ => None,
    }
}

/// Permission check middleware — must run **after** `rbac_middleware`.
pub async fn permission_middleware(request: Request<Body>, next: Next) -> Response {
    let path   = request.uri().path().to_string();
    let method = request.method().as_str().to_string();
    tracing::info!("Permission check: {} {}", method, path);

    let user = match request.extensions().get::<User>() {
        Some(u) => u.clone(),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "No user context — rbac_middleware must run first"})),
            ).into_response();
        }
    };

    let required = match extract_permission(&path, &method) {
        Some(p) => p,
        None    => return next.run(request).await,   // public endpoint
    };

    if !user.can(&required) {
        tracing::warn!(
            "🔒 Access denied: {} → {} {} (requires {:?})",
            user.username, method, path, required
        );
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error":      "Insufficient permissions",
                "required":   format!("{:?}", required),
                "your_roles": user.roles.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
                "hint":       "Contact an admin to request access"
            })),
        ).into_response();
    }

    tracing::debug!("✅ {} granted {} {}", user.username, method, path);
    next.run(request).await
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let val = headers.get("authorization")?.to_str().ok()?;
    val.strip_prefix("Bearer ").or_else(|| val.strip_prefix("bearer "))
}

pub fn get_current_user(request: &Request<Body>) -> Option<&User> {
    request.extensions().get::<User>()
}
