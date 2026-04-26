use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

use common::FlowDefinition;

use crate::executor::execute_flow_inner;
use crate::state::AppState;

pub async fn load_scheduled_flows(state: &Arc<AppState>) -> Result<()> {
    let flows: Vec<FlowDefinition> = state.flows.read().await.values().cloned().collect();

    for flow in flows {
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
    }

    Ok(())
}

pub async fn register_with_control_plane(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📡 Registering with Control Plane...");
    state.nats.publish("dataplane.register", state.node_id.clone().into_bytes().into()).await?;
    tracing::info!("✅ Registration sent to Control Plane");
    tracing::info!("⏳ Waiting for flows to be pushed from Control Plane...");

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let flow_count = state.flows.read().await.len();
    tracing::info!("✅ Received {} flows from Control Plane", flow_count);

    Ok(())
}
