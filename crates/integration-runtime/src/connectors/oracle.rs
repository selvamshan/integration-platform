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

#[cfg(test)]
mod tests {
    use super::*;
    use common::Connector;
    use serde_json::json;

    fn make_msg(payload: serde_json::Value) -> Message {
        Message::new(payload)
    }

    fn make_connector() -> OracleConnector {
        OracleConnector::new("user".into(), "pass".into(), "//localhost/xe".into())
    }

    // ── normalize_placeholders unit tests ─────────────────────────────────────

    #[test]
    fn normalize_dollar_style_preserves_number() {
        assert_eq!(
            normalize_placeholders("SELECT * FROM t WHERE id = $1"),
            "SELECT * FROM t WHERE id = :1"
        );
    }

    #[test]
    fn normalize_dollar_style_multiple_preserves_numbers() {
        assert_eq!(
            normalize_placeholders("INSERT INTO t (a, b) VALUES ($1, $2)"),
            "INSERT INTO t (a, b) VALUES (:1, :2)"
        );
    }

    #[test]
    fn normalize_question_mark_sequential_counter() {
        assert_eq!(
            normalize_placeholders("INSERT INTO t (a, b) VALUES (?, ?)"),
            "INSERT INTO t (a, b) VALUES (:1, :2)"
        );
    }

    #[test]
    fn normalize_dollar_style_does_not_use_counter() {
        // dollar style keeps the original number, not the sequential counter
        assert_eq!(
            normalize_placeholders("WHERE x = $3 AND y = $1"),
            "WHERE x = :3 AND y = :1"
        );
    }

    #[test]
    fn normalize_no_placeholders_unchanged() {
        let sql = "SELECT 1 FROM DUAL";
        assert_eq!(normalize_placeholders(sql), sql);
    }

    #[test]
    fn normalize_multi_digit_dollar_placeholder() {
        assert_eq!(
            normalize_placeholders("WHERE x = $10"),
            "WHERE x = :10"
        );
    }

    // ── value_to_bind_string unit tests ───────────────────────────────────────

    #[test]
    fn bind_string_from_string() {
        assert_eq!(value_to_bind_string(&json!("hello")), "hello");
    }

    #[test]
    fn bind_string_from_integer() {
        assert_eq!(value_to_bind_string(&json!(42)), "42");
    }

    #[test]
    fn bind_string_from_float() {
        assert_eq!(value_to_bind_string(&json!(3.14)), "3.14");
    }

    #[test]
    fn bind_string_from_bool_true() {
        assert_eq!(value_to_bind_string(&json!(true)), "1");
    }

    #[test]
    fn bind_string_from_bool_false() {
        assert_eq!(value_to_bind_string(&json!(false)), "0");
    }

    #[test]
    fn bind_string_from_null() {
        assert_eq!(value_to_bind_string(&json!(null)), "");
    }

    #[test]
    fn bind_string_from_object_serialized() {
        let val = json!({"k": "v"});
        assert_eq!(value_to_bind_string(&val), val.to_string());
    }

    // ── error path unit tests (no real DB) ────────────────────────────────────

    #[tokio::test]
    async fn query_before_connect_returns_error() {
        let c = make_connector();
        let err = c.execute("query", make_msg(json!({"sql": "SELECT 1 FROM DUAL"}))).await.unwrap_err();
        match &err {
            common::Error::Connector(msg) => assert!(msg.contains("Not connected")),
            _ => panic!("expected Connector error, got: {}", err),
        }
    }

