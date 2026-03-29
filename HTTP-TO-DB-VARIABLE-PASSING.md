# Passing HTTP Request Parameters to Database Connectors

Complete guide on using `{{variable}}` syntax to pass HTTP trigger data to database queries.

---

## Quick Answer

Use `{{trigger.path_params.X}}`, `{{trigger.query_params.X}}`, or `{{trigger.body.X}}` in your SQL params:

```json
{
  "trigger": {
    "type": "http",
    "path": "/users/:userId",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "connector": "postgres",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE id = $1",
        "params": ["{{trigger.path_params.userId}}"]
      }
    }
  ]
}
```

---

## Variable Sources

### 1. Path Parameters (`:param` in URL)

**Flow:**
```json
{
  "trigger": {
    "type": "http",
    "path": "/users/:userId",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "connector": "postgres",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE id = $1",
        "params": ["{{trigger.path_params.userId}}"]
      }
    }
  ]
}
```

**HTTP Request:**
```
GET /users/123
```

**Result:**
```sql
SELECT * FROM users WHERE id = '123'
```

---

### 2. Query Parameters (`?param=value`)

**Flow:**
```json
{
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "connector": "postgres",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE status = $1 AND age >= $2",
        "params": [
          "{{trigger.query_params.status}}",
          "{{trigger.query_params.min_age}}"
        ]
      }
    }
  ]
}
```

**HTTP Request:**
```
GET /users?status=active&min_age=18
```

**Result:**
```sql
SELECT * FROM users WHERE status = 'active' AND age >= 18
```

---

### 3. Request Body (POST/PUT)

**Flow:**
```json
{
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "POST"
  },
  "steps": [
    {
      "type": "call",
      "connector": "postgres",
      "operation": "execute",
      "params": {
        "sql": "INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING *",
        "params": [
          "{{trigger.body.name}}",
          "{{trigger.body.email}}",
          "{{trigger.body.age}}"
        ]
      }
    }
  ]
}
```

**HTTP Request:**
```json
POST /users
Content-Type: application/json

{
  "name": "John Doe",
  "email": "john@example.com",
  "age": 30
}
```

**Result:**
```sql
INSERT INTO users (name, email, age) VALUES ('John Doe', 'john@example.com', 30) RETURNING *
```

---

### 4. HTTP Headers

**Flow:**
```json
{
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "connector": "postgres",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE tenant_id = $1",
        "params": ["{{trigger.headers.x-tenant-id}}"]
      }
    }
  ]
}
```

**HTTP Request:**
```
GET /users
X-Tenant-ID: tenant-123
```

**Result:**
```sql
SELECT * FROM users WHERE tenant_id = 'tenant-123'
```

---

## Complete CRUD Examples

### Example 1: Get User by ID

```json
{
  "id": "get-user",
  "name": "Get User by ID",
  "trigger": {
    "type": "http",
    "path": "/api/users/:id",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "name": "fetch_user",
      "connector": "postgres",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE id = $1",
        "params": ["{{trigger.path_params.id}}"]
      }
    }
  ]
}
```

**Usage:**
```bash
curl http://localhost:8080/api/users/123
```

---

### Example 2: Search Users (Multiple Filters)

```json
{
  "id": "search-users",
  "name": "Search Users",
  "trigger": {
    "type": "http",
    "path": "/api/users/search",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "name": "search_users",
      "connector": "postgres",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE (name ILIKE $1 OR email ILIKE $1) AND status = $2 ORDER BY created_at DESC LIMIT $3",
        "params": [
          "%{{trigger.query_params.q}}%",
          "{{trigger.query_params.status}}",
          "{{trigger.query_params.limit || 10}}"
        ]
      }
    }
  ]
}
```

**Usage:**
```bash
# Search for users with name/email containing "john", status "active", limit 20
curl "http://localhost:8080/api/users/search?q=john&status=active&limit=20"
```

---

### Example 3: Create User

