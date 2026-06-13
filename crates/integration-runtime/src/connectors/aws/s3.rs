use common::{Connector, Message, Result, Error};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_config::BehaviorVersion;
use base64::engine::general_purpose::STANDARD as B64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ConnectorConfig {
    /// Default bucket — can be overridden per operation
    #[serde(default)]
    pub bucket: Option<String>,
    /// AWS region (e.g. "us-east-1")
    #[serde(default = "default_region")]
    pub region: String,
    /// Explicit AWS access key ID (falls back to env / IAM role when absent)
    #[serde(default)]
    pub access_key_id: Option<String>,
    /// Explicit AWS secret access key
    #[serde(default)]
    pub secret_access_key: Option<String>,
    /// Custom endpoint URL — useful for MinIO or LocalStack
    #[serde(default)]
    pub endpoint_url: Option<String>,
    /// Force path-style addressing (required for MinIO)
    #[serde(default)]
    pub path_style: bool,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

impl Default for S3ConnectorConfig {
    fn default() -> Self {
        Self {
            bucket: None,
            region: default_region(),
            access_key_id: None,
            secret_access_key: None,
            endpoint_url: None,
            path_style: false,
        }
    }
}

pub struct S3Connector {
    config: S3ConnectorConfig,
    client: Option<Client>,
}

impl S3Connector {
    pub fn new() -> Self {
        Self {
            config: S3ConnectorConfig::default(),
            client: None,
        }
    }

    pub fn from_config(extra_attributes: &serde_json::Value) -> Result<Self> {
        let config: S3ConnectorConfig = serde_json::from_value(extra_attributes.clone())
            .map_err(|e| Error::Connector(format!("Invalid S3 connector config: {}", e)))?;
        Ok(Self { config, client: None })
    }

    fn client(&self) -> Result<&Client> {
        self.client
            .as_ref()
            .ok_or_else(|| Error::Connector("S3 connector not connected — call connect() first".into()))
    }

    fn resolve_bucket<'a>(&'a self, params: &'a serde_json::Value) -> Result<&'a str> {
        params.get("bucket")
            .and_then(|v| v.as_str())
            .or_else(|| self.config.bucket.as_deref())
            .ok_or_else(|| Error::Connector("Missing 'bucket' parameter and no default bucket configured".into()))
    }

    fn resolve_key<'a>(&self, params: &'a serde_json::Value) -> Result<&'a str> {
        params.get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'key' parameter".into()))
    }

    // ── Operations ────────────────────────────────────────────────────────────

    async fn put_object(&self, params: Message) -> Result<Message> {
        let client = self.client()?;
        let bucket = self.resolve_bucket(&params.payload)?.to_string();
        let key = self.resolve_key(&params.payload)?.to_string();

        let raw_body = params.payload.get("body").cloned().unwrap_or(json!(""));
        let bytes: Vec<u8> = match &raw_body {
            serde_json::Value::String(s) => {
                // Try base64 first, fall back to raw UTF-8
                use base64::Engine as _;
                B64.decode(s).unwrap_or_else(|_| s.as_bytes().to_vec())
            }
            other => serde_json::to_vec(other)
                .map_err(|e| Error::Connector(format!("Failed to serialise body: {}", e)))?,
        };

        let content_type = params.payload.get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("application/octet-stream")
            .to_string();

        tracing::info!("📦 S3 PutObject: s3://{}/{}", bucket, key);

        let content_length = bytes.len() as i64;
        client
            .put_object()
            .bucket(&bucket)
            .key(&key)
            .content_type(content_type)
            .content_length(content_length)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| Error::Connector(format!("S3 PutObject failed: {}", e)))?;

        Ok(Message::new(json!({
            "bucket": bucket,
            "key": key,
            "status": "uploaded"
        })))
    }

    async fn get_object(&self, params: Message) -> Result<Message> {
        let client = self.client()?;
        let bucket = self.resolve_bucket(&params.payload)?.to_string();
        let key = self.resolve_key(&params.payload)?.to_string();

        tracing::info!("📥 S3 GetObject: s3://{}/{}", bucket, key);

        let resp = client
            .get_object()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("S3 GetObject failed: {}", e)))?;

        let content_type = resp.content_type().unwrap_or("").to_string();
        let content_length = resp.content_length().unwrap_or(0);

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| Error::Connector(format!("Failed to read S3 object body: {}", e)))?
            .into_bytes();

        use base64::Engine as _;
        let body_b64 = B64.encode(&bytes);

        Ok(Message::new(json!({
            "bucket": bucket,
            "key": key,
            "content_type": content_type,
            "content_length": content_length,
            "body": body_b64
        })))
    }

    async fn delete_object(&self, params: Message) -> Result<Message> {
        let client = self.client()?;
        let bucket = self.resolve_bucket(&params.payload)?.to_string();
        let key = self.resolve_key(&params.payload)?.to_string();

        tracing::info!("🗑️  S3 DeleteObject: s3://{}/{}", bucket, key);

        client
            .delete_object()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("S3 DeleteObject failed: {}", e)))?;

        Ok(Message::new(json!({
            "bucket": bucket,
            "key": key,
            "status": "deleted"
        })))
    }

    async fn list_objects(&self, params: Message) -> Result<Message> {
        let client = self.client()?;
        let bucket = self.resolve_bucket(&params.payload)?.to_string();
        let prefix = params.payload.get("prefix").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let max_keys = params.payload.get("max_keys").and_then(|v| v.as_i64()).unwrap_or(1000) as i32;

        tracing::info!("📋 S3 ListObjects: s3://{}/{}", bucket, prefix);

        let resp = client
            .list_objects_v2()
            .bucket(&bucket)
            .prefix(&prefix)
            .max_keys(max_keys)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("S3 ListObjects failed: {}", e)))?;

        let objects: Vec<serde_json::Value> = resp
            .contents()
            .iter()
            .map(|obj| json!({
                "key": obj.key().unwrap_or(""),
                "size": obj.size().unwrap_or(0),
                "last_modified": obj.last_modified().map(|t| t.to_string()).unwrap_or_default(),
                "etag": obj.e_tag().unwrap_or(""),
            }))
            .collect();

        Ok(Message::new(json!({
            "bucket": bucket,
            "prefix": prefix,
            "count": objects.len(),
            "truncated": resp.is_truncated().unwrap_or(false),
            "objects": objects
        })))
    }

    async fn head_object(&self, params: Message) -> Result<Message> {
        let client = self.client()?;
        let bucket = self.resolve_bucket(&params.payload)?.to_string();
        let key = self.resolve_key(&params.payload)?.to_string();

        tracing::info!("🔍 S3 HeadObject: s3://{}/{}", bucket, key);

        let resp = client
            .head_object()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("S3 HeadObject failed: {}", e)))?;

        Ok(Message::new(json!({
            "bucket": bucket,
            "key": key,
            "content_type": resp.content_type().unwrap_or(""),
            "content_length": resp.content_length().unwrap_or(0),
            "last_modified": resp.last_modified().map(|t| t.to_string()).unwrap_or_default(),
            "etag": resp.e_tag().unwrap_or(""),
            "exists": true
        })))
    }

    async fn copy_object(&self, params: Message) -> Result<Message> {
        let client = self.client()?;

        let source_bucket = params.payload.get("source_bucket")
            .and_then(|v| v.as_str())
            .or_else(|| self.config.bucket.as_deref())
            .ok_or_else(|| Error::Connector("Missing 'source_bucket' parameter".into()))?
            .to_string();

        let source_key = params.payload.get("source_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'source_key' parameter".into()))?
            .to_string();

        let dest_bucket = params.payload.get("dest_bucket")
            .and_then(|v| v.as_str())
            .or_else(|| self.config.bucket.as_deref())
            .ok_or_else(|| Error::Connector("Missing 'dest_bucket' parameter".into()))?
            .to_string();

        let dest_key = params.payload.get("dest_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'dest_key' parameter".into()))?
            .to_string();

        let copy_source = format!("{}/{}", source_bucket, source_key);

        tracing::info!("📋 S3 CopyObject: s3://{}/{} → s3://{}/{}", source_bucket, source_key, dest_bucket, dest_key);

        client
            .copy_object()
            .copy_source(&copy_source)
            .bucket(&dest_bucket)
            .key(&dest_key)
            .send()
            .await
            .map_err(|e| Error::Connector(format!("S3 CopyObject failed: {}", e)))?;

        Ok(Message::new(json!({
            "source_bucket": source_bucket,
            "source_key": source_key,
            "dest_bucket": dest_bucket,
            "dest_key": dest_key,
            "status": "copied"
        })))
    }
}

