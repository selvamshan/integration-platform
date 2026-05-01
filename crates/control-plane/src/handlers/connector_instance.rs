use axum::extract::{Path, State};
use axum::Json;
use async_nats::Client as NatsClient;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct CreateConnectorInstanceBody {
    pub id:               Option<String>,
    pub name:             String,
    pub connector_type:   String,
    pub host:             Option<String>,
    pub port:             Option<u16>,
    pub database_name:    Option<String>,
    pub username:         Option<String>,
    pub password:         Option<String>,
    pub extra_attributes: Option<Value>,
}

#[derive(serde::Deserialize)]
pub struct TestConnectorBody {
    pub connector_type: String,
    pub host:           Option<String>,
    pub port:           Option<u16>,
    pub database_name:  Option<String>,
    pub username:       Option<String>,
    pub password:       Option<String>,
}

pub async fn create_connector_instance(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateConnectorInstanceBody>,
) -> Result<Json<Value>, AppError> {
    let id = body.id.unwrap_or_else(|| format!("conn_{}", uuid::Uuid::new_v4().simple()));

    let is_db = matches!(body.connector_type.as_str(), "postgres" | "mysql" | "mssql" | "oracle");
    if is_db {
        if let Some(host) = &body.host {
            if host.starts_with("http://") || host.starts_with("https://") {
                return Err(AppError::BadRequest(
                    "Host must be a hostname or IP address, not a URL".into(),
                ));
            }
        }
    }

    let password_encrypted: Option<String> = match body.password.as_deref() {
        Some(pwd) => Some(state.crypto.encrypt(pwd)
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?),
        None => None,
    };

    let extra = body.extra_attributes.unwrap_or(json!({}));

    sqlx::query!(
        "INSERT INTO connector_instances
         (id, name, connector_type, host, port, database_name, username, password_encrypted, extra_attributes, active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)",
        id, body.name, body.connector_type, body.host, body.port.map(|p| p as i32),
        body.database_name, body.username, password_encrypted, extra
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let instance = common::ConnectorInstance {
        id:                 id.clone(),
        name:               body.name.clone(),
        connector_type:     body.connector_type.clone(),
        host:               body.host.clone(),
        port:               body.port,
        database:           body.database_name.clone(),
        username:           body.username.clone(),
        password_encrypted: password_encrypted.clone(),
        extra_attributes:   extra.clone(),
        active:             true,
        created_at:         chrono::Utc::now(),
    };

    state.connector_instances.write().await.push(instance.clone());

    let event = common::ConnectorInstanceEvent::Created { instance: instance.clone() };
    publish_connector_instance_event(&state.nats, &event).await?;

    tracing::info!("✅ Connector instance created: {} ({})", body.name, id);

    Ok(Json(json!({
        "id":             id,
        "name":           body.name,
        "connector_type": body.connector_type,
        "host":           body.host,
        "port":           body.port,
        "status":         "created"
    })))
}

pub async fn list_connector_instances(State(state): State<Arc<AppState>>) -> Json<Value> {
    let instances = state.connector_instances.read().await;
    let sanitized: Vec<Value> = instances.iter().map(|c| json!({
        "id":               c.id,
        "name":             c.name,
        "connector_type":   c.connector_type,
        "host":             c.host,
        "port":             c.port,
        "database_name":    c.database,
        "username":         c.username,
        "active":           c.active,
        "extra_attributes": c.extra_attributes,
        "created_at":       c.created_at,
    })).collect();
    Json(json!({ "connectors": sanitized, "count": sanitized.len() }))
}

pub async fn get_connector_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let instances = state.connector_instances.read().await;
    let instance = instances.iter().find(|c| c.id == id)
        .ok_or_else(|| AppError::NotFound(format!("Connector not found: {}", id)))?;
    Ok(Json(json!({
        "id":               instance.id,
        "name":             instance.name,
        "connector_type":   instance.connector_type,
        "host":             instance.host,
        "port":             instance.port,
        "database_name":    instance.database,
        "username":         instance.username,
        "active":           instance.active,
        "extra_attributes": instance.extra_attributes,
        "created_at":       instance.created_at,
    })))
}

pub async fn list_connector_instances_by_type(
    State(state): State<Arc<AppState>>,
    Path(connector_type): Path<String>,
) -> Json<Value> {
    let instances = state.connector_instances.read().await;
    let filtered: Vec<Value> = instances
        .iter()
        .filter(|c| c.connector_type == connector_type)
        .map(|c| json!({
            "id":             c.id,
            "name":           c.name,
            "connector_type": c.connector_type,
            "host":           c.host,
            "active":         c.active,
        }))
        .collect();
    Json(json!({ "instances": filtered }))
}

pub async fn test_connector_instance(Json(body): Json<TestConnectorBody>) -> Json<Value> {
    let result = match body.connector_type.as_str() {
        "postgres" => {
            test_postgres_connection(
                body.host.as_deref().unwrap_or("localhost"),
                body.port.unwrap_or(5432),
                body.database_name.as_deref().unwrap_or(""),
                body.username.as_deref().unwrap_or(""),
                body.password.as_deref().unwrap_or(""),
            ).await
        }
        "mysql" => {
            test_tcp_connection(
                body.host.as_deref().unwrap_or("localhost"),
                body.port.unwrap_or(3306),
            ).await
        }
        "mssql" => {
            test_tcp_connection(
                body.host.as_deref().unwrap_or("localhost"),
                body.port.unwrap_or(1433),
            ).await
        }
        "oracle" => {
            test_tcp_connection(
                body.host.as_deref().unwrap_or("localhost"),
                body.port.unwrap_or(1521),
            ).await
        }
        "http" => test_http_connection(body.host.as_deref().unwrap_or("")).await,
        other => Err(format!("Unknown connector type: {}", other)),
    };

    match result {
        Ok(msg)  => Json(json!({ "success": true,  "message": msg })),
        Err(msg) => Json(json!({ "success": false, "message": msg })),
    }
}

