# Dynamic Connector Instances

Instead of hardcoded connection strings, the platform now supports **dynamic connector registration** with encrypted credentials. This enables:

- Multiple database environments (dev, uat, prod) without code changes
- Secure credential storage with AES-256-GCM encryption
- Runtime connector instantiation per-flow execution
- Centralized credential management in Control Plane

---

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                    Control Plane :8081                        │
│                                                               │
│  POST /connector-instances    ──► Encrypt password           │
│  GET  /connector-instances         ──► Store in PostgreSQL   │
│  GET  /connector-instances/:id                                │
│  DELETE /connector-instances/:id                              │
│                                                               │
│  NATS publish: connector.instance.created/updated/deleted     │
└───────────────────────────┬───────────────────────────────────┘
                            │  NATS events
┌───────────────────────────▼───────────────────────────────────┐
│                     Data Plane :8080                          │
│                                                               │
│  Connector Registry (in-memory cache of instances)            │
│  ├─ Listen: connector.instance.* events                       │
│  └─ Store: ConnectorInstance with encrypted password          │
│                                                               │
│  On flow execution:                                           │
│  1. Extract connector IDs from flow steps                     │
│  2. For each connector (except http):                         │
│     ├─ Lookup instance from registry                          │
│     ├─ Decrypt password                                       │
│     ├─ Build connection URL                                   │
│     ├─ connector.connect()                                    │
│     └─ executor.register_connector(id, connector)             │
│  3. Execute flow with all connectors ready                    │
└───────────────────────────────────────────────────────────────┘
```

---

## Configuration

### Environment Variables

| Variable         | Description                          | Example                                                   |
|------------------|--------------------------------------|-----------------------------------------------------------|
| `ENCRYPTION_KEY` | 32-byte hex key for AES-256-GCM     | `64 hex chars` (auto-generated if not set — dev only!)   |
| `JWT_SECRET`     | Shared secret for JWT signatures     | `integration-platform-secret`                             |

**Generate a production ENCRYPTION_KEY:**
```bash
openssl rand -hex 32
# Example output: a1b2c3d4e5f6...  (64 characters)
```

Set in `.env` or docker-compose:
```bash
ENCRYPTION_KEY=a1b2c3d4e5f6789...
JWT_SECRET=your-jwt-secret
```

---

## Connector Instance Structure

```json
{
  "id":              "postgres_prod",
  "name":            "Production Database",
  "connector_type":  "postgres",
  "host":            "prod-db.example.com",
  "port":            5432,
  "database":        "myapp",
  "username":        "app_user",
  "password":        "plaintext-here",  // encrypted on storage
  "extra_attributes": {
    "ssl_mode": "require",
    "pool_size": 10
  }
}
```

### Supported Connector Types

| Type       | Required Fields                     | Notes                          |
|------------|-------------------------------------|--------------------------------|
| `postgres` | host, port, database, username, password | Standard PostgreSQL            |
| `http`     | None (stateless)                    | Already registered at startup  |

---

## API Reference

### POST /connector-instances

Create a new connector instance with credentials.

**Request:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "postgres_prod",
    "name": "Production Database",
    "connector_type": "postgres",
    "host": "prod-db.example.com",
    "port": 5432,
    "database": "myapp",
    "username": "app_user",
    "password": "secure-password-123",
    "extra_attributes": {"ssl_mode": "require"}
  }'
```

**Response:**
```json
{
  "id": "postgres_prod",
  "name": "Production Database",
  "connector_type": "postgres",
  "host": "prod-db.example.com",
  "port": 5432,
  "status": "created"
}
```

---

### GET /connector-instances

List all registered connector instances.

**Request:**
```bash
curl http://localhost:8081/connector-instances
```

**Response:**
```json
{
  "connectors": [
    {
      "id": "postgres_prod",
      "name": "Production Database",
      "connector_type": "postgres",
      "host": "prod-db.example.com",
      "port": 5432,
      "database": "myapp",
      "username": "app_user",
      "active": true,
      "created_at": "2024-02-18T10:00:00Z"
    }
  ],
  "count": 1
}
```

