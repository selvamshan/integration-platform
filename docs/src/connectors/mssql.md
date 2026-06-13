# MSSQL (SQL Server) Connector

## Register a Connector Instance

```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "id": "mssql_prod",
    "name": "SQL Server Production",
    "connector_type": "mssql",
    "host": "sqlserver.example.com",
    "port": 1433,
    "database_name": "mydb",
    "username": "sa",
    "password": "StrongPassword123!"
  }'
```

## Operations

### `query`

```json
{
  "connector": "mssql_prod",
  "operation": "query",
  "params": {
    "sql": "SELECT TOP 100 * FROM dbo.Orders WHERE CustomerId = @p1",
    "params": ["{{trigger.body.customer_id}}"]
  }
}
```

### `execute`

```json
{
  "connector": "mssql_prod",
  "operation": "execute",
  "params": {
    "sql": "INSERT INTO dbo.AuditLog (Action, UserId) VALUES (@p1, @p2)",
    "params": ["{{trigger.body.action}}", "{{trigger.body.user_id}}"]
  }
}
```

SQL Server uses `@p1`, `@p2`, … positional placeholders.
