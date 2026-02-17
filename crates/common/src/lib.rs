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

// ─── Auth ────────────────────────────────────────────────────────────────────

/// Stored in PostgreSQL by the Control Plane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCredential {
    pub client_id: String,
    pub client_secret_hash: String, // bcrypt hash
    pub name: String,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// JWT claims issued by the Control Plane /auth/token endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,         // client_id
    pub client_name: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,         // unique token id
}

/// Request body for token issuance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    pub client_id: String,
    pub client_secret: String,
}

/// Response body for token issuance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub client_id: String,
}

/// Authenticated principal propagated through middleware
#[derive(Debug, Clone)]
pub struct AuthPrincipal {
    pub client_id: String,
    pub client_name: String,
    pub auth_method: AuthMethod,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    ClientCredentials,
    JwtToken,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::ClientCredentials => write!(f, "client_credentials"),
            AuthMethod::JwtToken => write!(f, "jwt"),
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitPolicy>,
     #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryPolicy>,
}

/// Retry policy for flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Backoff multiplier (e.g., 2.0 for exponential)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// Whether to use jitter
    #[serde(default = "default_jitter")]
    pub jitter: bool,
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_jitter() -> bool {
    false
}


/// Rate limit policy for flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    /// Maximum requests allowed in the time window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Rate limit key type
    #[serde(default = "default_key_type")]
    pub key_type: RateLimitKeyType,
    /// Custom message when rate limited
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn default_key_type() -> RateLimitKeyType {
    RateLimitKeyType::Global
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitKeyType {
    /// Global limit across all requests
    Global,
    /// Per IP address
    PerIp,
    /// Per user/API key (from headers)
    PerUser,
    /// Per flow
    PerFlow,
}

/// Rate limit event sent from Data Plane to Control Plane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEvent {
    pub flow_id: String,
    pub key: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub allowed: bool,
    pub current_count: u32,
    pub limit: u32,
}

/// Circuit breaker policy for flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerPolicy {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Time window in seconds to track failures
    pub window_seconds: u64,
    /// Time in seconds before attempting to close circuit
    pub timeout_seconds: u64,
    /// Success threshold to close circuit from half-open
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,
}

fn default_success_threshold() -> u32 {
    3
}

/// Circuit breaker state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
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
