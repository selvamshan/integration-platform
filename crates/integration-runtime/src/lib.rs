use common::{Message, Result, Error, FlowDefinition, FlowStep, Connector};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, error};

pub mod connectors;

use connectors::http::HttpConnector;
use connectors::postgres::PostgresConnector;

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
                
                FlowStep::Transform { name, script } => {
                    info!("🔄 [{}] Transforming data", name);
                    // Simple script evaluation (just log for now)
                    info!("   Script: {}", script);
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
