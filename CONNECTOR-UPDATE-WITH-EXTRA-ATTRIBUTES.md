# Connector Instance Update with extra_attributes

Updated the PUT endpoint to support `extra_attributes` JSONB column.

---

## Updated Endpoint

### PUT /connector-instances/:id

Now supports updating `extra_attributes` for storing connector-specific configuration.

**Request:**
```json
PUT /connector-instances/http-api-1
{
  "name": "External API",
  "connector_type": "http",
  "host": "api.example.com",
  "port": 443,
  "username": "api-user",
  "password": "secret",
  "active": true,
  "extra_attributes": {
    "auth_type": "bearer",
    "bearer_token": "abc123",
    "base_url": "https://api.example.com/v1",
    "timeout": 30000,
    "headers": {
      "X-API-Key": "my-key",
      "X-Client-ID": "my-client"
    }
  }
}
```

**Response:**
```json
{
  "id": "http-api-1",
  "name": "External API",
  "connector_type": "http",
  "host": "api.example.com",
  "port": 443,
  "username": "api-user",
  "active": true,
  "extra_attributes": {
    "auth_type": "bearer",
    "bearer_token": "abc123",
    "base_url": "https://api.example.com/v1",
    "timeout": 30000,
    "headers": {
      "X-API-Key": "my-key",
      "X-Client-ID": "my-client"
    }
  },
  "created_at": "2024-02-22T10:00:00Z"
}
```

---

## extra_attributes Use Cases

### HTTP Connector with Authentication

```json
{
  "extra_attributes": {
    "auth_type": "bearer",
    "bearer_token": "eyJhbGc...",
    "base_url": "https://api.example.com/v1"
  }
}
```

### Database Connector with SSL

```json
{
  "extra_attributes": {
    "ssl_mode": "require",
    "ssl_cert": "/path/to/cert.pem",
    "connection_timeout": 30,
    "pool_size": 10
  }
}
```

### API Connector with Custom Headers

```json
{
  "extra_attributes": {
    "headers": {
      "X-API-Key": "secret-key",
      "X-Client-ID": "client-123",
      "Authorization": "Bearer token"
    },
    "retry_policy": {
      "max_retries": 3,
      "backoff": "exponential"
    }
  }
}
```

### MongoDB with Connection Options

```json
{
  "extra_attributes": {
    "replica_set": "rs0",
    "read_preference": "secondaryPreferred",
    "write_concern": "majority",
    "auth_source": "admin"
  }
}
```

---

## Implementation Details

### Handler Changes

```rust
// Extract extra_attributes from payload
let extra_attributes = payload.get("extra_attributes").cloned();

// Update in-memory instance
if let Some(attrs) = &extra_attributes {
    instance.extra_attributes = Some(attrs.clone());
}

// Convert to sqlx::types::Json for database
let extra_attrs_json = extra_attributes.as_ref()
    .map(|v| sqlx::types::Json(v.clone()));

// Update SQL includes extra_attributes
UPDATE connector_instances 
SET ..., extra_attributes = $9
WHERE id = $10
```

### Database Column

```sql
extra_attributes JSONB
```

Stores any valid JSON object with connector-specific configuration.

---

## Example Requests

### Update HTTP Connector with Bearer Auth

```bash
curl -X PUT http://localhost:8081/connector-instances/my-api \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "External API",
    "connector_type": "http",
    "host": "api.example.com",
    "port": 443,
    "active": true,
    "extra_attributes": {
      "auth_type": "bearer",
      "bearer_token": "abc123xyz",
      "base_url": "https://api.example.com/v1"
    }
  }'
```

### Update PostgreSQL with SSL

```bash
curl -X PUT http://localhost:8081/connector-instances/my-postgres \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "Production DB",
    "connector_type": "postgres",
    "host": "db.example.com",
    "port": 5432,
    "database": "production",
    "username": "app_user",
    "password": "secure_password",
    "active": true,
    "extra_attributes": {
      "ssl_mode": "require",
      "connection_timeout": 30,
      "pool_size": 10
    }
  }'
```

### Clear extra_attributes

```bash
curl -X PUT http://localhost:8081/connector-instances/my-connector \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "My Connector",
    "connector_type": "http",
    "active": true,
    "extra_attributes": null
  }'
```

---

## Frontend Integration

### TypeScript Interface

```typescript
interface ConnectorInstance {
  id: string
  name: string
  connector_type: string
  host?: string
  port?: number
  database?: string
  username?: string
  active: boolean
  extra_attributes?: {
    [key: string]: any
  }
  created_at: string
}
```

### Update with Extra Attributes

```typescript
const updateConnector = async (id: string) => {
  const response = await api.put(`/connector-instances/${id}`, {
    name: 'External API',
    connector_type: 'http',
    host: 'api.example.com',
    port: 443,
    active: true,
    extra_attributes: {
      auth_type: 'bearer',
      bearer_token: 'abc123',
      base_url: 'https://api.example.com/v1',
      headers: {
        'X-API-Key': 'my-key'
      }
    }
  })
  
  return response.data
}
```

---

## Benefits

✅ **Flexible configuration** — Store any connector-specific settings  
✅ **Type-safe storage** — JSONB validates JSON structure  
✅ **Indexed queries** — Can query on JSON fields  
✅ **No schema changes** — Add new fields without migrations  
✅ **Backward compatible** — Optional field  

---

## Testing

```bash
# Build and deploy
docker-compose build control-plane
docker-compose up -d control-plane

# Test update
curl -X PUT http://localhost:8081/connector-instances/test-conn \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "Test Connector",
    "connector_type": "http",
    "active": true,
    "extra_attributes": {
      "auth_type": "apikey",
      "api_key": "secret-key",
      "custom_field": "custom_value"
    }
  }'

# Verify
curl http://localhost:8081/connector-instances/test-conn \
  -H "Authorization: Bearer $TOKEN" | jq '.extra_attributes'
```

---

## Summary

✅ **extra_attributes support added** — Store connector-specific config  
✅ **JSONB column updated** — In database  
✅ **In-memory sync** — Updated  
✅ **NATS events** — Published with extra_attributes  
✅ **Response includes** — extra_attributes field  

**Your connectors can now store flexible configuration!** 🔌✨✅
