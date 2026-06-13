# Oracle Connector

## Prerequisites

The Oracle connector requires Oracle Instant Client libraries installed on the host running the Data Plane.

```bash
# Ubuntu/Debian
apt-get install -y libaio1
# Download Instant Client from Oracle and set LD_LIBRARY_PATH
export LD_LIBRARY_PATH=/opt/oracle/instantclient_21_9:$LD_LIBRARY_PATH
```

## Register a Connector Instance

```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "id": "oracle_prod",
    "name": "Oracle Production",
    "connector_type": "oracle",
    "host": "oracle.example.com",
    "port": 1521,
    "database_name": "ORCL",
    "username": "app_user",
    "password": "secret"
  }'
```

## Operations

### `query`

```json
{
  "connector": "oracle_prod",
  "operation": "query",
  "params": {
    "sql": "SELECT * FROM EMPLOYEES WHERE DEPARTMENT_ID = :1",
    "params": ["{{trigger.body.dept_id}}"]
  }
}
```

### `execute`

```json
{
  "connector": "oracle_prod",
  "operation": "execute",
  "params": {
    "sql": "UPDATE EMPLOYEES SET SALARY = :1 WHERE EMPLOYEE_ID = :2",
    "params": ["{{trigger.body.salary}}", "{{trigger.body.emp_id}}"]
  }
}
```

Oracle uses `:1`, `:2`, … positional bind variables.