pub async fn delete_connector_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    sqlx::query!("DELETE FROM connector_instances WHERE id = $1", id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    state.connector_instances.write().await.retain(|c| c.id != id);

    let event = common::ConnectorInstanceEvent::Deleted { id: id.clone() };
    publish_connector_instance_event(&state.nats, &event).await?;

    tracing::info!("🗑️  Deleted connector instance: {}", id);
    Ok(Json(json!({ "deleted": id })))
}

pub async fn update_connector_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let name = payload["name"].as_str()
        .ok_or_else(|| AppError::BadRequest("Missing 'name' field".to_string()))?;
    let connector_type = payload["connector_type"].as_str()
        .ok_or_else(|| AppError::BadRequest("Missing 'connector_type' field".to_string()))?;
    let host = payload["host"].as_str();
    let port = payload["port"].as_i64().map(|p| p as i32);
    let database = payload["database_name"].as_str();
    let username = payload["username"].as_str();
    let password = payload["password"].as_str();
    let active = payload["active"].as_bool().unwrap_or(true);
    let extra_attributes = payload.get("extra_attributes").cloned();

    let mut instances = state.connector_instances.write().await;
    let instance = instances.iter_mut().find(|c| c.id == id)
        .ok_or_else(|| AppError::NotFound(format!("Connector instance not found: {}", id)))?;

    instance.name = name.to_string();
    instance.connector_type = connector_type.to_string();
    instance.host = host.map(|s| s.to_string());
    instance.port = port.map(|p| p as u16);
    instance.database = database.map(|s| s.to_string());
    instance.username = username.map(|s| s.to_string());
    instance.active = active;

    if let Some(attrs) = &extra_attributes {
        instance.extra_attributes = attrs.clone();
    }

    if let Some(pwd) = password {
        let encrypted = state.crypto.encrypt(pwd)
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;
        instance.password_encrypted = Some(encrypted);
    }

    let updated = instance.clone();
    drop(instances);

    let extra_attrs_json = extra_attributes.as_ref().map(|v| sqlx::types::Json(v.clone()));

    sqlx::query!(
        r#"
        UPDATE connector_instances
        SET name = $1, connector_type = $2, host = $3, port = $4,
            database_name = $5, username = $6, password_encrypted = $7, active = $8,
            extra_attributes = $9
        WHERE id = $10
        "#,
        updated.name,
        updated.connector_type,
        updated.host,
        updated.port.map(|p| p as i32),
        updated.database,
        updated.username,
        updated.password_encrypted,
        updated.active,
        extra_attrs_json as Option<sqlx::types::Json<Value>>,
        id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let event = common::ConnectorInstanceEvent::Updated { instance: updated.clone() };
    if let Err(e) = publish_connector_instance_event(&state.nats, &event).await {
        tracing::warn!("Failed to publish connector update event: {}", e);
    }

    tracing::info!("✏️ Updated connector instance: {}", updated.name);

    Ok(Json(json!({
        "id":               updated.id,
        "name":             updated.name,
        "connector_type":   updated.connector_type,
        "host":             updated.host,
        "port":             updated.port,
        "database":         updated.database,
        "username":         updated.username,
        "active":           updated.active,
        "extra_attributes": updated.extra_attributes,
        "created_at":       updated.created_at,
    })))
}

pub async fn publish_connector_instance_event(
    nats: &NatsClient,
    event: &common::ConnectorInstanceEvent,
) -> Result<(), AppError> {
    let payload = serde_json::to_vec(event)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    nats.publish(event.subject(), payload.into()).await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    tracing::debug!("📤 Published {}", event.subject());
    Ok(())
}

fn resolve_host(host: &str) -> &str {
    match host {
        "localhost" | "127.0.0.1" => "host.docker.internal",
        other => other,
    }
}

async fn test_postgres_connection(
    host: &str, port: u16, database: &str, username: &str, password: &str,
) -> Result<String, String> {
    use sqlx::Connection;
    use sqlx::postgres::{PgConnectOptions, PgConnection};

    let opts = PgConnectOptions::new()
        .host(resolve_host(host))
        .port(port)
        .database(database)
        .username(username)
        .password(password);

    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        PgConnection::connect_with(&opts),
    ).await {
        Ok(Ok(mut conn)) => { conn.close().await.ok(); Ok("PostgreSQL connection successful".into()) }
        Ok(Err(e))       => Err(format!("Connection failed: {}", e)),
        Err(_)           => Err("Connection timed out after 10 seconds".into()),
    }
}

async fn test_tcp_connection(host: &str, port: u16) -> Result<String, String> {
    let addr = format!("{}:{}", resolve_host(host), port);
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::net::TcpStream::connect(&addr),
    ).await {
        Ok(Ok(_))  => Ok(format!("TCP connection to {} successful", addr)),
        Ok(Err(e)) => Err(format!("Connection failed: {}", e)),
        Err(_)     => Err("Connection timed out after 10 seconds".into()),
    }
}

async fn test_http_connection(url: &str) -> Result<String, String> {
    if url.is_empty() {
        return Err("Base URL is required for HTTP connector".into());
    }
    let url = &url
        .replace("://localhost:", "://host.docker.internal:")
        .replace("://127.0.0.1:", "://host.docker.internal:");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    match client.head(url).send().await {
        Ok(resp) => Ok(format!("HTTP connection successful ({})", resp.status())),
        Err(e)   => Err(format!("HTTP request failed: {}", e)),
    }
}
