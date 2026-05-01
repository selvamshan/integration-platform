use common::{Connector, Message, Result, Error};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Oracle Database Connector (via OCI — requires Oracle Instant Client)
pub struct OracleConnector {
    username:       String,
    password:       String,
    connect_string: String,
    connected:      bool,
}

impl OracleConnector {
    pub fn new(username: String, password: String, connect_string: String) -> Self {
        Self { username, password, connect_string, connected: false }
    }
}

#[async_trait]
impl Connector for OracleConnector {
    async fn connect(&mut self) -> Result<()> {
        tracing::info!("🔌 Connecting to Oracle...");
        let u  = self.username.clone();
        let p  = self.password.clone();
        let cs = self.connect_string.clone();

        tokio::task::spawn_blocking(move || {
            oracle::Connection::connect(&u, &p, &cs)
                .map_err(|e| Error::Connector(format!("Failed to connect to Oracle: {}", e)))
        })
        .await
        .map_err(|e| Error::Connector(format!("Task error: {}", e)))??;

        self.connected = true;
        tracing::info!("✅ Oracle connected");
        Ok(())
    }

    async fn execute(&self, operation: &str, params: Message) -> Result<Message> {
        match operation {
            "query"   => self.query(params).await,
            "execute" => self.execute_query(params).await,
            _ => Err(Error::Connector(format!("Unknown operation: {}", operation))),
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        tracing::info!("Oracle disconnected");
        Ok(())
    }
}

/// Convert `$1`, `$2`, … (PostgreSQL style) or `?` (MySQL style) to `:1`, `:2`, … (Oracle style).
fn normalize_placeholders(sql: &str) -> String {
    let mut result  = String::with_capacity(sql.len());
    let mut chars   = sql.chars().peekable();
    let mut counter = 1usize;

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            let mut num = String::new();
            while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                num.push(chars.next().unwrap());
            }
            result.push(':');
            result.push_str(&num);
        } else if ch == '?' {
            result.push_str(&format!(":{}", counter));
            counter += 1;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Convert a JSON value to a String that Oracle can accept as a bind parameter.
fn value_to_bind_string(v: &Value) -> String {
    match v {
        Value::String(s)  => s.clone(),
        Value::Number(n)  => n.to_string(),
        Value::Bool(b)    => if *b { "1".into() } else { "0".into() },
        Value::Null       => String::new(),
        _                 => v.to_string(),
    }
}

impl OracleConnector {
    /// Create a fresh OCI connection — used inside spawn_blocking closures.
    fn open_conn(u: &str, p: &str, cs: &str) -> Result<oracle::Connection> {
        oracle::Connection::connect(u, p, cs)
            .map_err(|e| Error::Connector(format!("Oracle connection failed: {}", e)))
    }

    async fn query(&self, params: Message) -> Result<Message> {
        if !self.connected {
            return Err(Error::Connector("Not connected to Oracle".into()));
        }

        let sql_raw: String = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?
            .to_string();
        let sql = normalize_placeholders(&sql_raw);

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let u  = self.username.clone();
        let p  = self.password.clone();
        let cs = self.connect_string.clone();

        tracing::info!("📊 Executing Oracle query: {} ({} params)", sql, sql_params.len());

        let results = tokio::task::spawn_blocking(move || -> Result<Vec<Value>> {
            let conn = Self::open_conn(&u, &p, &cs)?;

            let bind_strings: Vec<String> = sql_params.iter().map(value_to_bind_string).collect();
            let bind_refs: Vec<&dyn oracle::sql_type::ToSql> =
                bind_strings.iter().map(|s| s as &dyn oracle::sql_type::ToSql).collect();

            let stmt = conn.query(&sql, bind_refs.as_slice())
                .map_err(|e| Error::Connector(format!("Query failed: {}", e)))?;

            let col_names: Vec<String> = stmt.column_info().iter()
                .map(|c| c.name().to_string())
                .collect();

            let mut rows = Vec::new();
            for row_result in stmt {
                let row = row_result
                    .map_err(|e| Error::Connector(format!("Row error: {}", e)))?;
                let mut row_data = serde_json::Map::new();
                for (idx, name) in col_names.iter().enumerate() {
                    let value: Value = match row.get::<usize, Option<String>>(idx) {
                        Ok(Some(s)) => json!(s),
                        Ok(None)    => json!(null),
                        Err(_)      => json!(null),
                    };
                    row_data.insert(name.clone(), value);
                }
                rows.push(json!(row_data));
            }
            Ok(rows)
        })
        .await
        .map_err(|e| Error::Connector(format!("Task error: {}", e)))??;

        tracing::info!("   Rows returned: {}", results.len());
        Ok(Message::new(json!({ "rows": results, "count": results.len() })))
    }

    async fn execute_query(&self, params: Message) -> Result<Message> {
        if !self.connected {
            return Err(Error::Connector("Not connected to Oracle".into()));
        }

        let sql_raw: String = params.payload.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Connector("Missing 'sql' parameter".into()))?
            .to_string();
        let sql = normalize_placeholders(&sql_raw);

        let sql_params: Vec<Value> = params.payload.get("params")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let u  = self.username.clone();
        let p  = self.password.clone();
        let cs = self.connect_string.clone();

        tracing::info!("📊 Executing Oracle statement: {} ({} params)", sql, sql_params.len());

        let rows_affected = tokio::task::spawn_blocking(move || -> Result<u64> {
            let conn = Self::open_conn(&u, &p, &cs)?;

            let bind_strings: Vec<String> = sql_params.iter().map(value_to_bind_string).collect();
            let bind_refs: Vec<&dyn oracle::sql_type::ToSql> =
                bind_strings.iter().map(|s| s as &dyn oracle::sql_type::ToSql).collect();

            let stmt = conn.execute(&sql, bind_refs.as_slice())
                .map_err(|e| Error::Connector(format!("Execute failed: {}", e)))?;
            let count = stmt.row_count()
                .map_err(|e| Error::Connector(format!("row_count failed: {}", e)))?;
            conn.commit()
                .map_err(|e| Error::Connector(format!("Commit failed: {}", e)))?;
            Ok(count)
        })
        .await
        .map_err(|e| Error::Connector(format!("Task error: {}", e)))??;

        tracing::info!("   Rows affected: {}", rows_affected);
        Ok(Message::new(json!({ "rows_affected": rows_affected })))
    }
}
