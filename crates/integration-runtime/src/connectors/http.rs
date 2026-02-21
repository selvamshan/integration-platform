use common::{Connector, Message, Result, Error};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;
use std::str::FromStr;

/// HTTP Connector with authentication and instance support
pub struct HttpConnector {
    client: reqwest::Client,
    config: HttpConnectorConfig,
    oauth_token_cache: Option<String>,
}

/// HTTP connector configuration from connector instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConnectorConfig {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    #[serde(default)]
    pub default_headers: Option<serde_json::Value>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
}

impl Default for HttpConnectorConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            auth: None,
            default_headers: None,
            timeout_ms: Some(30000),
            retry: None,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        header_name: String,
        api_key: String,
    },
    OAuth2 {
        token_url: String,
        client_id: String,
        client_secret: String,
        #[serde(default)]
        scope: Option<String>,
        #[serde(default)]
        grant_type: Option<String>,
    },
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

impl HttpConnector {
    /// Create new HTTP connector with default settings
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            config: HttpConnectorConfig::default(),
            oauth_token_cache: None,
        }
    }

    /// Create HTTP connector from connector instance extra_attributes
    pub fn from_config(extra_attributes: &serde_json::Value) -> Result<Self> {
        let config: HttpConnectorConfig = serde_json::from_value(extra_attributes.clone())
            .map_err(|e| Error::Connector(format!("Invalid HTTP connector config: {}", e)))?;

        let timeout = Duration::from_millis(config.timeout_ms.unwrap_or(30000));
        
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| Error::Connector(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            client,
            config,
            oauth_token_cache: None,
        })
    }

    /// Build complete URL from base_url and path
    fn build_url(&self, path: &str) -> String {
        match &self.config.base_url {
            Some(base) => {
                if path.starts_with("http://") || path.starts_with("https://") {
                    path.to_string()
                } else {
                    format!("{}{}", base.trim_end_matches('/'), path)
                }
            }
            None => path.to_string(),
        }
    }

    /// Build headers from default headers and request headers
    fn build_headers(
        &self,
        request_headers: Option<&serde_json::Value>,
    ) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // Add default headers from config
        if let Some(default_headers) = &self.config.default_headers {
            if let Some(obj) = default_headers.as_object() {
                for (key, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        if let Ok(header_name) = HeaderName::from_str(key) {
                            if let Ok(header_value) = HeaderValue::from_str(val_str) {
                                headers.insert(header_name, header_value);
                            }
                        }
                    }
                }
            }
        }

        // Add request-specific headers (override defaults)
        if let Some(req_headers) = request_headers {
            if let Some(obj) = req_headers.as_object() {
                for (key, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        if let Ok(header_name) = HeaderName::from_str(key) {
                            if let Ok(header_value) = HeaderValue::from_str(val_str) {
                                headers.insert(header_name, header_value);
                            }
                        }
                    }
                }
            }
        }

        Ok(headers)
    }

    /// Apply authentication to request headers
    async fn apply_auth(&mut self, headers: &mut HeaderMap) -> Result<()> {
        match &self.config.auth {
            None | Some(AuthConfig::None) => Ok(()),
            
            Some(AuthConfig::Bearer { token }) => {
                if let Ok(auth_value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                    headers.insert("authorization", auth_value);
                }
                Ok(())
            }
            
            Some(AuthConfig::Basic { username, password }) => {
                let credentials = base64::encode(format!("{}:{}", username, password));
                if let Ok(auth_value) = HeaderValue::from_str(&format!("Basic {}", credentials)) {
                    headers.insert("authorization", auth_value);
                }
                Ok(())
            }
            
            Some(AuthConfig::ApiKey { header_name, api_key }) => {
                if let Ok(header_name) = HeaderName::from_str(header_name) {
                    if let Ok(header_value) = HeaderValue::from_str(api_key) {
                        headers.insert(header_name, header_value);
                    }
                }
                Ok(())
            }
            
            Some(AuthConfig::OAuth2 { .. }) => {
                let token = self.get_oauth_token().await?;
                if let Ok(auth_value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                    headers.insert("authorization", auth_value);
                }
                Ok(())
            }
        }
    }

    /// Get OAuth2 token (with caching)
    async fn get_oauth_token(&mut self) -> Result<String> {
        // Return cached token if available
        if let Some(token) = &self.oauth_token_cache {
            return Ok(token.clone());
        }

        // Request new token
        if let Some(AuthConfig::OAuth2 {
            token_url,
            client_id,
            client_secret,
            scope,
            grant_type,
        }) = &self.config.auth
        {
            let mut form = vec![
                ("client_id", client_id.clone()),
                ("client_secret", client_secret.clone()),
                ("grant_type", grant_type.clone().unwrap_or_else(|| "client_credentials".to_string())),
            ];

            if let Some(s) = scope {
                form.push(("scope", s.clone()));
            }

            tracing::info!("🔐 Requesting OAuth2 token from {}", token_url);

            let response = self.client
                .post(token_url)
                .form(&form)
                .send()
                .await
                .map_err(|e| Error::Connector(format!("OAuth token request failed: {}", e)))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(Error::Connector(format!(
                    "OAuth token request failed with status {}: {}",
                    status, error_text
                )));
            }

            let token_response: serde_json::Value = response.json()
                .await
                .map_err(|e| Error::Connector(format!("Failed to parse OAuth response: {}", e)))?;

            let access_token = token_response["access_token"]
                .as_str()
                .ok_or_else(|| Error::Connector("No access_token in OAuth response".to_string()))?
                .to_string();

            // Cache the token
            self.oauth_token_cache = Some(access_token.clone());
            tracing::info!("✅ OAuth2 token obtained and cached");

            Ok(access_token)
        } else {
            Err(Error::Connector("OAuth2 not configured".to_string()))
        }
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<Message> {
        let status = response.status().as_u16();
        
        tracing::info!("   Status: {}", status);

        // Try to parse as JSON
        let body = response.json::<serde_json::Value>()
            .await
            .unwrap_or_else(|_| json!({}));

        let mut result = Message::new(json!({
            "status": status,
            "data": body
        }));

        result.attributes.insert("http_status".to_string(), status.to_string());

        Ok(result)
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
        tracing::info!("🌐 HTTP connector initialized");
        
        if let Some(base_url) = &self.config.base_url {
            tracing::info!("   Base URL: {}", base_url);
        }
        
        if let Some(auth) = &self.config.auth {
            let auth_type = match auth {
                AuthConfig::None => "None",
                AuthConfig::Bearer { .. } => "Bearer",
                AuthConfig::Basic { .. } => "Basic",
                AuthConfig::ApiKey { .. } => "API Key",
                AuthConfig::OAuth2 { .. } => "OAuth2",
            };
            tracing::info!("   Auth: {}", auth_type);
        }
        
        Ok(())
    }
    
    async fn execute(&self, operation: &str, params: Message) -> Result<Message> {
        match operation {
            "get" => self.get(params).await,
            "post" => self.post(params).await,
            "put" => self.put(params).await,
            "delete" => self.delete(params).await,
            "patch" => self.patch(params).await,
            "oauth_token" => self.oauth_token(params).await,
            _ => Err(Error::Connector(format!("Unknown operation: {}", operation))),
        }
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        self.oauth_token_cache = None;
        Ok(())
    }
}

