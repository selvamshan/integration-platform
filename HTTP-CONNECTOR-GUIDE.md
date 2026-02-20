# HTTP Connector — Complete Guide

HTTP connector with support for multiple authentication types, configurable headers, timeouts, and OAuth2.

---

## Features

✅ **5 Authentication Types:**
- None (no auth)
- Bearer token
- Basic auth (username/password)
- API Key (custom header)
- OAuth2 (client credentials with token caching)

✅ **5 HTTP Methods:** GET, POST, PUT, DELETE, PATCH

✅ **Connector Instance Support:** Parse `extra_attributes` from database

✅ **Flexible Configuration:**
- Base URL
- Default headers
- Timeouts
- Retry logic (configuration ready)

✅ **OAuth Token Caching:** Automatic token management

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Connector Instance                            │
│                   (connector_instances table)                    │
│                                                                  │
│  {                                                               │
│    "id": "salesforce_api",                                       │
│    "connector_type": "http",                                     │
│    "extra_attributes": {                                         │
│      "base_url": "https://api.salesforce.com",                   │
│      "auth": {                                                   │
│        "type": "oauth2",                                         │
│        "token_url": "https://login.salesforce.com/...",          │
│        "client_id": "...",                                       │
│        "client_secret": "..."                                    │
│      },                                                          │
│      "default_headers": {...},                                   │
│      "timeout_ms": 30000                                         │
│    }                                                             │
│  }                                                               │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         │ HttpConnector::from_config()
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      HttpConnector                               │
│                                                                  │
│  • Parses extra_attributes                                       │
│  • Builds HTTP client with timeout                              │
│  • Stores auth configuration                                    │
│  • Caches OAuth tokens                                           │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         │ Flow execution
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Request Execution                             │
│                                                                  │
│  1. Build URL (base_url + path)                                 │
│  2. Build headers (default + request)                           │
│  3. Apply authentication                                        │
│  4. Execute HTTP request                                        │
│  5. Handle response                                             │
└──────────────────────────────────────────────────────────────────┘
```

---

## Connector Instance Configuration

### 1. No Authentication

```json
{
  "id": "public_api",
  "name": "Public API",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://api.example.com",
    "timeout_ms": 30000,
    "default_headers": {
      "Content-Type": "application/json",
      "Accept": "application/json"
    }
  }
}
```

**Registration:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "public_api",
    "name": "Public API",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.example.com",
      "default_headers": {
        "Content-Type": "application/json"
      }
    }
  }'
```

---

### 2. Bearer Token Authentication

```json
{
  "id": "github_api",
  "name": "GitHub API",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://api.github.com",
    "auth": {
      "type": "bearer",
      "token": "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
    },
    "default_headers": {
      "Accept": "application/vnd.github.v3+json",
      "User-Agent": "Integration-Platform"
    }
  }
}
```

**Registration:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "github_api",
    "name": "GitHub API",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.github.com",
      "auth": {
        "type": "bearer",
        "token": "ghp_your_token_here"
      }
    }
  }'
```

---

### 3. Basic Authentication

```json
{
  "id": "internal_api",
  "name": "Internal API",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://internal.example.com/api",
    "auth": {
      "type": "basic",
      "username": "api_user",
      "password": "api_password"
    },
    "timeout_ms": 60000
  }
}
```

**Registration:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "internal_api",
    "name": "Internal API",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://internal.example.com/api",
      "auth": {
        "type": "basic",
        "username": "api_user",
        "password": "secret123"
      }
    }
  }'
```

---

### 4. API Key Authentication

```json
{
  "id": "stripe_api",
  "name": "Stripe API",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://api.stripe.com/v1",
    "auth": {
      "type": "apikey",
      "header_name": "Authorization",
      "api_key": "Bearer sk_test_xxxxxxxxxxxxxxxxxxxx"
    },
    "default_headers": {
      "Stripe-Version": "2023-10-16"
    }
  }
}
```

**Or with custom header:**
```json
{
  "id": "custom_api",
  "name": "Custom API",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://api.custom.com",
    "auth": {
      "type": "apikey",
      "header_name": "X-API-Key",
      "api_key": "your_api_key_here"
    }
  }
}
```

**Registration:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "stripe_api",
    "name": "Stripe API",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.stripe.com/v1",
      "auth": {
        "type": "apikey",
        "header_name": "Authorization",
        "api_key": "Bearer sk_test_..."
      }
    }
  }'
