use anyhow::Result;
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;

use common::{ApiDefinition, ConnectorDefinition, FlowDefinition, TriggerDefinition};

use crate::state::AppState;

pub async fn load_flows_from_database(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📥 Loading flows from database into memory...");
    let rows = sqlx::query("SELECT config FROM flow_definitions")
        .fetch_all(&state.db)
        .await?;

    let mut flows = state.flows.write().await;
    for row in rows {
        let config: serde_json::Value = row.try_get("config")?;
        if let Ok(flow) = serde_json::from_value::<FlowDefinition>(config) {
            tracing::info!("  ➕ Loaded: {} ({})", flow.name, flow.id);
            flows.push(flow);
        }
    }
    tracing::info!("✅ Loaded {} flows into memory", flows.len());
    Ok(())
}

pub async fn load_apis_from_database(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📥 Loading API definitions from database into memory...");
    let rows = sqlx::query("SELECT config FROM api_definitions")
        .fetch_all(&state.db)
        .await?;

    let mut apis = state.apis.write().await;
    for row in rows {
        let config: serde_json::Value = row.try_get("config")?;
        if let Ok(api) = serde_json::from_value::<ApiDefinition>(config) {
            tracing::info!("  ➕ Loaded API: {} v{}", api.name, api.version);
            apis.push(api);
        }
    }
    tracing::info!("✅ Loaded {} API definitions into memory", apis.len());
    Ok(())
}

pub async fn load_connector_instances(state: &AppState) -> Result<()> {
    let rows = sqlx::query!(
        "SELECT id, name, connector_type, host, port, database_name, username,
                password_encrypted, extra_attributes, active, created_at
         FROM connector_instances WHERE active = TRUE"
    )
    .fetch_all(&state.db)
    .await?;

    let mut instances = state.connector_instances.write().await;
    for row in rows {
        instances.push(common::ConnectorInstance {
            id:                 row.id,
            name:               row.name,
            connector_type:     row.connector_type,
            host:               row.host,
            port:               row.port.map(|p| p as u16),
            database:           row.database_name,
            username:           row.username,
            password_encrypted: row.password_encrypted,
            extra_attributes:   row.extra_attributes,
            active:             row.active,
            created_at:         row.created_at,
        });
    }
    tracing::info!("✅ Loaded {} connector instances from DB", instances.len());
    Ok(())
}

