use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

use crate::state::AppState;

pub async fn listen_for_connector_instances(state: Arc<AppState>) -> Result<()> {
    tracing::info!("🔌 Listening for connector instance events...");

    let mut created = state.nats.subscribe("connector.instance.created").await?;
    let mut updated = state.nats.subscribe("connector.instance.updated").await?;
    let mut deleted = state.nats.subscribe("connector.instance.deleted").await?;

    loop {
        tokio::select! {
            Some(msg) = created.next() => {
                if let Ok(common::ConnectorInstanceEvent::Created { instance }) =
                    serde_json::from_slice::<common::ConnectorInstanceEvent>(&msg.payload)
                {
                    tracing::info!("📥 Connector instance created: {}", instance.id);
                    state.connector_registry.register(instance).await;
                }
            }
            Some(msg) = updated.next() => {
                if let Ok(common::ConnectorInstanceEvent::Updated { instance }) =
                    serde_json::from_slice::<common::ConnectorInstanceEvent>(&msg.payload)
                {
                    tracing::info!("📥 Connector instance updated: {}", instance.id);
                    state.connector_registry.register(instance).await;
                }
            }
            Some(msg) = deleted.next() => {
                if let Ok(common::ConnectorInstanceEvent::Deleted { id }) =
                    serde_json::from_slice::<common::ConnectorInstanceEvent>(&msg.payload)
                {
                    tracing::info!("📥 Connector instance deleted: {}", id);
                    state.connector_registry.unregister(&id).await;
                }
            }
        }
    }
}