```json
{
  "id": "create-user",
  "name": "Create User",
  "trigger": {
    "type": "http",
    "path": "/api/users",
    "method": "POST"
  },
  "steps": [
    {
      "type": "call",
      "name": "insert_user",
      "connector": "postgres",
      "operation": "execute",
      "params": {
        "sql": "INSERT INTO users (name, email, age, status, created_at) VALUES ($1, $2, $3, $4, NOW()) RETURNING *",
        "params": [
          "{{trigger.body.name}}",
          "{{trigger.body.email}}",
          "{{trigger.body.age}}",
          "{{trigger.body.status || 'active'}}"
        ]
      }
    }
  ]
}
```

**Usage:**
```bash
curl -X POST http://localhost:8080/api/users \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Jane Doe",
    "email": "jane@example.com",
    "age": 28
  }'
```

---

### Example 4: Update User

```json
{
  "id": "update-user",
  "name": "Update User",
  "trigger": {
    "type": "http",
    "path": "/api/users/:id",
    "method": "PUT"
  },
  "steps": [
    {
      "type": "call",
      "name": "update_user",
      "connector": "postgres",
      "operation": "execute",
      "params": {
        "sql": "UPDATE users SET name = $1, email = $2, updated_at = NOW() WHERE id = $3 RETURNING *",
        "params": [
          "{{trigger.body.name}}",
          "{{trigger.body.email}}",
          "{{trigger.path_params.id}}"
        ]
      }
    }
  ]
}
```

**Usage:**
```bash
curl -X PUT http://localhost:8080/api/users/123 \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Jane Smith",
    "email": "jane.smith@example.com"
  }'
```

---

### Example 5: Delete User

```json
{
  "id": "delete-user",
  "name": "Delete User",
  "trigger": {
    "type": "http",
    "path": "/api/users/:id",
    "method": "DELETE"
  },
  "steps": [
    {
      "type": "call",
      "name": "delete_user",
      "connector": "postgres",
      "operation": "execute",
      "params": {
        "sql": "DELETE FROM users WHERE id = $1 RETURNING id",
        "params": ["{{trigger.path_params.id}}"]
      }
    }
  ]
}
```

**Usage:**
```bash
curl -X DELETE http://localhost:8080/api/users/123
```

---

## Advanced Patterns

### Pattern 1: Multiple Path Parameters

```json
{
  "trigger": {
    "type": "http",
    "path": "/users/:userId/posts/:postId",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "connector": "postgres",
      "params": {
        "sql": "SELECT * FROM posts WHERE user_id = $1 AND id = $2",
        "params": [
          "{{trigger.path_params.userId}}",
          "{{trigger.path_params.postId}}"
        ]
      }
    }
  ]
}
```

**Request:**
```
GET /users/123/posts/456
```

---

### Pattern 2: Nested Body Objects

```json
{
  "trigger": {
    "type": "http",
    "path": "/orders",
    "method": "POST"
  },
  "steps": [
    {
      "type": "call",
      "connector": "postgres",
      "params": {
        "sql": "INSERT INTO orders (user_id, product_id, quantity, shipping_address) VALUES ($1, $2, $3, $4)",
        "params": [
          "{{trigger.body.user.id}}",
          "{{trigger.body.product.id}}",
          "{{trigger.body.quantity}}",
          "{{trigger.body.shipping.address}}"
        ]
      }
    }
  ]
}
```

**Request Body:**
```json
{
  "user": { "id": 123 },
  "product": { "id": 456 },
  "quantity": 2,
  "shipping": {
    "address": "123 Main St",
    "city": "New York"
  }
}
```

---

### Pattern 3: Default Values

```json
{
  "params": {
    "sql": "SELECT * FROM users WHERE status = $1 LIMIT $2",
    "params": [
      "{{trigger.query_params.status || 'active'}}",
      "{{trigger.query_params.limit || 10}}"
    ]
  }
}
```

If parameters are missing, defaults are used:
- No `status` → defaults to `'active'`
- No `limit` → defaults to `10`

---

### Pattern 4: Array Handling

