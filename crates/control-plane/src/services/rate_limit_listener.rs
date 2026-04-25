use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

use common::RateLimitEvent;

use crate::state::AppState;

pub async fn rate_limit_event_listener(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📊 Starting Rate Limit Event Listener...");
    let mut subscriber = state.nats.subscribe("ratelimit.event").await?;
    tracing::info!("✅ Subscribed to ratelimit.event");

    while let Some(message) = subscriber.next().await {
        match serde_json::from_slice::<RateLimitEvent>(&message.payload) {
            Ok(event) => {
                if !event.allowed {
                    tracing::warn!(
                        "🚫 Rate limit exceeded: flow={}, key={}, count={}/{}",
                        event.flow_id, event.key, event.current_count, event.limit
                    );
                } else {
                    tracing::debug!(
                        "✅ Rate limit check: flow={}, count={}/{}",
                        event.flow_id, event.current_count, event.limit
                    );
                }

                let mut stats = state.rate_limit_stats.write().await;
                let flow_events = stats.entry(event.flow_id.clone()).or_insert_with(Vec::new);
                flow_events.push(event);

                if flow_events.len() > 1000 {
                    let excess = flow_events.len() - 1000;
                    flow_events.drain(0..excess);
                }
            }
            Err(e) => {
                tracing::error!("Failed to deserialize rate limit event: {}", e);
            }
        }
    }

    Ok(())
}
