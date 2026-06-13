# MySQL Connector

## Register a Connector Instance

```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "id": "mysql_prod",
    "name": "MySQL Production",
    "connector_type": "mysql",
    "host": "mysql.example.com",
    "port": 3306,
    "database_name": "mydb",
    "username": "app_user",
    "password": "secret"
  }'
```

## Operations

### `query`

```json
{
  "connector": "mysql_prod",
  "operation": "query",
  "params": {
    "sql": "SELECT * FROM customers WHERE region = ?",
    "params": ["{{trigger.body.region}}"]
  }
}
```

### `execute`

```json
{
  "connector": "mysql_prod",
  "operation": "execute",
  "params": {
    "sql": "UPDATE orders SET status = ? WHERE id = ?",
    "params": ["shipped", "{{trigger.body.order_id}}"]
  }
}
```

MySQL uses `?` positional placeholders (unlike PostgreSQL's `$1`, `$2`).