    #[tokio::test]
    async fn execute_before_connect_returns_error() {
        let c = make_connector();
        let err = c.execute("execute", make_msg(json!({"sql": "BEGIN NULL; END;"}))).await.unwrap_err();
        assert!(matches!(err, common::Error::Connector(_)));
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn unknown_operation_returns_error() {
        let c = make_connector();
        let err = c.execute("bad_op", make_msg(json!({}))).await.unwrap_err();
        match err {
            common::Error::Connector(msg) => assert!(msg.contains("Unknown operation")),
            _ => panic!("expected Connector error"),
        }
    }

    #[tokio::test]
    async fn disconnect_without_connect_is_noop() {
        let mut c = make_connector();
        c.disconnect().await.unwrap();
        assert!(!c.connected);
    }

    // ── integration tests (require Oracle Instant Client + running Oracle DB) ──
    // Install Oracle Instant Client first:  https://oracle.github.io/odpi/doc/installation.html#linux
    // Then set env vars:
    //   TEST_ORACLE_USER=system
    //   TEST_ORACLE_PASS=oracle
    //   TEST_ORACLE_CS=//localhost:1521/XE
    //
    // If Oracle Instant Client is not installed the tests skip automatically.

    /// Returns true when Oracle Instant Client is missing; the test should return early.
    async fn oracle_client_missing(c: &mut OracleConnector) -> bool {
        match c.connect().await {
            Ok(()) => false,
            Err(e) if e.to_string().contains("DPI-1047") => {
                println!("SKIP — Oracle Instant Client not installed: {e}");
                true
            }
            Err(e) => panic!("unexpected connect error: {e}"),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn integration_connect_and_ping() {
        let user = std::env::var("TEST_ORACLE_USER").expect("set TEST_ORACLE_USER");
        let pass = std::env::var("TEST_ORACLE_PASS").expect("set TEST_ORACLE_PASS");
        let cs   = std::env::var("TEST_ORACLE_CS").expect("set TEST_ORACLE_CS");
        let mut c = OracleConnector::new(user, pass, cs);
        if oracle_client_missing(&mut c).await { return; }
        let result = c.execute("query", make_msg(json!({"sql": "SELECT 1 AS val FROM DUAL"}))).await.unwrap();
        let rows = result.payload["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        c.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_missing_sql_param_returns_error() {
        let user = std::env::var("TEST_ORACLE_USER").expect("set TEST_ORACLE_USER");
        let pass = std::env::var("TEST_ORACLE_PASS").expect("set TEST_ORACLE_PASS");
        let cs   = std::env::var("TEST_ORACLE_CS").expect("set TEST_ORACLE_CS");
        let mut c = OracleConnector::new(user, pass, cs);
        if oracle_client_missing(&mut c).await { return; }
        let err = c.execute("query", make_msg(json!({}))).await.unwrap_err();
        assert!(err.to_string().contains("Missing 'sql'"));
        c.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_insert_select_delete() {
        let user = std::env::var("TEST_ORACLE_USER").expect("set TEST_ORACLE_USER");
        let pass = std::env::var("TEST_ORACLE_PASS").expect("set TEST_ORACLE_PASS");
        let cs   = std::env::var("TEST_ORACLE_CS").expect("set TEST_ORACLE_CS");
        let mut c = OracleConnector::new(user.clone(), pass.clone(), cs.clone());
        if oracle_client_missing(&mut c).await { return; }

        // create a test table (global temporary for session scope)
        c.execute("execute", make_msg(json!({
            "sql": "CREATE GLOBAL TEMPORARY TABLE ora_conn_test (id NUMBER GENERATED ALWAYS AS IDENTITY, name VARCHAR2(100), age NUMBER) ON COMMIT PRESERVE ROWS"
        }))).await.ok(); // may already exist from a previous run

        let ins = c.execute("execute", make_msg(json!({
            "sql": "INSERT INTO ora_conn_test (name, age) VALUES ($1, $2)",
            "params": ["Alice", "30"]
        }))).await.unwrap();
        assert_eq!(ins.payload["rows_affected"], json!(1));

        c.execute("execute", make_msg(json!({
            "sql": "INSERT INTO ora_conn_test (name, age) VALUES (?, ?)",
            "params": ["Bob", "25"]
        }))).await.unwrap();

        let result = c.execute("query", make_msg(json!({
            "sql": "SELECT name, age FROM ora_conn_test WHERE age = $1",
            "params": ["30"]
        }))).await.unwrap();
        let rows = result.payload["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");

        let all = c.execute("query", make_msg(json!({
            "sql": "SELECT name FROM ora_conn_test ORDER BY name"
        }))).await.unwrap();
        assert_eq!(all.payload["count"], json!(2));

        c.execute("execute", make_msg(json!({
            "sql": "DELETE FROM ora_conn_test WHERE name = $1",
            "params": ["Bob"]
        }))).await.unwrap();

        c.disconnect().await.unwrap();
    }
}
