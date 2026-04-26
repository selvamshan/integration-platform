use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use serde_json::{json, Value};

use common::{CircuitBreakerPolicy, FlowDefinition, FlowStep, Message};

use crate::circuit_breaker::{update_circuit_breaker_on_failure, update_circuit_breaker_on_success};
use crate::metrics::{
    FLOW_EXECUTION_DURATION, FLOW_EXECUTIONS_FAILED, FLOW_EXECUTIONS_SUCCESS,
    FLOW_EXECUTIONS_TOTAL, RETRY_ATTEMPTS_TOTAL, RETRY_EXHAUSTED_TOTAL, RETRY_SUCCESS_TOTAL,
};
use crate::state::AppState;

pub async fn execute_with_retry<F, Fut>(
    retry_policy: &common::RetryPolicy,
    flow_id: &str,
    mut operation: F,
) -> Result<Message, common::Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Message, common::Error>>,
{
    let mut attempt = 0;
    let mut delay_ms = retry_policy.initial_delay_ms;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    RETRY_SUCCESS_TOTAL.inc();
                    tracing::info!("✅ Flow {} succeeded on attempt {}/{}", flow_id, attempt, retry_policy.max_attempts);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt >= retry_policy.max_attempts {
                    RETRY_EXHAUSTED_TOTAL.inc();
                    tracing::error!("❌ Flow {} failed after {} attempts: {}", flow_id, attempt, e);
                    return Err(e);
                }

                RETRY_ATTEMPTS_TOTAL.inc();

                let actual_delay = if retry_policy.jitter {
                    let jitter_factor = 0.5 + (rand::random::<f64>() * 0.5);
                    (delay_ms as f64 * jitter_factor) as u64
                } else {
                    delay_ms
                };

                tracing::warn!("🔄 Flow {} failed on attempt {}/{}, retrying in {}ms: {}",
                    flow_id, attempt, retry_policy.max_attempts, actual_delay, e);

                tokio::time::sleep(tokio::time::Duration::from_millis(actual_delay)).await;
                delay_ms = ((delay_ms as f64) * retry_policy.backoff_multiplier) as u64;
                delay_ms = delay_ms.min(retry_policy.max_delay_ms);
            }
        }
    }
}

pub async fn connect_flow_connectors(state: &AppState, flow: &FlowDefinition) -> Result<()> {
    let mut connector_ids = HashSet::new();

    for node in &flow.nodes {
        if let FlowStep::Call { connector, .. } = &node.step {
            connector_ids.insert(connector.clone());
        }
    }
    for step in &flow.steps {
        if let FlowStep::Call { connector, .. } = step {
            connector_ids.insert(connector.clone());
        }
    }

    if connector_ids.is_empty() {
        return Ok(());
    }

    let mut executor = state.executor.write().await;
    for connector_id in connector_ids {
        if executor.has_connector(&connector_id) {
            continue;
        }
        state.connector_registry
            .connect_for_flow(&connector_id, &mut *executor)
            .await
            .map_err(|e| {
                tracing::error!("Failed to connect {}: {}", connector_id, e);
                e
            })?;
    }

    Ok(())
}

pub async fn execute_flow_inner(state: &Arc<AppState>, flow_id: &str, payload: Value) -> Result<Value> {
    tracing::info!("📨 Executing flow: {}", flow_id);

    FLOW_EXECUTIONS_TOTAL.inc();
    let start = Instant::now();

    let flow = {
        let flows = state.flows.read().await;
        flows.get(flow_id).cloned()
    };
    let flow = flow.ok_or_else(|| anyhow::anyhow!("Flow not found: {}", flow_id))?;

    connect_flow_connectors(state, &flow).await
        .map_err(|e| anyhow::anyhow!("Connector setup failed: {}", e))?;

    let cb_policy = flow.circuit_breaker.clone();
    let retry_policy = flow.retry.clone();
    let input = Message::new(payload);

    let result = if let Some(ref policy) = retry_policy {
        let executor   = state.executor.clone();
        let flow_clone = flow.clone();
        let input_clone = input.clone();
        execute_with_retry(policy, flow_id, move || {
            let executor    = executor.clone();
            let flow        = flow_clone.clone();
            let input       = input_clone.clone();
            async move {
                let executor = executor.read().await;
                executor.execute_flow(&flow, input).await
            }
        }).await
    } else {
        let executor = state.executor.read().await;
        executor.execute_flow(&flow, input).await
    };

    let duration = start.elapsed().as_secs_f64();
    FLOW_EXECUTION_DURATION.observe(duration);

    match result {
        Ok(output) => {
            FLOW_EXECUTIONS_SUCCESS.inc();
            if let Some(policy) = cb_policy {
                update_circuit_breaker_on_success(state.clone(), flow_id.to_string(), policy).await;
            }
            tracing::info!("✅ Flow {} completed in {:.3}s", flow_id, duration);
            Ok(json!({
                "flow_id":          flow_id,
                "flow_name":        flow.name,
                "status":           "completed",
                "result":           output.payload,
                "timestamp":        output.timestamp,
                "duration_seconds": duration,
                "node_id":          state.node_id
            }))
        }
        Err(e) => {
            FLOW_EXECUTIONS_FAILED.inc();
            if let Some(policy) = cb_policy {
                update_circuit_breaker_on_failure(state.clone(), flow_id.to_string(), policy).await;
            }
            tracing::error!("❌ Flow {} failed after {:.3}s: {}", flow_id, duration, e);
            Err(e.into())
        }
    }
}
