use sqlx::PgPool;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use async_nats::Client as NatsClient;

use common::{
    ApiDefinition,
    FlowDefinition,
    ConnectorDefinition,
    TriggerDefinition,
    RateLimitEvent,
};
use crate::crypto::CryptoService;
use crate::oidc::OidcAuth;
use crate::audit::AuditLogger;

type RedisConnection = redis::aio::ConnectionManager;

pub struct AppState {
    pub db: PgPool,
    pub nats: NatsClient,
    pub redis: RedisConnection,
    pub apis:                Arc<RwLock<Vec<ApiDefinition>>>,
    pub flows:               Arc<RwLock<Vec<FlowDefinition>>>,
    pub connectors:          Arc<RwLock<Vec<ConnectorDefinition>>>,
    pub triggers:            Arc<RwLock<Vec<TriggerDefinition>>>,
    pub connector_instances: Arc<RwLock<Vec<common::ConnectorInstance>>>,
    pub rate_limit_stats:    Arc<RwLock<HashMap<String, Vec<RateLimitEvent>>>>,
    pub jwt_secret:          String,
    pub crypto:              Arc<CryptoService>,
    pub oidc:                Arc<OidcAuth>,
    pub audit_logger:        Arc<AuditLogger>,
}
