# HTTP Triggers - All Methods Supported

HTTP triggers now support **GET, POST, PUT, and DELETE** methods, allowing flows to be invoked via any standard HTTP method.

---

## Overview

```
HTTP Request                    Data Plane                    Flow Execution
     │                              │                              │
     │  GET /api/trigger/users      │                              │
     │ ─────────────────────────────►                              │
     │                              │  Match: path + method        │
     │                              │  Extract: body (if present)  │
     │                              │ ────────────────────────────►│
     │                              │                              │ Execute flow
     │                              │                              │ with trigger data
     │                              │ ◄────────────────────────────│
     │ ◄─────────────────────────────                              │
     │  200 OK {result}             │                              │
```

---

## Supported Methods

| Method | Request Body | Use Case |
|--------|-------------|----------|
| **GET** | No | Retrieve data, trigger read-only flows |
| **POST** | Yes | Create resources, trigger data ingestion |
| **PUT** | Yes | Update resources, trigger data sync |
| **DELETE** | Optional | Delete resources, trigger cleanup |

---

## Flow Definition

Define flows with specific HTTP methods:

```json
{
  "id": "create-user-flow",
  "name": "Create User",
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "POST"         ← Specify method
  },
  "steps": [
    {
      "type": "log",
      "name": "log_request",
      "message": "Creating user: {{trigger.body.name}}"
    },
    {
      "type": "call",
      "name": "insert_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id",
        "params": ["{{trigger.body.name}}", "{{trigger.body.email}}"]
      }
    }
  ]
}
```

---

## Trigger Payload

The flow receives a `Message` with the following structure:

```json
{
  "trigger": "http",
  "path": "/users",
  "method": "POST",           ← HTTP method
  "body": {                   ← Request body (for POST/PUT/DELETE)
    "name": "Alice",
    "email": "alice@example.com"
  }
}
```

Access in flow steps:
- `{{trigger.method}}` → `"POST"`
- `{{trigger.path}}` → `"/users"`
- `{{trigger.body.name}}` → `"Alice"`
- `{{trigger.body.email}}` → `"alice@example.com"`

---

## Examples

### 1. GET — Retrieve Data

**Flow Definition:**
```json
{
  "id": "list-users",
  "name": "List Users",
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "name": "fetch_users",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT id, name, email FROM users ORDER BY created_at DESC LIMIT 50"
      }
    }
  ]
}
```

**Invoke:**
```bash
curl http://localhost:8080/api/trigger/users
```

**Response:**
```json
{
  "id": "...",
  "name": "...",
  "rows": [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"}
  ]
}
```

---

### 2. POST — Create Resource

**Flow Definition:**
```json
{
  "id": "create-user",
  "name": "Create User",
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "POST"
  },
  "steps": [
    {
      "type": "call",
      "name": "validate_email",
      "connector": "http",
      "operation": "post",
      "params": {
        "url": "https://api.emailvalidation.com/check",
        "body": {"email": "{{trigger.body.email}}"}
      }
    },
    {
      "type": "call",
      "name": "create_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, created_at",
        "params": ["{{trigger.body.name}}", "{{trigger.body.email}}"]
      }
    },
    {
      "type": "log",
      "name": "log_created",
      "message": "User created: {{create_user.result.id}}"
    }
  ]
}
```

**Invoke:**
```bash
curl -X POST http://localhost:8080/api/trigger/users \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Charlie",
    "email": "charlie@example.com"
  }'
```

**Response:**
```json
{
  "id": 3,
  "created_at": "2024-02-18T10:00:00Z"
}
```

---

### 3. PUT — Update Resource

**Flow Definition:**
```json
{
  "id": "update-user",
  "name": "Update User",
  "trigger": {
    "type": "http",
    "path": "/users/update",
    "method": "PUT"
  },
  "steps": [
    {
      "type": "call",
      "name": "update_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING *",
        "params": [
          "{{trigger.body.name}}",
          "{{trigger.body.email}}",
          "{{trigger.body.id}}"
        ]
      }
    },
    {
      "type": "call",
      "name": "notify_update",
      "connector": "http",
      "operation": "post",
      "params": {
        "url": "https://api.slack.com/webhooks/...",
        "body": {
          "text": "User {{trigger.body.id}} updated: {{trigger.body.name}}"
        }
      }
    }
  ]
}
```

**Invoke:**
```bash
curl -X PUT http://localhost:8080/api/trigger/users/update \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "name": "Alice Smith",
    "email": "alice.smith@example.com"
  }'
```

---

### 4. DELETE — Remove Resource

**Flow Definition:**
```json
{
  "id": "delete-user",
  "name": "Delete User",
  "trigger": {
    "type": "http",
    "path": "/users/delete",
    "method": "DELETE"
  },
  "steps": [
    {
      "type": "call",
      "name": "soft_delete",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "UPDATE users SET deleted_at = NOW() WHERE id = $1 RETURNING id",
        "params": ["{{trigger.body.id}}"]
      }
    },
    {
      "type": "log",
      "name": "log_deletion",
      "message": "User {{trigger.body.id}} deleted"
    }
  ]
}
```

**Invoke:**
```bash
curl -X DELETE http://localhost:8080/api/trigger/users/delete \
  -H "Content-Type: application/json" \
  -d '{"id": 99}'
```

---

## Path Matching

Flows are matched by **exact path AND method**:

