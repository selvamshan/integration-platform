//! Dynamic connector registry.
//! Connectors are instantiated on-demand per flow execution using credentials from Control Plane.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use integration_runtime::FlowExecutor;
use integration_runtime::connectors::{http::HttpConnector, postgres::PostgresConnector};
use common::{Connector, ConnectorInstance};

pub struct ConnectorRegistry {
    /// Available connector instances (from Control Plane)
    instances: Arc<RwLock<HashMap<String, ConnectorInstance>>>,
    /// Decryption service
    crypto:    Arc<CryptoService>,
}

/// Minimal crypto service for data-plane (decrypt only)
pub struct CryptoService {
    cipher: aes_gcm::Aes256Gcm,
}

impl CryptoService {
    pub fn new() -> Result<Self> {
        use aes_gcm::KeyInit;
        let key_hex = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY env var required"))?;

        let key_bytes = hex::decode(&key_hex)
            .map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY must be 64 hex chars"))?;

        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("ENCRYPTION_KEY must be 32 bytes"));
        }

        let key = aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(&key_bytes);
        Ok(Self { cipher: aes_gcm::Aes256Gcm::new(key) })
    }

    pub fn decrypt(&self, encrypted_b64: &str) -> Result<String> {
        use aes_gcm::{aead::Aead, Nonce};

        let combined = base64::decode(encrypted_b64)?;
        if combined.len() < 12 { return Err(anyhow::anyhow!("Data too short")); }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        Ok(String::from_utf8(plaintext)?)
    }
}

impl ConnectorRegistry {
    pub fn new(crypto: Arc<CryptoService>) -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            crypto,
        }
    }

    /// Register a connector instance from Control Plane
    pub async fn register(&self, instance: ConnectorInstance) {
        let id = instance.id.clone();
        self.instances.write().await.insert(id.clone(), instance);
        tracing::info!("📌 Registered connector instance: {}", id);
    }

    /// Unregister a connector instance
    pub async fn unregister(&self, id: &str) {
        self.instances.write().await.remove(id);
        tracing::info!("🗑️  Unregistered connector instance: {}", id);
    }

    /// Instantiate and register a connector for a flow execution
    pub async fn connect_for_flow(&self, connector_id: &str, executor: &mut FlowExecutor) -> Result<()> {
        let instances = self.instances.read().await;
        let instance = instances.get(connector_id)
            .ok_or_else(|| anyhow::anyhow!("Connector not found: {}", connector_id))?;

        if !instance.active {
            return Err(anyhow::anyhow!("Connector {} is inactive", connector_id));
        }

        // Decrypt password
        let password = match &instance.password_encrypted {
            Some(enc) => self.crypto.decrypt(enc)?,
            None => String::new(),
        };

        match instance.connector_type.as_str() {
            "postgres" => {
                let database = instance.database.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Postgres connector requires database field"))?;

                let port = instance.port.unwrap_or(5432);
                let username = instance.username.as_deref().unwrap_or("platform");
                let host = instance.host.as_deref().unwrap_or("localhost");
                let url = format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    username, password, host, port, database
                );

                let mut conn = PostgresConnector::new(url);
                conn.connect().await?;
                executor.register_connector(connector_id.to_string(), Box::new(conn));
                tracing::debug!("✅ Connected postgres: {}", connector_id);
            }
            "http" => {
                let mut conn = HttpConnector::new();
                conn.connect().await?;
                executor.register_connector(connector_id.to_string(), Box::new(conn));
                tracing::debug!("✅ Connected http: {}", connector_id);
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported connector type: {}", instance.connector_type));
            }
        }

        Ok(())
    }
}
