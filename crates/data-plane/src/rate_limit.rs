use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use redis::AsyncCommands;
use serde_json::json;

use common::{RateLimitEvent, RateLimitKeyType, RateLimitPolicy};

use crate::state::AppState;

pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();

    let path = request.uri().path();
    let flow_id = if path.starts_with("/flows/") && path.ends_with("/execute") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 { Some(parts[2].to_string()) } else { None }
    } else if path.starts_with("/api/trigger/") {
        let trigger_path = path.strip_prefix("/api/trigger/").unwrap_or("");
        let method_str = request.method().as_str();
        let flows = state.flows.read().await;
        flows.values()
            .find(|f| {
                if let common::Trigger::Http { path: p, method } = &f.trigger {
                    method.to_uppercase() == method_str.to_uppercase()
                        && crate::handlers::match_path_pattern_simple(p, trigger_path)
                } else {
                    false
                }
            })
            .map(|f| f.id.clone())
    } else {
        None
    };

    if let Some(ref flow_id_str) = flow_id {
        let flows = state.flows.read().await;
        if let Some(flow) = flows.get(flow_id_str) {
            if let Some(rate_limit) = &flow.rate_limit {
                let key = generate_rate_limit_key(flow_id_str, rate_limit, &client_ip);

                match check_rate_limit(&state, flow_id_str, &key, rate_limit).await {
                    Ok(allowed) => {
                        if !allowed {
                            tracing::warn!("🚫 Rate limit exceeded for flow {} (key: {})", flow_id_str, key);
                            let message = rate_limit.message.clone().unwrap_or_else(|| {
                                format!("Rate limit exceeded: {} requests per {} seconds",
                                    rate_limit.max_requests, rate_limit.window_seconds)
                            });
                            return (
                                StatusCode::TOO_MANY_REQUESTS,
                                Json(json!({
                                    "error":          message,
                                    "flow_id":        flow_id_str,
                                    "limit":          rate_limit.max_requests,
                                    "window_seconds": rate_limit.window_seconds
                                })),
                            ).into_response();
                        }
                        tracing::debug!("✅ Rate limit check passed for flow {} (key: {})", flow_id_str, key);
                    }
                    Err(e) => {
                        tracing::error!("Rate limit check error: {}", e);
                        // fail open
                    }
                }
            }
        }
    }

    next.run(request).await
}

pub fn generate_rate_limit_key(flow_id: &str, policy: &RateLimitPolicy, client_ip: &str) -> String {
    match policy.key_type {
        RateLimitKeyType::Global  => format!("ratelimit:global:{}", flow_id),
        RateLimitKeyType::PerIp   => format!("ratelimit:ip:{}:{}", client_ip, flow_id),
        RateLimitKeyType::PerFlow => format!("ratelimit:flow:{}", flow_id),
        RateLimitKeyType::PerUser => format!("ratelimit:user:{}:{}", client_ip, flow_id),
    }
}

pub async fn check_rate_limit(
    state: &AppState,
    flow_id: &str,
    key: &str,
    policy: &RateLimitPolicy,
) -> Result<bool> {
    let mut redis = state.redis.clone();

    let count: u32 = redis.incr(key, 1).await?;

    if count == 1 {
        let _: () = redis.expire::<_, ()>(key, policy.window_seconds as i64).await?;
    }

    let allowed = count <= policy.max_requests;

    let event = RateLimitEvent {
        flow_id:       flow_id.to_string(),
        key:           key.to_string(),
        timestamp:     chrono::Utc::now(),
        allowed,
        current_count: count,
        limit:         policy.max_requests,
    };

    let nats = state.nats.clone();
    tokio::spawn(async move {
        let payload = serde_json::to_vec(&event).unwrap();
        let _ = nats.publish("ratelimit.event", payload.into()).await;
    });

    Ok(allowed)
}