```

---

### 5. OAuth2 Authentication

```json
{
  "id": "salesforce_api",
  "name": "Salesforce API",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://na1.salesforce.com",
    "auth": {
      "type": "oauth2",
      "token_url": "https://login.salesforce.com/services/oauth2/token",
      "client_id": "3MVG9...",
      "client_secret": "1234567890...",
      "grant_type": "client_credentials",
      "scope": "api"
    },
    "default_headers": {
      "Content-Type": "application/json"
    },
    "timeout_ms": 30000
  }
}
```

**Registration:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "salesforce_oauth",
    "name": "Salesforce OAuth",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://login.salesforce.com",
      "auth": {
        "type": "oauth2",
        "token_url": "https://login.salesforce.com/services/oauth2/token",
        "client_id": "your_client_id",
        "client_secret": "your_client_secret"
      }
    }
  }'
```

---

## Flow Usage

### Example 1: Simple GET Request

**Register connector:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -d '{
    "id": "jsonplaceholder",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://jsonplaceholder.typicode.com"
    }
  }'
```

**Create flow:**
```json
{
  "id": "get-users",
  "name": "Get Users",
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "name": "fetch_users",
      "connector": "jsonplaceholder",
      "operation": "get",
      "params": {
        "path": "/users"
      }
    }
  ]
}
```

**Test:**
```bash
curl http://localhost:8080/api/trigger/users
```

---

### Example 2: POST with Bearer Auth

**Register connector:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -d '{
    "id": "github_api",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.github.com",
      "auth": {
        "type": "bearer",
        "token": "ghp_your_token"
      }
    }
  }'
```

**Create flow:**
```json
{
  "id": "create-issue",
  "name": "Create GitHub Issue",
  "trigger": {
    "type": "http",
    "path": "/create-issue",
    "method": "POST"
  },
  "steps": [
    {
      "type": "call",
      "name": "create_issue",
      "connector": "github_api",
      "operation": "post",
      "params": {
        "path": "/repos/owner/repo/issues",
        "body": {
          "title": "{{trigger.body.title}}",
          "body": "{{trigger.body.description}}"
        }
      }
    }
  ]
}
```

**Test:**
```bash
curl -X POST http://localhost:8080/api/trigger/create-issue \
  -H "Content-Type: application/json" \
  -d '{
    "title": "New issue",
    "description": "Issue description"
  }'
```

---

### Example 3: OAuth2 Flow with Token Caching

**Register connector:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -d '{
    "id": "salesforce_oauth",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://na1.salesforce.com",
      "auth": {
        "type": "oauth2",
        "token_url": "https://login.salesforce.com/services/oauth2/token",
        "client_id": "your_client_id",
        "client_secret": "your_client_secret"
      }
    }
  }'
```

**Create flow:**
```json
{
  "id": "sync-salesforce",
  "name": "Sync Salesforce Data",
  "trigger": {
    "type": "http",
    "path": "/sync-sf",
    "method": "POST"
  },
  "steps": [
    {
      "type": "call",
      "name": "get_accounts",
      "connector": "salesforce_oauth",
      "operation": "get",
      "params": {
        "path": "/services/data/v57.0/sobjects/Account"
      }
    },
    {
      "type": "call",
      "name": "save_to_db",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "INSERT INTO accounts (sf_id, name) VALUES ($1, $2)",
        "params": ["{{get_accounts.data.id}}", "{{get_accounts.data.name}}"]
      }
    }
  ]
}
```

**OAuth token automatically obtained and cached!**

---

### Example 4: Multi-Step API Integration

**Scenario:** Get OAuth token → Fetch data → Save to database

**Register connector:**
```bash
curl -X POST http://localhost:8081/connector-instances \
  -d '{
    "id": "protected_api",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.protected.com",
      "auth": {
        "type": "oauth2",
        "token_url": "https://auth.protected.com/token",
        "client_id": "client123",
        "client_secret": "secret456"
      }
    }
  }'
