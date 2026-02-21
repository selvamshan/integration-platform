use common::{Message, Result, Error, FlowDefinition, FlowStep, Connector};
//use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, error};

pub mod connectors;
pub mod transformers;

// use connectors::http::HttpConnector;
// use connectors::postgres::PostgresConnector;
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
    
    /// Execute a flow
    pub async fn execute_flow(&self, flow: &FlowDefinition, input: Message) -> Result<Message> {
        info!("🚀 Executing flow: {}", flow.name);
        
        let mut current_output = input;
        
        for step in &flow.steps {
            match step {
                FlowStep::Log { name, message } => {
                    info!("📝 [{}] {}", name, message);
                    info!("   Current payload: {}", serde_json::to_string_pretty(&current_output.payload).unwrap_or_default());
                }
                
                FlowStep::Call { name, connector, operation, params } => {
                    info!("🔌 [{}] Calling connector: {} - {}", name, connector, operation);
                    
                    let conn = self.connectors.get(connector)
                        .ok_or_else(|| Error::Connector(format!("Connector not found: {}", connector)))?;
                    
                    // Merge params with current output
                    let mut call_params = current_output.clone();
                    if let serde_json::Value::Object(params_obj) = params {
                        if let serde_json::Value::Object(ref mut payload_obj) = call_params.payload {
                            for (k, v) in params_obj {
                                payload_obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    
                    current_output = conn.execute(operation, call_params).await?;
                    info!("   ✅ Connector call completed");
                }
                
                FlowStep::Transform { name, spec } => {
                    info!("🔄 [{}] Transforming data", name);
                    
                    // Determine what data to transform
                    // If payload has a "body" field (from HTTP trigger), transform that
                    // Otherwise transform the whole payload
                    let data_to_transform = if let Some(body) = current_output.payload.get("body") {
                        body
                    } else {
                        &current_output.payload
                    };
                    
                    // Apply transformation using TransformEngine
                    let transformed = TransformEngine::transform(data_to_transform, spec)
                        .map_err(|e| {
                            error!("   ❌ Transform failed: {}", e);
                            e
                        })?;
                    
                    // Update current output
                    // If we transformed the body, replace it; otherwise replace the whole payload
                    if current_output.payload.get("body").is_some() {
                        if let serde_json::Value::Object(ref mut map) = current_output.payload {
                            map.insert("body".to_string(), transformed);
                        }
                    } else {
                        current_output.payload = transformed;
                    }
                    
                    info!("   ✅ Transform completed");
                    info!("   Output: {}", serde_json::to_string_pretty(&current_output.payload).unwrap_or_default());
                }
            }
        }
        
        info!("✅ Flow completed: {}", flow.name);
        Ok(current_output)
    }
}

impl Default for FlowExecutor {
    fn default() -> Self {
        Self::new()
    }
}
