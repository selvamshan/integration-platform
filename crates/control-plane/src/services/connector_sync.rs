use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

use crate::state::AppState;

pub async fn connector_instance_sync_service(state: Arc<AppState>) -> Result<()> {
    let mut subscriber = state.nats.subscribe("dataplane.register").await?;
    tracing::info!("🔄 Connector instance sync: listening for data-plane registrations");

    while let Some(msg) = subscriber.next().await {
        let node_id = String::from_utf8_lossy(&msg.payload).to_string();
        tracing::info!("📡 Data-plane registered: {}, syncing connector instances...", node_id);

        let instances = state.connector_instances.read().await.clone();
        for instance in &instances {
            let event = common::ConnectorInstanceEvent::Created { instance: instance.clone() };
            let payload = match serde_json::to_vec(&event) {
                Ok(p)  => p,
                Err(e) => {
                    tracing::error!("Failed to serialize connector instance {}: {}", instance.id, e);
                    continue;
                }
            };
            if let Err(e) = state.nats.publish(event.subject(), payload.into()).await {
                tracing::error!("Failed to push connector instance {}: {}", instance.id, e);
            }
        }

        tracing::info!("✅ Pushed {} connector instances to {}", instances.len(), node_id);
    }

    Ok(())
}
