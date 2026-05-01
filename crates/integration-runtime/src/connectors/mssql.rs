use common::{Connector, Message, Result, Error};
use async_trait::async_trait;
use futures::TryStreamExt;
use serde_json::{json, Value};
use tiberius::{Client, Config, Query, Row};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub struct MssqlConnector {
    client: Mutex<Option<Client<Compat<TcpStream>>>>,
    connection_string: String,
}

impl MssqlConnector {
    pub fn new(connection_string: String) -> Self {
        Self { client: Mutex::new(None), connection_string }
    }
}

#[async_trait]
impl Connector for MssqlConnector {
    async fn connect(&mut self) -> Result<()> {
        tracing::info!("Connecting to MS SQL Server...");
        let config = Config::from_ado_string(&self.connection_string)
            .map_err(|e| Error::Connector(format!("Invalid connection string: {}", e)))?;
        let tcp = TcpStream::connect(config.get_addr()).await
            .map_err(|e| Error::Connector(format!("TCP connect failed: {}", e)))?;
        tcp.set_nodelay(true)
            .map_err(|e| Error::Connector(format!("set_nodelay failed: {}", e)))?;
        let client = Client::connect(config, tcp.compat_write()).await
            .map_err(|e| Error::Connector(format!("TDS handshake failed: {}", e)))?;
        *self.client.lock().await = Some(client);
        tracing::info!("MS SQL Server connected");
        Ok(())
    }

    async fn execute(&self, operation: &str, params: Message) -> Result<Message> {
        match operation {
            "query"   => self.query(params).await,
            "execute" => self.execute_query(params).await,
            _         => Err(Error::Connector(format!("Unknown operation: {}", operation))),
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        *self.client.lock().await = None;
        tracing::info!("MS SQL Server disconnected");
        Ok(())
    }
}

/// Convert `$1`/`$2` (PostgreSQL style) or `?` (MySQL style) to `@p1`/`@p2` (MSSQL style).
fn normalize_placeholders(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    let mut counter = 1usize;
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                chars.next();
            }
            result.push_str(&format!("@p{}", counter));
            counter += 1;
        } else if ch == '?' {
            result.push_str(&format!("@p{}", counter));
            counter += 1;
        } else {
            result.push(ch);
        }
    }
    result
}

fn bind_param(query: &mut Query<'_>, value: &Value) {
    match value {
        Value::String(s) => query.bind(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i);
            } else {
                query.bind(n.as_f64().unwrap_or(0.0));
            }
        }
        Value::Bool(b) => query.bind(*b),
        Value::Null => query.bind(Option::<String>::None),
        _ => query.bind(value.to_string()),
    }
}

fn row_to_json(row: &Row) -> Value {
    let mut map = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        let v = if let Ok(Some(s)) = row.try_get::<&str, _>(i) {
            json!(s)
        } else if let Ok(Some(n)) = row.try_get::<i32, _>(i) {
            json!(n)
        } else if let Ok(Some(n)) = row.try_get::<i64, _>(i) {
            json!(n)
        } else if let Ok(Some(f)) = row.try_get::<f64, _>(i) {
            json!(f)
        } else if let Ok(Some(b)) = row.try_get::<bool, _>(i) {
            json!(b)
        } else {
            json!(null)
        };
        map.insert(name, v);
    }
    Value::Object(map)
}

impl MssqlConnector {
    async fn query(&self, params: Message) -> Result<Message> {
        let mut lock = self.client.lock().await;
        let client = lock.as_mut()
            .ok_or_else(|| Error::Connector("Not connected to MS SQL Server".into()))?;

        let sql_raw = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?;
        let sql = normalize_placeholders(sql_raw);

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::info!("Executing MSSQL query: {} ({} params)", sql, sql_params.len());

        let mut q = Query::new(sql);
        for param in &sql_params {
            bind_param(&mut q, param);
        }

        let stream = q.query(client).await
            .map_err(|e| Error::Connector(format!("Query failed: {}", e)))?;

        let rows: Vec<Row> = stream
            .into_row_stream()
            .try_collect()
            .await
            .map_err(|e| Error::Connector(format!("Fetch failed: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(row_to_json).collect();
        tracing::info!("Rows returned: {}", results.len());
        Ok(Message::new(json!({ "rows": results, "count": results.len() })))
    }

    async fn execute_query(&self, params: Message) -> Result<Message> {
        let mut lock = self.client.lock().await;
        let client = lock.as_mut()
            .ok_or_else(|| Error::Connector("Not connected to MS SQL Server".into()))?;

        let sql_raw = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?;
        let sql = normalize_placeholders(sql_raw);

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::info!("Executing MSSQL statement: {} ({} params)", sql, sql_params.len());

        let mut q = Query::new(sql);
        for param in &sql_params {
            bind_param(&mut q, param);
        }

        let result = q.execute(client).await
            .map_err(|e| Error::Connector(format!("Execute failed: {}", e)))?;

        let rows_affected: u64 = result.rows_affected().iter().sum();
        tracing::info!("Rows affected: {}", rows_affected);
        Ok(Message::new(json!({ "rows_affected": rows_affected })))
    }
}