```json
{
  "trigger": {
    "type": "http",
    "path": "/users/bulk",
    "method": "POST"
  },
  "steps": [
    {
      "type": "loop",
      "loop_mode": "foreach",
      "iterate_over": "{{trigger.body.users}}",
      "steps": [
        {
          "type": "call",
          "connector": "postgres",
          "params": {
            "sql": "INSERT INTO users (name, email) VALUES ($1, $2)",
            "params": [
              "{{item.name}}",
              "{{item.email}}"
            ]
          }
        }
      ]
    }
  ]
}
```

**Request:**
```json
{
  "users": [
    {"name": "John", "email": "john@example.com"},
    {"name": "Jane", "email": "jane@example.com"}
  ]
}
```

---

## Variable Context Reference

### Complete Trigger Context

```javascript
{
  trigger: {
    type: "http",
    method: "GET",
    path: "/users/:id",
    
    // Path parameters from :param in URL
    path_params: {
      id: "123"
    },
    
    // Query string parameters
    query_params: {
      status: "active",
      limit: "50",
      sort: "name"
    },
    
    // HTTP headers (lowercase keys)
    headers: {
      "content-type": "application/json",
      "authorization": "Bearer xxx",
      "x-tenant-id": "tenant-123"
    },
    
    // Request body (POST/PUT)
    body: {
      name: "John Doe",
      email: "john@example.com",
      metadata: {
        age: 30,
        city: "New York"
      }
    }
  }
}
```

### Accessing Variables

```
{{trigger.path_params.id}}              → "123"
{{trigger.query_params.status}}         → "active"
{{trigger.headers.x-tenant-id}}         → "tenant-123"
{{trigger.body.name}}                   → "John Doe"
{{trigger.body.metadata.age}}           → 30
```

---

## Security: SQL Injection Prevention

### ✅ CORRECT - Use Parameterized Queries

```json
{
  "sql": "SELECT * FROM users WHERE id = $1",
  "params": ["{{trigger.path_params.id}}"]
}
```

The database driver safely escapes the parameter.

### ❌ WRONG - Direct String Interpolation

```json
{
  "sql": "SELECT * FROM users WHERE id = {{trigger.path_params.id}}"
}
```

**This is vulnerable to SQL injection!**

---

## Testing Your Flow

Use the test endpoint to verify variable passing:

```bash
curl -X POST http://localhost:8081/flows/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "flow": {
      "trigger": {
        "type": "http",
        "path": "/users/:id",
        "method": "GET"
      },
      "steps": [
        {
          "type": "call",
          "connector": "postgres",
          "operation": "query",
          "params": {
            "sql": "SELECT * FROM users WHERE id = $1",
            "params": ["{{trigger.path_params.id}}"]
          }
        }
      ]
    },
    "test_input": {
      "path_params": {"id": "123"},
      "query_params": {},
      "body": null
    }
  }'
```

---

## Common Mistakes & Solutions

### Mistake 1: Wrong Variable Path

```json
❌ "params": ["{{id}}"]                    // Missing trigger prefix
✅ "params": ["{{trigger.path_params.id}}"] // Correct
```

### Mistake 2: Direct SQL Interpolation

```json
❌ "sql": "... WHERE id = {{trigger.path_params.id}}"  // SQL injection risk!
✅ "sql": "... WHERE id = $1",                          // Safe
   "params": ["{{trigger.path_params.id}}"]
```

### Mistake 3: Accessing Non-existent Variables

```json
// If query param doesn't exist, use default
✅ "{{trigger.query_params.limit || 10}}"
```

---

## Summary

### Variable Sources
- `{{trigger.path_params.X}}` — URL path parameters (`:param`)
- `{{trigger.query_params.X}}` — Query string (`?param=value`)
- `{{trigger.body.X}}` — Request body (POST/PUT)
- `{{trigger.headers.X}}` — HTTP headers

### Best Practices
✅ Always use parameterized queries (`$1`, `$2`, etc.)  
✅ Use `||` for default values  
✅ Test with `/flows/test` endpoint  
✅ Never interpolate directly into SQL strings  

**Your HTTP triggers can now safely pass parameters to database queries!** 🔄✅
