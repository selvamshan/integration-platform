use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

use common::ConfigUpdate;

use crate::handlers::flow::publish_event;
use crate::state::AppState;

pub async fn flow_sync_service(state: Arc<AppState>) -> Result<()> {
    tracing::info!("🔄 Starting Flow Sync Service...");
    let mut subscriber = state.nats.subscribe("dataplane.register").await?;
    tracing::info!("✅ Flow Sync Service listening for Data Plane registrations");

    while let Some(message) = subscriber.next().await {
        let node_id = String::from_utf8_lossy(&message.payload).to_string();
        tracing::info!("📡 Data Plane registered: {}", node_id);

        let flows = state.flows.read().await;
        let flow_count = flows.len();

        for flow in flows.iter() {
            let event = ConfigUpdate::FlowCreated { flow: flow.clone() };
            if let Err(e) = publish_event(&state.nats, &event).await {
                tracing::error!("Failed to push flow {}: {}", flow.id, e);
            } else {
                tracing::debug!("  ✅ Pushed: {} ({})", flow.name, flow.id);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        tracing::info!("✅ Successfully pushed {} flows to Data Plane: {}", flow_count, node_id);
    }

    Ok(())
}
