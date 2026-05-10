use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use async_nats::Client as NatsClient;
use serde::Serialize;

use common::{CircuitState, FlowDefinition};
use integration_runtime::{FlowExecutor, NodeRunResult};

use crate::connector_registry::ConnectorRegistry;
use crate::scheduler::FlowScheduler;

pub type RedisConnection = redis::aio::ConnectionManager;

#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: u64,
    pub opened_at: u64,
}

impl CircuitBreakerState {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: 0,
            opened_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowRunRecord {
    pub run_id: String,
    pub flow_id: String,
    pub flow_name: String,
    pub started_at: String,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub node_results: Vec<NodeRunResult>,
}

pub struct AppState {
    pub executor:           Arc<RwLock<FlowExecutor>>,
    pub flows:              Arc<RwLock<HashMap<String, FlowDefinition>>>,
    pub circuit_breakers:   Arc<RwLock<HashMap<String, CircuitBreakerState>>>,
    pub connector_registry: Arc<ConnectorRegistry>,
    pub nats:               NatsClient,
    pub redis:              RedisConnection,
    pub node_id:            String,
    pub jwt_secret:         String,
    pub scheduler:          Arc<FlowScheduler>,
    pub run_history:        Arc<RwLock<HashMap<String, VecDeque<FlowRunRecord>>>>,
}