**Note:** `password_encrypted` is **never** returned in API responses.

---

### GET /connector-instances/:id

Get a single connector instance.

**Request:**
```bash
curl http://localhost:8081/connector-instances/postgres_prod
```

---

### DELETE /connector-instances/:id

Delete a connector instance.

**Request:**
```bash
curl -X DELETE http://localhost:8081/connector-instances/postgres_prod
```

---

## Usage in Flows

Reference the connector instance ID in flow step definitions:

```json
{
  "id": "data-sync-flow",
  "name": "Sync Production Data",
  "trigger": {"type": "http", "path": "/sync", "method": "POST"},
  "steps": [
    {
      "type": "call",
      "name": "fetch_users",
      "connector": "postgres_prod",     ← connector instance ID
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE active = true"
      }
    },
    {
      "type": "call",
      "name": "send_to_api",
      "connector": "http",
      "operation": "post",
      "params": {
        "url": "https://api.example.com/users",
        "body": "{{fetch_users.result}}"
      }
    }
  ]
}
```

**Flow execution sequence:**

1. User calls `POST /flows/data-sync-flow/execute`
2. Data Plane extracts connector IDs: `["postgres_prod", "http"]`
3. For `postgres_prod`:
   - Lookup instance from connector registry
   - Decrypt password: `secure-password-123`
   - Build URL: `postgresql://app_user:secure-password-123@prod-db.example.com:5432/myapp`
   - Connect and register: `executor.register_connector("postgres_prod", connector)`
4. For `http`: Already registered at startup (stateless)
5. Execute flow with both connectors ready

---

## Complete Example

### 1. Register Connector Instances

```bash
# Development database
curl -X POST http://localhost:8081/connector-instances -d '{
  "id": "postgres_dev",
  "name": "Dev Database",
  "connector_type": "postgres",
  "host": "localhost",
  "port": 5432,
  "database": "myapp_dev",
  "username": "dev_user",
  "password": "dev123"
}'

# UAT database
curl -X POST http://localhost:8081/connector-instances -d '{
  "id": "postgres_uat",
  "name": "UAT Database",
  "connector_type": "postgres",
  "host": "uat-db.internal",
  "port": 5432,
  "database": "myapp_uat",
  "username": "uat_user",
  "password": "uat456"
}'

# Production database
curl -X POST http://localhost:8081/connector-instances -d '{
  "id": "postgres_prod",
  "name": "Production Database",
  "connector_type": "postgres",
  "host": "prod-db.example.com",
  "port": 5432,
  "database": "myapp",
  "username": "app_user",
  "password": "prod-secret-789"
}'
```

### 2. Create Flow Using Connector

```bash
curl -X POST http://localhost:8081/flows -d '{
  "id": "report-flow",
  "name": "Generate Report",
  "trigger": {"type": "http", "path": "/report", "method": "GET"},
  "steps": [
    {
      "type": "call",
      "name": "get_data",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT COUNT(*) as total FROM orders WHERE created_at > NOW() - INTERVAL '\''1 day'\''"
      }
    },
    {
      "type": "log",
      "name": "log_result",
      "message": "Report generated: {{get_data.result}}"
    }
  ]
}'
```

### 3. Execute Flow

The flow automatically connects to `postgres_prod` before execution:

```bash
curl http://localhost:8080/api/trigger/report
```

**Logs:**
```
📥 Connector instance created: postgres_prod
🎯 HTTP Trigger: GET /report
🔌 Connecting postgres_prod...
✅ Connected postgres: postgres_prod
📨 Executing flow: report-flow
✅ Flow completed
```

---

## Frontend Integration

### Dropdown Population

```javascript
// Fetch available connector instances for dropdown
fetch('http://localhost:8081/connector-instances')
  .then(res => res.json())
  .then(data => {
    const connectors = data.connectors;
    
    // Filter by type for relevant dropdowns
    const dbConnectors = connectors.filter(c => c.connector_type === 'postgres');
    
    // Populate dropdown
    dbConnectors.forEach(conn => {
      dropdown.add(new Option(conn.name, conn.id));
      // Example: <option value="postgres_prod">Production Database</option>
    });
  });
```