pub async fn initialize_builtin_registry(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📋 Initializing built-in connectors and triggers...");

    save_connector(&state, ConnectorDefinition {
        id:             "http-connector".to_string(),
        name:           "HTTP/REST".to_string(),
        connector_type: "http".to_string(),
        description:    "Make HTTP GET/POST requests to external APIs".to_string(),
        icon:           Some("🌐".to_string()),
        operations: vec![
            common::ConnectorOperation {
                name:        "get".to_string(),
                description: "Make HTTP GET request".to_string(),
                parameters:  vec![
                    common::OperationParameter {
                        name:          "url".to_string(),
                        param_type:    "string".to_string(),
                        required:      true,
                        description:   "Target URL".to_string(),
                        default_value: None,
                    },
                ],
            },
            common::ConnectorOperation {
                name:        "post".to_string(),
                description: "Make HTTP POST request".to_string(),
                parameters:  vec![
                    common::OperationParameter {
                        name:          "url".to_string(),
                        param_type:    "string".to_string(),
                        required:      true,
                        description:   "Target URL".to_string(),
                        default_value: None,
                    },
                    common::OperationParameter {
                        name:          "body".to_string(),
                        param_type:    "object".to_string(),
                        required:      false,
                        description:   "Request body (JSON)".to_string(),
                        default_value: Some(json!({})),
                    },
                ],
            },
        ],
        config_schema: json!({"type": "object", "properties": {}}),
        enabled:       true,
    }).await?;

    save_connector(&state, ConnectorDefinition {
        id:             "postgres-connector".to_string(),
        name:           "PostgreSQL".to_string(),
        connector_type: "postgres".to_string(),
        description:    "Execute SQL queries on PostgreSQL database".to_string(),
        icon:           Some("🐘".to_string()),
        operations: vec![
            common::ConnectorOperation {
                name:        "query".to_string(),
                description: "Execute SELECT query".to_string(),
                parameters:  vec![common::OperationParameter {
                    name:          "sql".to_string(),
                    param_type:    "string".to_string(),
                    required:      true,
                    description:   "SQL SELECT statement".to_string(),
                    default_value: None,
                }],
            },
            common::ConnectorOperation {
                name:        "execute".to_string(),
                description: "Execute INSERT/UPDATE/DELETE".to_string(),
                parameters:  vec![common::OperationParameter {
                    name:          "sql".to_string(),
                    param_type:    "string".to_string(),
                    required:      true,
                    description:   "SQL statement".to_string(),
                    default_value: None,
                }],
            },
        ],
        config_schema: json!({"type": "object", "properties": {"connection_string": {"type": "string"}}}),
        enabled:       true,
    }).await?;

    save_connector(&state, ConnectorDefinition {
        id:             "mysql-connector".to_string(),
        name:           "MySQL".to_string(),
        connector_type: "mysql".to_string(),
        description:    "Execute SQL queries on MySQL database".to_string(),
        icon:           Some("🐬".to_string()),
        operations: vec![
            common::ConnectorOperation {
                name:        "query".to_string(),
                description: "Execute SELECT query".to_string(),
                parameters:  vec![
                    common::OperationParameter {
                        name:          "sql".to_string(),
                        param_type:    "string".to_string(),
                        required:      true,
                        description:   "SQL SELECT statement".to_string(),
                        default_value: None,
                    },
                    common::OperationParameter {
                        name:          "params".to_string(),
                        param_type:    "array".to_string(),
                        required:      false,
                        description:   "Positional query parameters".to_string(),
                        default_value: Some(json!([])),
                    },
                ],
            },
            common::ConnectorOperation {
                name:        "execute".to_string(),
                description: "Execute INSERT/UPDATE/DELETE".to_string(),
                parameters:  vec![
                    common::OperationParameter {
                        name:          "sql".to_string(),
                        param_type:    "string".to_string(),
                        required:      true,
                        description:   "SQL statement".to_string(),
                        default_value: None,
                    },
                    common::OperationParameter {
                        name:          "params".to_string(),
                        param_type:    "array".to_string(),
                        required:      false,
                        description:   "Positional query parameters".to_string(),
                        default_value: Some(json!([])),
                    },
                ],
            },
        ],
        config_schema: json!({"type": "object", "properties": {"connection_string": {"type": "string"}}}),
        enabled:       true,
    }).await?;

    save_trigger(&state, TriggerDefinition {
        id:           "http-trigger".to_string(),
        name:         "HTTP Request".to_string(),
        trigger_type: "http".to_string(),
        description:  "Trigger flow on HTTP GET/POST request".to_string(),
        icon:         Some("🌐".to_string()),
        config_schema: json!({
            "type": "object",
            "properties": {
                "path":   {"type": "string", "description": "URL path"},
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"]}
            },
            "required": ["path", "method"]
        }),
        enabled: true,
    }).await?;

    save_trigger(&state, TriggerDefinition {
        id:           "schedule-trigger".to_string(),
        name:         "Schedule".to_string(),
        trigger_type: "schedule".to_string(),
        description:  "Trigger flow on schedule (cron)".to_string(),
        icon:         Some("⏰".to_string()),
        config_schema: json!({
            "type": "object",
            "properties": {
                "cron": {"type": "string", "description": "Cron expression"}
            },
            "required": ["cron"]
        }),
        enabled: true,
    }).await?;

    tracing::info!("✅ Built-in registry initialized");
    Ok(())
}

async fn save_connector(state: &AppState, connector: ConnectorDefinition) -> Result<()> {
    sqlx::query(
        "INSERT INTO connector_definitions (name, connector_type, config) VALUES ($1, $2, $3) ON CONFLICT (name) DO NOTHING"
    )
    .bind(&connector.name)
    .bind(&connector.connector_type)
    .bind(serde_json::to_value(&connector)?)
    .execute(&state.db)
    .await?;

    let mut connectors = state.connectors.write().await;
    if !connectors.iter().any(|c| c.id == connector.id) {
        connectors.push(connector);
    }
    Ok(())
}

async fn save_trigger(state: &AppState, trigger: TriggerDefinition) -> Result<()> {
    sqlx::query(
        "INSERT INTO trigger_definitions (name, trigger_type, config) VALUES ($1, $2, $3) ON CONFLICT (name) DO NOTHING"
    )
    .bind(&trigger.name)
    .bind(&trigger.trigger_type)
    .bind(serde_json::to_value(&trigger)?)
    .execute(&state.db)
    .await?;

    let mut triggers = state.triggers.write().await;
    if !triggers.iter().any(|t| t.id == trigger.id) {
        triggers.push(trigger);
    }
    Ok(())
}
