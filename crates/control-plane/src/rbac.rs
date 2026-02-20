//! RBAC middleware for Control Plane endpoints

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
use crate::keycloak::KeycloakConfig;

/// RBAC middleware - validates Keycloak JWT and checks permissions
pub async fn rbac_middleware(
    State(keycloak): State<Arc<KeycloakConfig>>,
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Extract Bearer token
    tracing::info!("RBAC middleware authentication");
    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Missing or invalid Authorization header",
                    "hint": "Use: Authorization: Bearer <keycloak-token>"
                }))
            ).into_response();
        }
    };

    // Validate token with Keycloak
    let user = match keycloak.validate_token(token).await {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("Token validation failed: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid or expired token",
                    "details": e.to_string()
                }))
            ).into_response();
        }
    };

    tracing::info!("🔓 Authenticated: {} (roles: {:?})", user.username, user.roles);

    // Store user in request extensions
    request.extensions_mut().insert(user);

    next.run(request).await
}

/// Extract required permission from request path and method
pub fn extract_permission(path: &str, method: &str) -> Option<Permission> {
    match (method, path) {
        // Flow permissions
        ("GET", p) if p.starts_with("/flows") => Some(Permission::ReadFlows),
        ("POST", "/flows") => Some(Permission::WriteFlows),
        ("PUT", p) if p.starts_with("/flows/") => Some(Permission::WriteFlows),
        ("DELETE", p) if p.starts_with("/flows/") => Some(Permission::DeleteFlows),
        
        // Connector permissions
        ("GET", p) if p.starts_with("/connector-instances") => Some(Permission::ReadConnectors),
        ("POST", "/connector-instances") => Some(Permission::WriteConnectors),
        ("DELETE", p) if p.starts_with("/connector-instances/") => Some(Permission::DeleteConnectors),
        
        // API permissions
        ("GET", p) if p.starts_with("/apis") => Some(Permission::ReadApis),
        ("POST", "/apis") => Some(Permission::WriteApis),
        ("DELETE", p) if p.starts_with("/apis/") => Some(Permission::DeleteApis),
        
        // Client permissions
        ("GET", p) if p.starts_with("/auth/clients") => Some(Permission::ReadClients),
        ("POST", "/auth/clients") => Some(Permission::WriteClients),
        ("DELETE", p) if p.starts_with("/auth/clients/") => Some(Permission::DeleteClients),
        ("PATCH", p) if p.starts_with("/auth/clients/") => Some(Permission::WriteClients),
        
        // Monitoring permissions
        ("GET", "/metrics") => Some(Permission::ReadMetrics),
        ("GET", "/rate-limit-stats") => Some(Permission::ReadRateLimits),
        
        // User management (admin only)
        ("GET", "/users") => Some(Permission::ManageUsers),
        ("POST", "/users/invite") => Some(Permission::InviteUsers),
        ("DELETE", p) if p.starts_with("/users/") => Some(Permission::ManageUsers),
        
        // Public endpoints (no permission required)
        ("GET", "/health") => None,
        ("POST", "/auth/token") => None,  // Token issuance is public
        
        _ => None,
    }
}

/// Permission check middleware - must be used AFTER rbac_middleware
pub async fn permission_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().as_str().to_string();
    tracing::info!("Permission check middleware {}", &path);
    // Get user from extensions (set by rbac_middleware)
    let user = match request.extensions().get::<User>() {
        Some(u) => u.clone(),
        None => {
            // No user found - should have been set by rbac_middleware
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "No user context found"}))
            ).into_response();
        }
    };

    // Extract required permission
    let required_permission = match extract_permission(&path, &method) {
        Some(p) => p,
        None => {
            // No permission required (public endpoint)
            return next.run(request).await;
        }
    };

    // Check if user has permission
    if !user.can(&required_permission) {
        tracing::warn!(
            "🔒 Access denied: {} tried to access {} {} (requires {:?})",
            user.username, method, path, required_permission
        );

        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Insufficient permissions",
                "required": format!("{:?}", required_permission),
                "your_roles": user.roles.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
                "hint": "Contact admin to request access"
            }))
        ).into_response();
    }

    tracing::debug!("✅ Permission check passed: {} for {}", user.username, path);

    next.run(request).await
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers.get("authorization")?
        .to_str().ok()?
        .strip_prefix("Bearer ")
        .or_else(|| headers.get("authorization")?.to_str().ok()?.strip_prefix("bearer "))
}

/// Helper to get current user from request extensions
pub fn get_current_user(request: &Request<Body>) -> Option<&User> {
    request.extensions().get::<User>()
}
