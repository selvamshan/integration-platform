use common::{Connector, Message, Result, Error};
use async_trait::async_trait;
use sqlx::{MySqlPool, Row, mysql::MySqlPoolOptions, Column};
use serde_json::{json, Value};

/// MySQL Database Connector
pub struct MySqlConnector {
    pool: Option<MySqlPool>,
    connection_string: String,
}

impl MySqlConnector {
    pub fn new(connection_string: String) -> Self {
        Self {
            pool: None,
            connection_string,
        }
    }
}

#[async_trait]
impl Connector for MySqlConnector {
    async fn connect(&mut self) -> Result<()> {
        tracing::info!("🔌 Connecting to MySQL...");

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&self.connection_string)
            .await
            .map_err(|e| Error::Connector(format!("Failed to connect to MySQL: {}", e)))?;

        self.pool = Some(pool);
        tracing::info!("✅ MySQL connected");
        Ok(())
    }

    async fn execute(&self, operation: &str, params: Message) -> Result<Message> {
        match operation {
            "query" => self.query(params).await,
            "execute" => self.execute_query(params).await,
            _ => Err(Error::Connector(format!("Unknown operation: {}", operation))),
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(pool) = &self.pool {
            pool.close().await;
        }
        self.pool = None;
        tracing::info!("MySQL disconnected");
        Ok(())
    }
}

fn bind_json<'q>(
    query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    value: &Value,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match value {
        Value::String(s) => {
            if let Ok(i) = s.parse::<i64>() {
                query.bind(i)
            } else if let Ok(f) = s.parse::<f64>() {
                query.bind(f)
            } else {
                query.bind(s.clone())
            }
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i)
            } else {
                query.bind(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::Bool(b) => query.bind(*b),
        Value::Null => query.bind(Option::<String>::None),
        _ => query.bind(value.to_string()),
    }
}

impl MySqlConnector {
    async fn query(&self, params: Message) -> Result<Message> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| Error::Connector("Not connected to MySQL".into()))?;

        let sql = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?
            .to_string();

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::info!("📊 Executing MySQL query: {} ({} params)", sql, sql_params.len());

        let mut q = sqlx::query(&sql);
        for param in &sql_params {
            q = bind_json(q, param);
        }

        let rows = q
            .fetch_all(pool)
            .await
            .map_err(|e| Error::Connector(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();

        for row in rows {
            let mut row_data = serde_json::Map::new();

            for (idx, column) in row.columns().iter().enumerate() {
                let column_name = column.name();

                let value = if let Ok(val) = row.try_get::<String, _>(idx) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<i32, _>(idx) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<i64, _>(idx) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<f64, _>(idx) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<bool, _>(idx) {
                    json!(val)
                } else {
                    json!(null)
                };

                row_data.insert(column_name.to_string(), value);
            }

            results.push(json!(row_data));
        }

        tracing::info!("   Rows returned: {}", results.len());

        Ok(Message::new(json!({
            "rows": results,
            "count": results.len()
        })))
    }

    async fn execute_query(&self, params: Message) -> Result<Message> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| Error::Connector("Not connected to MySQL".into()))?;

        let sql = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?
            .to_string();

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::info!("📊 Executing MySQL statement: {} ({} params)", sql, sql_params.len());

        let mut q = sqlx::query(&sql);
        for param in &sql_params {
            q = bind_json(q, param);
        }

        let result = q
            .execute(pool)
            .await
            .map_err(|e| Error::Connector(format!("Execute failed: {}", e)))?;

        let rows_affected = result.rows_affected();
        tracing::info!("   Rows affected: {}", rows_affected);

        Ok(Message::new(json!({
            "rows_affected": rows_affected
        })))
    }
}