### Flow Step Builder

```javascript
{
  "type": "call",
  "connector": selectedConnectorId,  // e.g., "postgres_prod" from dropdown
  "operation": "query",
  "params": { /* user input */ }
}
```

---

## Security

### Encryption Details

- **Algorithm:** AES-256-GCM (authenticated encryption)
- **Key:** 32-byte key from `ENCRYPTION_KEY` env var
- **Nonce:** 12-byte random nonce per encryption (prepended to ciphertext)
- **Storage:** Base64-encoded `nonce || ciphertext` in database

**Encryption flow:**
```
Plaintext password
    ↓
AES-256-GCM encrypt with random nonce
    ↓
Base64(nonce || ciphertext)
    ↓
Store in connector_instances.password_encrypted
```

**Decryption flow (Data Plane only):**
```
Fetch encrypted password from registry
    ↓
Base64 decode → extract nonce + ciphertext
    ↓
AES-256-GCM decrypt with ENCRYPTION_KEY
    ↓
Plaintext password → build connection URL
```

### Best Practices

1. **Rotate ENCRYPTION_KEY periodically**
   - Re-encrypt all passwords with new key
   - Zero-downtime rotation: dual-key support (future enhancement)

2. **Never log decrypted passwords**
   - Connection URLs are built in-memory only
   - Logs show connector ID, never credentials

3. **Use database SSL/TLS**
   - Set `extra_attributes: {"ssl_mode": "require"}` for PostgreSQL
   - Enforce encrypted connections to prevent MITM

4. **Restrict Control Plane access**
   - `/connector-instances` endpoints should be admin-only
   - Implement RBAC (future enhancement)

---

## Migration from Hardcoded Connectors

**Old way** (hardcoded in code):
```rust
let mut postgres = PostgresConnector::new("postgresql://...");
executor.register_connector("postgres".to_string(), Box::new(postgres));
```

**New way** (dynamic registration):
```bash
# Register once via API
curl -X POST http://localhost:8081/connector-instances -d '{...}'

# Reference in flows by ID
"connector": "postgres_prod"
```

**Migration steps:**

1. Identify all hardcoded connection strings
2. Register each as a connector instance via POST `/connector-instances`
3. Update flow definitions to use new connector IDs
4. Remove hardcoded registration code
5. Restart services with `ENCRYPTION_KEY` set

---

## Troubleshooting

### Connector not found

**Error:** `Connector not found: postgres_prod`

**Cause:** Connector instance not registered or Data Plane hasn't received NATS event.

**Fix:**
```bash
# Check Control Plane
curl http://localhost:8081/connector-instances

# Check Data Plane logs
docker-compose logs data-plane | grep "Connector instance"

# Re-register if missing
curl -X POST http://localhost:8081/connector-instances -d '{...}'
```

### Decryption failed

**Error:** `Decryption failed: invalid ciphertext`

**Cause:** `ENCRYPTION_KEY` mismatch between Control Plane and Data Plane.

**Fix:** Ensure both services use the same `ENCRYPTION_KEY`.

### Connection refused

**Error:** `Failed to connect postgres_prod: connection refused`

**Cause:** Database host/port unreachable or credentials invalid.

**Fix:**
- Verify host/port are correct
- Test connection manually: `psql -h host -p port -U username -d database`
- Check firewall rules

---

## Testing

```bash
./test-connector-instances.sh
```

The test script:
1. Registers 3 connector instances (dev, uat, prod)
2. Creates a flow using `postgres_prod`
3. Executes the flow
4. Verifies dynamic connection
5. Lists and deletes instances
6. Validates NATS events propagation

---

## Summary

| Feature | Old | New |
|---------|-----|-----|
| Connection config | Hardcoded in code | Dynamic via API |
| Multiple environments | Duplicate code | Multiple instances with same type |
| Credential security | Plain text in env vars | AES-256-GCM encrypted in DB |
| Changes | Requires code deploy | API call + NATS event |
| Frontend | Not possible | Dropdown from `/connector-instances` |

**Your platform now supports enterprise-grade connection management!** 🔌🔐
