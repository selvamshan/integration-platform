use common::{Connector, Message, Result, Error};
use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgPoolOptions, Column};
use serde_json::{json, Value};

/// PostgreSQL Database Connector
pub struct PostgresConnector {
    pool: Option<PgPool>,
    connection_string: String,
}

impl PostgresConnector {
    pub fn new(connection_string: String) -> Self {
        Self {
            pool: None,
            connection_string,
        }
    }
}

#[async_trait]
impl Connector for PostgresConnector {
    async fn connect(&mut self) -> Result<()> {
        tracing::info!("🔌 Connecting to PostgreSQL...");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&self.connection_string)
            .await
            .map_err(|e| Error::Connector(format!("Failed to connect to database: {}", e)))?;

        self.pool = Some(pool);
        tracing::info!("✅ PostgreSQL connected");
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
        tracing::info!("PostgreSQL disconnected");
        Ok(())
    }
}

/// Bind a JSON value to a sqlx query as the appropriate Postgres type.
fn bind_json<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    value: &Value,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match value {
        Value::String(s) => {
            // Coerce numeric-looking strings to their native type so that
            // integer/bigint columns don't fail with "operator does not exist: integer = text".
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
        // Arrays/objects: serialize to JSON string
        _ => query.bind(value.to_string()),
    }
}

impl PostgresConnector {
    async fn query(&self, params: Message) -> Result<Message> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| Error::Connector("Not connected to database".into()))?;

        let sql = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?
            .to_string();

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::info!("📊 Executing query: {} ({} params)", sql, sql_params.len());

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
            .ok_or_else(|| Error::Connector("Not connected to database".into()))?;

        let sql = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?
            .to_string();

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::info!("📊 Executing statement: {} ({} params)", sql, sql_params.len());

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

#[cfg(test)]
mod tests {
    use super::*;
    use common::Connector;
    use serde_json::json;

    fn make_msg(payload: serde_json::Value) -> Message {
        Message::new(payload)
    }

    // ── unit tests (no real DB) ───────────────────────────────────────────────

    #[tokio::test]
    async fn query_before_connect_returns_error() {
        let connector = PostgresConnector::new("postgresql://localhost/test".into());
        let err = connector.execute("query", make_msg(json!({"sql": "SELECT 1"}))).await.unwrap_err();
        assert!(matches!(err, common::Error::Connector(_)));
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn execute_before_connect_returns_error() {
        let connector = PostgresConnector::new("postgresql://localhost/test".into());
        let err = connector.execute("execute", make_msg(json!({"sql": "SELECT 1"}))).await.unwrap_err();
        assert!(matches!(err, common::Error::Connector(_)));
    }

    #[tokio::test]
    async fn unknown_operation_returns_error() {
        let connector = PostgresConnector::new("postgresql://localhost/test".into());
        let err = connector.execute("bad_op", make_msg(json!({}))).await.unwrap_err();
        match err {
            common::Error::Connector(msg) => assert!(msg.contains("Unknown operation")),
            _ => panic!("expected Connector error"),
        }
    }

    #[tokio::test]
    async fn disconnect_without_connect_is_noop() {
        let mut connector = PostgresConnector::new("postgresql://localhost/test".into());
        connector.disconnect().await.unwrap();
    }

    // ── integration tests (require TEST_POSTGRES_URL env var) ────────────────

    #[tokio::test]
    #[ignore]
    async fn integration_connect_and_ping() {
        let url = std::env::var("TEST_POSTGRES_URL")
            .expect("set TEST_POSTGRES_URL to run integration tests");
        let mut c = PostgresConnector::new(url);
        c.connect().await.unwrap();
        let result = c.execute("query", make_msg(json!({"sql": "SELECT 1 AS val"}))).await.unwrap();
        let rows = result.payload["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        c.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_missing_sql_param_returns_error() {
        let url = std::env::var("TEST_POSTGRES_URL")
            .expect("set TEST_POSTGRES_URL to run integration tests");
        let mut c = PostgresConnector::new(url);
        c.connect().await.unwrap();
        let err = c.execute("query", make_msg(json!({}))).await.unwrap_err();
        assert!(err.to_string().contains("Missing 'sql'"));
        c.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_insert_select_delete() {
        let url = std::env::var("TEST_POSTGRES_URL")
            .expect("set TEST_POSTGRES_URL to run integration tests");
        let mut c = PostgresConnector::new(url);
        c.connect().await.unwrap();

        // Unique name avoids conflicts when tests run in parallel.
        // BIGINT matches the i64 that bind_json always produces for numbers,
        // preventing a type-OID mismatch in Postgres's extended-query protocol.
        let tbl = unique_table("pg_isd");
        c.execute("execute", make_msg(json!({
            "sql": format!("CREATE TABLE {tbl} (id BIGSERIAL PRIMARY KEY, name TEXT, age BIGINT)")
        }))).await.unwrap();

        let ins = c.execute("execute", make_msg(json!({
            "sql": format!("INSERT INTO {tbl} (name, age) VALUES ($1, $2)"),
            "params": ["Alice", 30]
        }))).await.unwrap();
        assert_eq!(ins.payload["rows_affected"], json!(1));

        c.execute("execute", make_msg(json!({
            "sql": format!("INSERT INTO {tbl} (name, age) VALUES ($1, $2)"),
            "params": ["Bob", 25]
        }))).await.unwrap();

        let result = c.execute("query", make_msg(json!({
            "sql": format!("SELECT name, age FROM {tbl} WHERE age = $1"),
            "params": [30]
        }))).await.unwrap();
        let rows = result.payload["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["age"], json!(30));

        let all = c.execute("query", make_msg(json!({
            "sql": format!("SELECT name FROM {tbl} ORDER BY name")
        }))).await.unwrap();
        assert_eq!(all.payload["count"], json!(2));

        let del = c.execute("execute", make_msg(json!({
            "sql": format!("DELETE FROM {tbl} WHERE name = $1"),
            "params": ["Alice"]
        }))).await.unwrap();
        assert_eq!(del.payload["rows_affected"], json!(1));

        c.execute("execute", make_msg(json!({
            "sql": format!("DROP TABLE {tbl}")
        }))).await.unwrap();
        c.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_null_param_binding() {
        let url = std::env::var("TEST_POSTGRES_URL")
            .expect("set TEST_POSTGRES_URL to run integration tests");
        let mut c = PostgresConnector::new(url);
        c.connect().await.unwrap();

        let tbl = unique_table("pg_null");
        c.execute("execute", make_msg(json!({
            "sql": format!("CREATE TABLE {tbl} (id BIGSERIAL PRIMARY KEY, val TEXT)")
        }))).await.unwrap();

        c.execute("execute", make_msg(json!({
            "sql": format!("INSERT INTO {tbl} (val) VALUES ($1)"),
            "params": [null]
        }))).await.unwrap();

        let result = c.execute("query", make_msg(json!({
            "sql": format!("SELECT val FROM {tbl}")
        }))).await.unwrap();
        let rows = result.payload["rows"].as_array().unwrap();
        assert_eq!(rows[0]["val"], serde_json::Value::Null);

        c.execute("execute", make_msg(json!({
            "sql": format!("DROP TABLE {tbl}")
        }))).await.unwrap();
        c.disconnect().await.unwrap();
    }
}

fn unique_table(prefix: &str) -> String {
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}_{ns}")
}
