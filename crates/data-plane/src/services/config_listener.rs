use anyhow::Result;
use futures::StreamExt;
use serde_json::Value;
use std::sync::Arc;

use common::ConfigUpdate;

use crate::executor::execute_flow_inner;
use crate::state::AppState;

pub async fn listen_for_config_updates(state: Arc<AppState>) -> Result<()> {
    tracing::info!("🎧 Starting config event listener...");
    let mut subscriber = state.nats.subscribe("config.>").await?;
    tracing::info!("✅ Subscribed to config.* events");

    while let Some(message) = subscriber.next().await {
        let subject = message.subject.as_str();
        match serde_json::from_slice::<ConfigUpdate>(&message.payload) {
            Ok(event) => {
                tracing::info!("📥 Received event from {}: {:?}", subject, event);
                if let Err(e) = handle_config_update(state.clone(), event).await {
                    tracing::error!("Failed to handle config update: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to deserialize config update: {}", e);
            }
        }
    }

    Ok(())
}

async fn handle_config_update(state: Arc<AppState>, event: ConfigUpdate) -> Result<()> {
    match event {
        ConfigUpdate::FlowCreated { flow } | ConfigUpdate::FlowUpdated { flow } => {
            let is_update = state.flows.read().await.contains_key(&flow.id);
            tracing::info!("{} flow: {} ({})",
                if is_update { "🔄 Updating" } else { "➕ Adding" },
                flow.name, flow.id);

            if let common::Trigger::Schedule { cron } = &flow.trigger {
                let state_clone = state.clone();
                let executor = move |flow_id: String, context: Value| {
                    let s = state_clone.clone();
                    tokio::spawn(async move {
                        execute_flow_inner(&s, &flow_id, context).await
                    })
                };
                state.scheduler.schedule_flow(flow.id.clone(), flow.name.clone(), cron, executor).await?;
            }

            state.flows.write().await.insert(flow.id.clone(), flow);
            tracing::info!("✅ Flow registered in data plane");
        }

        ConfigUpdate::FlowDeleted { flow_id } => {
            tracing::info!("➖ Removing flow: {}", flow_id);
            let _ = state.scheduler.unschedule_flow(&flow_id).await;
            state.flows.write().await.remove(&flow_id);
            tracing::info!("✅ Flow removed from data plane");
        }

        ConfigUpdate::ApiCreated { api } => {
            tracing::info!("📋 API registered: {} v{}", api.name, api.version);
        }

        _ => {
            tracing::debug!("Received other config event");
        }
    }

    Ok(())
}