impl HttpConnector {
    async fn get(&self, params: Message) -> Result<Message> {
        let path = params.payload.get("url")
            .or_else(|| params.payload.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'url' or 'path' parameter".into()))?;

        let url = self.build_url(path);
        let headers = self.build_headers(params.payload.get("headers"))?;

        tracing::info!("📡 HTTP GET: {}", url);

        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("HTTP GET failed: {}", e)))?;

        self.handle_response(response).await
    }

    async fn post(&self, params: Message) -> Result<Message> {
        let path = params.payload.get("url")
            .or_else(|| params.payload.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'url' or 'path' parameter".into()))?;

        let url = self.build_url(path);
        let body = params.payload.get("body").cloned().unwrap_or(json!({}));
        let headers = self.build_headers(params.payload.get("headers"))?;

        tracing::info!("📡 HTTP POST: {}", url);

        let response = self.client.post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("HTTP POST failed: {}", e)))?;

        self.handle_response(response).await
    }

    async fn put(&self, params: Message) -> Result<Message> {
        let path = params.payload.get("url")
            .or_else(|| params.payload.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'url' or 'path' parameter".into()))?;

        let url = self.build_url(path);
        let body = params.payload.get("body").cloned().unwrap_or(json!({}));
        let headers = self.build_headers(params.payload.get("headers"))?;

        tracing::info!("📡 HTTP PUT: {}", url);

        let response = self.client.put(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("HTTP PUT failed: {}", e)))?;

        self.handle_response(response).await
    }

    async fn delete(&self, params: Message) -> Result<Message> {
        let path = params.payload.get("url")
            .or_else(|| params.payload.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'url' or 'path' parameter".into()))?;

        let url = self.build_url(path);
        let headers = self.build_headers(params.payload.get("headers"))?;

        tracing::info!("📡 HTTP DELETE: {}", url);

        let response = self.client.delete(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("HTTP DELETE failed: {}", e)))?;

        self.handle_response(response).await
    }

    async fn patch(&self, params: Message) -> Result<Message> {
        let path = params.payload.get("url")
            .or_else(|| params.payload.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'url' or 'path' parameter".into()))?;

        let url = self.build_url(path);
        let body = params.payload.get("body").cloned().unwrap_or(json!({}));
        let headers = self.build_headers(params.payload.get("headers"))?;

        tracing::info!("📡 HTTP PATCH: {}", url);

        let response = self.client.patch(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("HTTP PATCH failed: {}", e)))?;

        self.handle_response(response).await
    }

    async fn oauth_token(&self, _params: Message) -> Result<Message> {
        // Create mutable copy for oauth operation
        let mut temp_connector = Self {
            client: self.client.clone(),
            config: self.config.clone(),
            oauth_token_cache: self.oauth_token_cache.clone(),
        };
        
        let token = temp_connector.get_oauth_token().await?;

        Ok(Message::new(json!({
            "access_token": token,
            "token_type": "Bearer"
        })))
    }
}
