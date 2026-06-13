# PostgreSQL Connector

## Register a Connector Instance

```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "id": "postgres_prod",
    "name": "Production Database",
    "connector_type": "postgres",
    "host": "db.example.com",
    "port": 5432,
    "database_name": "mydb",
    "username": "app_user",
    "password": "secret"
  }'
```

## Operations

### `query` — Return rows

```json
{
  "type": "call",
  "name": "get_users",
  "connector": "postgres_prod",
  "operation": "query",
  "params": {
    "sql": "SELECT id, name, email FROM users WHERE active = true LIMIT $1",
    "params": [100]
  }
}
```

Response: `{ "rows": [...], "count": N }`

### `execute` — Insert / Update / Delete

```json
{
  "type": "call",
  "name": "insert_record",
  "connector": "postgres_prod",
  "operation": "execute",
  "params": {
    "sql": "INSERT INTO events (user_id, type) VALUES ($1, $2)",
    "params": ["{{trigger.body.user_id}}", "login"]
  }
}
```

Response: `{ "rows_affected": N }`

## Parameterized Queries

Always use `$1`, `$2`, … placeholders — never string-interpolate values into SQL. This prevents SQL injection.

```json
{
  "sql": "SELECT * FROM orders WHERE user_id = $1 AND status = $2",
  "params": ["{{trigger.body.user_id}}", "{{trigger.body.status}}"]
}
```

## Connection Pooling

The runtime maintains a connection pool (SQLx `PgPool`) per connector instance. The pool is created lazily on first use and shared across concurrent flow executions.
