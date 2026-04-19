use common::{Message, Result, Error, FlowDefinition, FlowStep, FlowNode, FlowEdge, Connector};
use std::collections::HashMap;
use tracing::{info, error};
use serde_json::Value;

pub mod connectors;
pub mod transformers;
pub mod loop_executor;
pub mod templates;
pub mod graph_executor;

pub use loop_executor::{LoopExecutor, LoopType};

use loop_executor::{StepExecutor, LoopType as LT, LoopBody};
use templates::{resolve_templates, resolve_template_str};
use transformers::json::TransformEngine;

/// Integration runtime executor
pub struct FlowExecutor {
    connectors: HashMap<String, Box<dyn Connector>>,
}

impl FlowExecutor {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    /// Register a connector
    pub fn register_connector(&mut self, name: String, connector: Box<dyn Connector>) {
        self.connectors.insert(name, connector);
    }

    /// Returns true if a connector with this name is already registered.
    pub fn has_connector(&self, name: &str) -> bool {
        self.connectors.contains_key(name)
    }

    /// Execute a flow — dispatches to graph executor when nodes are present,
    /// falls back to linear executor for legacy step-only flows.
    pub async fn execute_flow<'a>(&'a self, flow: &'a FlowDefinition, input: Message) -> Result<Message> {
        info!("🚀 Executing flow: {}", flow.name);
        let result = if flow.is_graph_flow() {
            info!("   mode: graph ({} nodes, {} edges)", flow.nodes.len(), flow.edges.len());
            graph_executor::execute_graph(flow, input, |node, msg| {
                Box::pin(self.execute_step(&node.step, msg))
            }).await?
        } else {
            info!("   mode: linear ({} steps)", flow.steps.len());
            self.execute_steps(&flow.steps, input).await?
        };
        info!("✅ Flow completed: {}", flow.name);
        Ok(result)
    }

    /// Execute a sequence of steps, threading the message through each one.
    pub async fn execute_steps(&self, steps: &[FlowStep], input: Message) -> Result<Message> {
        let mut current = input;
        for step in steps {
            current = self.execute_step(step, current).await?;
        }
        Ok(current)
    }

    /// Execute a single flow step.
    pub async fn execute_step(&self, step: &FlowStep, mut current: Message) -> Result<Message> {
        match step {
            FlowStep::Log { name, message } => {
                let resolved = match resolve_template_str(message, &current.payload) {
                    Value::String(s) => s,
                    other => other.to_string(),
                };
                info!("📝 [{}] {}", name, resolved);
                info!("   Current payload: {}", serde_json::to_string_pretty(&current.payload).unwrap_or_default());
                Ok(current)
            }

            FlowStep::Call { name, connector, operation, params } => {
                info!("🔌 [{}] Calling connector: {} - {}", name, connector, operation);

                let conn = self.connectors.get(connector)
                    .ok_or_else(|| Error::Connector(format!("Connector not found: {}", connector)))?;

                let resolved = resolve_templates(params, &current.payload);

                let mut call_params = current.clone();
                if let Value::Object(params_obj) = resolved {
                    if let Value::Object(ref mut payload_obj) = call_params.payload {
                        for (k, v) in params_obj {
                            payload_obj.insert(k, v);
                        }
                    }
                }

                current = conn.execute(operation, call_params).await?;
                info!("   ✅ Connector call completed");
                Ok(current)
            }

            FlowStep::Transform { name, spec } => {
                info!("🔄 [{}] Transforming data", name);

                let data_to_transform = if let Some(body) = current.payload.get("body") {
                    body
                } else {
                    &current.payload
                };

                let transformed = TransformEngine::transform(data_to_transform, spec)
                    .map_err(|e| {
                        error!("   ❌ Transform failed: {}", e);
                        e
                    })?;

                if current.payload.get("body").is_some() {
                    if let Value::Object(ref mut map) = current.payload {
                        map.insert("body".to_string(), transformed);
                    }
                } else {
                    current.payload = transformed;
                }

                info!("   ✅ Transform completed");
                info!("   Output: {}", serde_json::to_string_pretty(&current.payload).unwrap_or_default());
                Ok(current)
            }

            FlowStep::Loop { name, loop_mode, condition, iterate_over, count, steps, nodes, edges, max_iterations } => {
                let loop_type = match loop_mode.as_str() {
                    "while" => {
                        let cond = condition.clone()
                            .ok_or_else(|| Error::Flow(format!("Loop '{}': while mode requires a condition", name)))?;
                        LT::While { condition: cond }
                    }
                    "foreach" => {
                        let path = iterate_over.clone()
                            .ok_or_else(|| Error::Flow(format!("Loop '{}': foreach mode requires iterate_over", name)))?;
                        LT::ForEach { items_path: path }
                    }
                    "count" => {
                        let n = count
                            .ok_or_else(|| Error::Flow(format!("Loop '{}': count mode requires count", name)))?;
                        LT::Count { count: n }
                    }
                    other => return Err(Error::Flow(format!("Loop '{}': unknown loop_mode '{}'", name, other))),
                };

                let body = if !nodes.is_empty() {
                    LoopBody::Graph { nodes, edges }
                } else {
                    LoopBody::Steps(steps)
                };

                let loop_executor = LoopExecutor::new()
                    .with_max_iterations(max_iterations.unwrap_or(1000));

                loop_executor.execute(name, loop_type, body, current, self).await
            }
        }
    }
}

impl Default for FlowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StepExecutor for FlowExecutor {
    async fn run_steps(&self, steps: &[FlowStep], message: Message) -> Result<Message> {
        self.execute_steps(steps, message).await
    }

    async fn run_graph(&self, nodes: &[FlowNode], edges: &[FlowEdge], message: Message) -> Result<Message> {
        graph_executor::execute_graph_nodes(nodes, edges, message, |node, msg| {
            Box::pin(self.execute_step(&node.step, msg))
        }).await
    }
}