impl Default for S3Connector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connector for S3Connector {
    async fn connect(&mut self) -> Result<()> {
        tracing::info!("☁️  S3 connector initialising — region: {}", self.config.region);

        let region = aws_sdk_s3::config::Region::new(self.config.region.clone());

        let sdk_config = if let (Some(key_id), Some(secret)) = (
            self.config.access_key_id.clone(),
            self.config.secret_access_key.clone(),
        ) {
            tracing::info!("   Auth: explicit access key ({})", &key_id[..key_id.len().min(8)]);
            let creds = aws_credential_types::Credentials::new(
                key_id,
                secret,
                None,
                None,
                "s3-connector",
            );
            aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(creds)
                .load()
                .await
        } else {
            tracing::info!("   Auth: environment / IAM role");
            aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .load()
                .await
        };

        let mut builder = aws_sdk_s3::config::Builder::from(&sdk_config);

        if let Some(endpoint) = &self.config.endpoint_url {
            tracing::info!("   Endpoint: {}", endpoint);
            builder = builder.endpoint_url(endpoint);
        }

        if self.config.path_style {
            builder = builder.force_path_style(true);
        }

        self.client = Some(Client::from_conf(builder.build()));

        tracing::info!("✅ S3 connector ready");
        Ok(())
    }

    async fn execute(&self, operation: &str, params: Message) -> Result<Message> {
        match operation {
            "put_object"    => self.put_object(params).await,
            "get_object"    => self.get_object(params).await,
            "delete_object" => self.delete_object(params).await,
            "list_objects"  => self.list_objects(params).await,
            "head_object"   => self.head_object(params).await,
            "copy_object"   => self.copy_object(params).await,
            other => Err(Error::Connector(format!("Unknown S3 operation: {}", other))),
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.client = None;
        Ok(())
    }
}
