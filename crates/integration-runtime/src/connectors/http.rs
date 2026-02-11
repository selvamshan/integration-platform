use common::{Connector, Message, Result, Error};
use async_trait::async_trait;
use serde_json::json;

/// HTTP Connector for making HTTP requests
pub struct HttpConnector {
    client: reqwest::Client,
}

impl HttpConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connector for HttpConnector {
    async fn connect(&mut self) -> Result<()> {
        // HTTP client doesn't need explicit connection
        tracing::info!("HTTP connector initialized");
        Ok(())
    }
    
    async fn execute(&self, operation: &str, params: Message) -> Result<Message> {
        match operation {
            "get" => self.get(params).await,
            "post" => self.post(params).await,
            _ => Err(Error::Connector(format!("Unknown operation: {}", operation))),
        }
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }
}

impl HttpConnector {
    async fn get(&self, params: Message) -> Result<Message> {
        let url = params.payload.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'url' parameter".into()))?;
        
        tracing::info!("📡 HTTP GET: {}", url);
        
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("HTTP request failed: {}", e)))?;
        
        let status = response.status().as_u16();
        let body: serde_json::Value = response.json()
            .await
            .unwrap_or_else(|_| json!({}));
        
        tracing::info!("   Status: {}", status);
        
        let mut result = Message::new(json!({
            "status": status,
            "data": body
        }));
        
        result.attributes.insert("http_status".to_string(), status.to_string());
        
        Ok(result)
    }
    
    async fn post(&self, params: Message) -> Result<Message> {
        let url = params.payload.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'url' parameter".into()))?;
        
        let body = params.payload.get("body")
            .cloned()
            .unwrap_or(json!({}));
        
        tracing::info!("📡 HTTP POST: {}", url);
        
        let response = self.client.post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("HTTP request failed: {}", e)))?;
        
        let status = response.status().as_u16();
        let response_body: serde_json::Value = response.json()
            .await
            .unwrap_or_else(|_| json!({}));
        
        tracing::info!("   Status: {}", status);
        
        let mut result = Message::new(json!({
            "status": status,
            "data": response_body
        }));
        
        result.attributes.insert("http_status".to_string(), status.to_string());
        
        Ok(result)
    }
}