| Registered Flow | Incoming Request | Match? |
|----------------|------------------|--------|
| `POST /users` | `POST /api/trigger/users` | ✅ Yes |
| `POST /users` | `GET /api/trigger/users` | ❌ No (method mismatch) |
| `GET /users` | `POST /api/trigger/users` | ❌ No (method mismatch) |
| `GET /orders` | `GET /api/trigger/users` | ❌ No (path mismatch) |

**Multiple methods on same path:**
You can register different flows for the same path with different methods:

```json
[
  {
    "id": "get-users",
    "trigger": {"type": "http", "path": "/users", "method": "GET"},
    "steps": [...]
  },
  {
    "id": "create-user",
    "trigger": {"type": "http", "path": "/users", "method": "POST"},
    "steps": [...]
  },
  {
    "id": "update-user",
    "trigger": {"type": "http", "path": "/users", "method": "PUT"},
    "steps": [...]
  },
  {
    "id": "delete-user",
    "trigger": {"type": "http", "path": "/users", "method": "DELETE"},
    "steps": [...]
  }
]
```

Now you have a full RESTful API on `/api/trigger/users`!

---

## Error Handling

### No Flow Found

**Request:**
```bash
curl -X POST http://localhost:8080/api/trigger/nonexistent
```

**Response:**
```json
{
  "error": "No flow registered for POST /nonexistent"
}
```
**Status:** `404 Not Found`

### Wrong Method

**Registered:** `GET /users`

**Request:**
```bash
curl -X POST http://localhost:8080/api/trigger/users
```

**Response:**
```json
{
  "error": "No flow registered for POST /users"
}
```
**Status:** `404 Not Found`

**Hint:** Register a separate flow for `POST /users` if needed.

---

## Authentication

All trigger endpoints require authentication (same as flow execution):

```bash
# Method 1: Client credentials
curl -X POST http://localhost:8080/api/trigger/users \
  -H "X-Client-Id: cid_..." \
  -H "X-Client-Secret: cs_..." \
  -d '{"name": "Alice"}'

# Method 2: JWT token
curl -X POST http://localhost:8080/api/trigger/users \
  -H "Authorization: Bearer eyJ..." \
  -d '{"name": "Alice"}'
```

See `AUTH.md` for authentication details.

---

## Complete REST API Example

**Setup:**
```bash
# 1. Register connector
curl -X POST http://localhost:8081/connector-instances -d '{
  "id": "postgres_prod",
  "connector_type": "postgres",
  "host": "postgres",
  "port": 5432,
  "database": "myapp",
  "username": "app_user",
  "password": "secret123"
}'

# 2. Register flows for each method
curl -X POST http://localhost:8081/flows -d @get-users.json
curl -X POST http://localhost:8081/flows -d @create-user.json
curl -X POST http://localhost:8081/flows -d @update-user.json
curl -X POST http://localhost:8081/flows -d @delete-user.json

# 3. Get auth token
TOKEN=$(curl -X POST http://localhost:8081/auth/token \
  -d '{"client_id":"...","client_secret":"..."}' | jq -r '.access_token')
```

**Use the API:**
```bash
# List users
curl http://localhost:8080/api/trigger/users \
  -H "Authorization: Bearer $TOKEN"

# Create user
curl -X POST http://localhost:8080/api/trigger/users \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name":"Alice","email":"alice@example.com"}'

# Update user
curl -X PUT http://localhost:8080/api/trigger/users/update \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"id":1,"name":"Alice Smith","email":"alice.smith@example.com"}'

# Delete user
curl -X DELETE http://localhost:8080/api/trigger/users/delete \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"id":1}'
```

---

## Testing

Create test flows:

```bash
# GET flow
curl -X POST http://localhost:8081/flows -d '{
  "id": "test-get",
  "name": "Test GET",
  "trigger": {"type": "http", "path": "/test", "method": "GET"},
  "steps": [
    {"type": "log", "name": "log", "message": "GET triggered"}
  ]
}'

# POST flow
curl -X POST http://localhost:8081/flows -d '{
  "id": "test-post",
  "name": "Test POST",
  "trigger": {"type": "http", "path": "/test", "method": "POST"},
  "steps": [
    {"type": "log", "name": "log", "message": "POST triggered with: {{trigger.body}}"}
  ]
}'

# Test
curl http://localhost:8080/api/trigger/test
curl -X POST http://localhost:8080/api/trigger/test -d '{"key":"value"}'
```

---

## Migration from GET-only

**Before (only GET supported):**
```json
{
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "GET"
  }
}
```

**After (all methods supported):**
- Existing GET flows work unchanged
- Add new flows for POST/PUT/DELETE on the same or different paths
- Each method gets its own flow definition

**No breaking changes** — existing GET flows continue to work.

---

## Summary

| Feature | Support |
|---------|---------|
| **GET** | ✅ Supported |
| **POST** | ✅ Supported |
| **PUT** | ✅ Supported |
| **DELETE** | ✅ Supported |
| **Request body** | ✅ Available in `{{trigger.body}}` |
| **Path matching** | ✅ Exact match required |
| **Method matching** | ✅ Exact match required |
| **Multiple methods per path** | ✅ Supported (different flows) |
| **Authentication** | ✅ Required (client-creds or JWT) |

**Build complete RESTful APIs with HTTP triggers!** 🌐🔄✅
