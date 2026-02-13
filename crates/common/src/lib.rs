use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message represents data flowing through the platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub headers: HashMap<String, String>,
    pub payload: serde_json::Value,
    pub attributes: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Message {
    pub fn new(payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            headers: HashMap::new(),
            payload,
            attributes: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Platform-wide result type
pub type Result<T> = std::result::Result<T, Error>;

/// Platform-wide error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Connector error: {0}")]
    Connector(String),
    
    #[error("Flow execution error: {0}")]
    Flow(String),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
}

/// Trait for all connectors
#[async_trait::async_trait]
pub trait Connector: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn execute(&self, operation: &str, params: Message) -> Result<Message>;
    async fn disconnect(&mut self) -> Result<()>;
}

/// Connector definition for UI palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorDefinition {
    pub id: String,
    pub name: String,
    pub connector_type: String,
    pub description: String,
    pub icon: Option<String>,
    pub operations: Vec<ConnectorOperation>,
    pub config_schema: serde_json::Value,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorOperation {
    pub name: String,
    pub description: String,
    pub parameters: Vec<OperationParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<serde_json::Value>,
}

/// Trigger definition for UI palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDefinition {
    pub id: String,
    pub name: String,
    pub trigger_type: String,
    pub description: String,
    pub icon: Option<String>,
    pub config_schema: serde_json::Value,
    pub enabled: bool,
}

/// Flow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    pub id: String,
    pub name: String,
    pub trigger: Trigger,
    pub steps: Vec<FlowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    Http { path: String, method: String },
    Schedule { cron: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlowStep {
    Log {
        name: String,
        message: String,
    },
    Call {
        name: String,
        connector: String,
        operation: String,
        params: serde_json::Value,
    },
    Transform {
        name: String,
        script: String,
    },
}

/// API Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub base_path: String,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub path: String,
    pub method: String,
    pub flow_id: String,
}

/// Config update events for event-driven distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfigUpdate {
    FlowCreated { flow: FlowDefinition },
    FlowUpdated { flow: FlowDefinition },
    FlowDeleted { flow_id: String },
    ApiCreated { api: ApiDefinition },
    ApiUpdated { api: ApiDefinition },
    ApiDeleted { api_id: String },
    ConnectorRegistered { connector: ConnectorDefinition },
    TriggerRegistered { trigger: TriggerDefinition },
}

impl ConfigUpdate {
    pub fn subject(&self) -> &'static str {
        match self {
            ConfigUpdate::FlowCreated { .. } => "config.flow.created",
            ConfigUpdate::FlowUpdated { .. } => "config.flow.updated",
            ConfigUpdate::FlowDeleted { .. } => "config.flow.deleted",
            ConfigUpdate::ApiCreated { .. } => "config.api.created",
            ConfigUpdate::ApiUpdated { .. } => "config.api.updated",
            ConfigUpdate::ApiDeleted { .. } => "config.api.deleted",
            ConfigUpdate::ConnectorRegistered { .. } => "config.connector.registered",
            ConfigUpdate::TriggerRegistered { .. } => "config.trigger.registered",
        }
    }
}