```

**Create flow:**
```json
{
  "id": "fetch-and-save",
  "name": "Fetch and Save Data",
  "trigger": {
    "type": "http",
    "path": "/fetch-data",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "name": "fetch_data",
      "connector": "protected_api",
      "operation": "get",
      "params": {
        "path": "/api/v1/data"
      }
    },
    {
      "type": "call",
      "name": "save_data",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "INSERT INTO api_data (data) VALUES ($1)",
        "params": ["{{fetch_data.data}}"]
      }
    },
    {
      "type": "log",
      "name": "log_success",
      "message": "Saved {{fetch_data.data.count}} records"
    }
  ]
}
```

---

## Operations

### GET Request

```json
{
  "type": "call",
  "connector": "my_api",
  "operation": "get",
  "params": {
    "path": "/users/123",
    "headers": {
      "Custom-Header": "value"
    }
  }
}
```

**Response:**
```json
{
  "status": 200,
  "data": {
    "id": 123,
    "name": "John Doe"
  }
}
```

---

### POST Request

```json
{
  "type": "call",
  "connector": "my_api",
  "operation": "post",
  "params": {
    "path": "/users",
    "body": {
      "name": "Jane Doe",
      "email": "jane@example.com"
    },
    "headers": {
      "Content-Type": "application/json"
    }
  }
}
```

---

### PUT Request

```json
{
  "type": "call",
  "connector": "my_api",
  "operation": "put",
  "params": {
    "path": "/users/123",
    "body": {
      "name": "Updated Name"
    }
  }
}
```

---

### DELETE Request

```json
{
  "type": "call",
  "connector": "my_api",
  "operation": "delete",
  "params": {
    "path": "/users/123"
  }
}
```

---

### PATCH Request

```json
{
  "type": "call",
  "connector": "my_api",
  "operation": "patch",
  "params": {
    "path": "/users/123",
    "body": {
      "email": "newemail@example.com"
    }
  }
}
```

---

### OAuth Token (Explicit)

Get OAuth token explicitly (usually automatic):

```json
{
  "type": "call",
  "connector": "salesforce_oauth",
  "operation": "oauth_token",
  "params": {}
}
```

**Response:**
```json
{
  "access_token": "00D...",
  "token_type": "Bearer"
}
```

---

## Configuration Reference

### HttpConnectorConfig

```json
{
  "base_url": "https://api.example.com",
  "auth": { ... },
  "default_headers": {
    "Header-Name": "value"
  },
  "timeout_ms": 30000,
  "retry": {
    "max_attempts": 3,
    "backoff_ms": 1000
  }
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `base_url` | string | No | null | Base URL prepended to all paths |
| `auth` | AuthConfig | No | null | Authentication configuration |
| `default_headers` | object | No | {} | Headers added to all requests |
| `timeout_ms` | number | No | 30000 | Request timeout in milliseconds |
| `retry` | RetryConfig | No | null | Retry configuration (future) |

---

### AuthConfig Types

#### None
```json
{
  "type": "none"
}
```

#### Bearer
```json
{
  "type": "bearer",
  "token": "your_bearer_token"
}
```

#### Basic
```json
{
  "type": "basic",
  "username": "user",
  "password": "pass"
}
```

#### API Key
```json
{
  "type": "apikey",
  "header_name": "X-API-Key",
  "api_key": "your_key"
}
```

#### OAuth2
```json
{
  "type": "oauth2",
  "token_url": "https://auth.example.com/token",
  "client_id": "client_id",
  "client_secret": "client_secret",
  "scope": "api read write",
  "grant_type": "client_credentials"
}
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `token_url` | Yes | - | OAuth token endpoint |
| `client_id` | Yes | - | Client ID |
| `client_secret` | Yes | - | Client secret |
| `scope` | No | null | OAuth scope |
| `grant_type` | No | "client_credentials" | Grant type |

---

## URL Building

### With base_url

```json
{
  "base_url": "https://api.example.com",
  "path": "/users/123"
}
```

**Result:** `https://api.example.com/users/123`

### Without base_url

```json
{
  "base_url": null,
  "path": "https://api.example.com/users/123"
}
```

**Result:** `https://api.example.com/users/123`

### Full URL in path (overrides base_url)

```json
{
  "base_url": "https://api.example.com",
  "path": "https://other-api.com/data"
}
```

**Result:** `https://other-api.com/data`

---

## Header Merging

**Connector config:**
```json
{
  "default_headers": {
    "Content-Type": "application/json",
    "X-App-ID": "123"
  }
}
```

**Request params:**
```json
{
  "headers": {
    "Authorization": "Bearer token",
    "X-App-ID": "456"
  }
}
```

**Final headers:**
```
Content-Type: application/json
X-App-ID: 456          ← Request overrides default
Authorization: Bearer token
```

---

## OAuth Token Caching

**First request:**
```
1. Check cache → empty
2. Request token from token_url
3. Cache token in memory
4. Use token for request
```

**Subsequent requests:**
```
1. Check cache → token found
2. Use cached token
3. No token request needed
```

**Cache cleared:**
- On connector disconnect
- On platform restart

**Future enhancement:** Persistent cache with expiry

---

## Error Handling

### Connection Error

```json
{
  "error": "HTTP GET failed: connection timeout"
}
```

### OAuth Error

```json
{
  "error": "OAuth token request failed with status 401: Invalid client credentials"
}
```

### Missing Parameters

```json
{
  "error": "Missing 'url' or 'path' parameter"
}
```

---

## Real-World Examples

### Example: Slack Webhook

```bash
curl -X POST http://localhost:8081/connector-instances \
  -d '{
    "id": "slack_webhook",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://hooks.slack.com",
      "default_headers": {
        "Content-Type": "application/json"
      }
    }
  }'
```

**Flow:**
```json
{
  "steps": [
    {
      "connector": "slack_webhook",
      "operation": "post",
      "params": {
        "path": "/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXX",
        "body": {
          "text": "Deployment completed successfully!"
        }
      }
    }
  ]
}
```

---

### Example: Stripe Payment

```bash
curl -X POST http://localhost:8081/connector-instances \
  -d '{
    "id": "stripe",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.stripe.com/v1",
      "auth": {
        "type": "bearer",
        "token": "sk_test_..."
      },
      "default_headers": {
        "Stripe-Version": "2023-10-16"
      }
    }
  }'
```

**Flow:**
```json
{
  "steps": [
    {
      "connector": "stripe",
      "operation": "post",
      "params": {
        "path": "/charges",
        "body": {
          "amount": 2000,
          "currency": "usd",
          "source": "tok_visa"
        }
      }
    }
  ]
}
```

---

### Example: GitHub API with Bearer

```bash
curl -X POST http://localhost:8081/connector-instances \
  -d '{
    "id": "github",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.github.com",
      "auth": {
        "type": "bearer",
        "token": "ghp_..."
      },
      "default_headers": {
        "Accept": "application/vnd.github.v3+json"
      }
    }
  }'
```

---

## Testing

### Test Connector Registration

```bash
#!/bin/bash

# Test 1: Public API (no auth)
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test_public",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://jsonplaceholder.typicode.com"
    }
  }'

# Test 2: Bearer auth
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test_bearer",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.example.com",
      "auth": {
        "type": "bearer",
        "token": "test_token"
      }
    }
  }'

# Test 3: OAuth2
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test_oauth",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.oauth.com",
      "auth": {
        "type": "oauth2",
        "token_url": "https://auth.example.com/token",
        "client_id": "test_client",
        "client_secret": "test_secret"
      }
    }
  }'
```

---

## Comparison with Database Connectors

| Feature | Database Connector | HTTP Connector |
|---------|-------------------|----------------|
| **Instance storage** | connector_instances table | connector_instances table |
| **Type field** | `connector_type: "postgres"` | `connector_type: "http"` |
| **Standard fields** | host, port, username, password | Not used (NULL) |
| **Config location** | extra_attributes | extra_attributes |
| **Auth types** | Password | Bearer, Basic, API Key, OAuth2 |
| **Connection** | TCP connection | HTTP client |
| **Operations** | query, execute | get, post, put, delete, patch |

**Same table, different configs — unified architecture!** ✅

---

## Future Enhancements

🔨 **In Progress:**
- Retry logic (config ready)
- Request/response transformations
- Rate limiting per connector

📝 **Planned:**
- Persistent OAuth token storage
- Token refresh for OAuth2
- Certificate-based auth (mTLS)
- SOAP connector (XML)
- GraphQL support
- WebSocket support

---

## Summary

✅ **Unified connector architecture** — Same table as database connectors  
✅ **5 authentication types** — None, Bearer, Basic, API Key, OAuth2  
✅ **5 HTTP methods** — GET, POST, PUT, DELETE, PATCH  
✅ **Flexible configuration** — base_url, headers, timeout, retry  
✅ **OAuth token caching** — Automatic token management  
✅ **Production ready** — Follows Airflow pattern  

**Your platform now supports HTTP connectors with enterprise auth!** 🌐🔐✅
