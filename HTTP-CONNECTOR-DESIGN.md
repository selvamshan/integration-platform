# HTTP Connector Design — Instance Strategy

## The Question

Should HTTP connectors use:
1. **Single `connector_instances` table** for all connector types (DB + HTTP)?
2. **Separate storage** for HTTP connector configurations?

---

## Current State: Database Connectors

**Example: PostgreSQL connector instance**

```json
{
  "id": "postgres_prod",
  "connector_type": "postgres",
  "host": "prod-db.example.com",
  "port": 5432,
  "database": "myapp",
  "username": "app_user",
  "password": "secret123",
  "extra_attributes": {}
}
```

**Connection URL built dynamically:**
```
postgresql://app_user:secret123@prod-db.example.com:5432/myapp
```

Used in flow:
```json
{
  "steps": [
    {
      "type": "call",
      "connector": "postgres_prod",  // ← References instance
      "operation": "query",
      "params": {"sql": "SELECT * FROM users"}
    }
  ]
}
```

---

## HTTP Connector Use Cases

### Use Case 1: OAuth Token Flow

```
1. POST /oauth/token (get access token)
2. GET /api/users (use token in header)
```

### Use Case 2: Multi-Step API Integration

```
1. POST /api/sessions (create session)
2. GET /api/data?sessionId=... (fetch data)
3. DELETE /api/sessions/... (cleanup)
```

### Use Case 3: External SaaS APIs

```
1. Salesforce API (custom domain, API keys)
2. Stripe API (different auth, rate limits)
3. Slack API (webhook URLs)
```

---

## Approach 1: Unified connector_instances Table ✅ RECOMMENDED

**Store HTTP configs alongside DB configs:**

```sql
CREATE TABLE connector_instances (
    id                  VARCHAR(64)  PRIMARY KEY,
    name                VARCHAR(255) NOT NULL,
    connector_type      VARCHAR(50)  NOT NULL,  -- 'postgres', 'http', 'mysql', etc.
    
    -- Database-specific (NULL for HTTP)
    host                VARCHAR(255),
    port                INTEGER,
    database_name       VARCHAR(255),
    username            VARCHAR(255),
    password_encrypted  TEXT,
    
    -- HTTP & generic attributes (JSON)
    extra_attributes    JSONB NOT NULL DEFAULT '{}',
    
    active              BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**HTTP connector instance examples:**

**A. OAuth Provider:**
```json
{
  "id": "salesforce_oauth",
  "name": "Salesforce OAuth",
  "connector_type": "http",
  "host": null,
  "port": null,
  "extra_attributes": {
    "base_url": "https://login.salesforce.com",
    "auth_type": "oauth2",
    "token_endpoint": "/services/oauth2/token",
    "client_id": "3MVG9...",
    "client_secret": "encrypted_secret",
    "default_headers": {
      "Content-Type": "application/x-www-form-urlencoded"
    }
  }
}
```

**B. REST API with API Key:**
```json
{
  "id": "stripe_api",
  "name": "Stripe API",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://api.stripe.com/v1",
    "auth_type": "bearer",
    "api_key": "sk_test_encrypted...",
    "default_headers": {
      "Stripe-Version": "2023-10-16"
    },
    "rate_limit": {
      "requests_per_second": 100
    }
  }
}
```

**C. Slack Webhook:**
```json
{
  "id": "slack_notifications",
  "name": "Slack Notifications",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://hooks.slack.com",
    "webhook_path": "/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXX",
    "auth_type": "none",
    "default_headers": {
      "Content-Type": "application/json"
    }
  }
}
```

**Flow usage:**
```json
{
  "steps": [
    {
      "type": "call",
      "connector": "salesforce_oauth",
      "operation": "post",
      "params": {
        "path": "/services/oauth2/token",
        "body": {
          "grant_type": "password",
          "username": "user@example.com",
          "password": "pass123"
        }
      }
    },
    {
      "type": "call",
      "connector": "salesforce_oauth",
      "operation": "get",
      "params": {
        "path": "/services/data/v57.0/sobjects/Account",
        "headers": {
          "Authorization": "Bearer {{step1.access_token}}"
        }
      }
    }
  ]
}
```

### Pros ✅

1. **Single source of truth** — All connectors in one place
2. **Consistent API** — Same CRUD endpoints for all connector types
3. **Unified sync** — One NATS event system for all connectors
4. **Simpler frontend** — Single dropdown, single management UI
5. **Database simplicity** — One table, one migration, one backup
6. **Reusable patterns** — Same encryption, same credentials management
7. **Cross-connector flows** — Easy to reference both DB and HTTP in same flow

### Cons ⚠️

1. Schema has nullable fields (host, port for HTTP; not used)
2. HTTP configs in JSON (less type-safe than columns)
3. More generic code (type checking at runtime)

---

## Approach 2: Separate HTTP Connector Storage

**New table:**
```sql
CREATE TABLE http_connector_instances (
    id                  VARCHAR(64)  PRIMARY KEY,
    name                VARCHAR(255) NOT NULL,
    base_url            VARCHAR(500) NOT NULL,
    auth_type           VARCHAR(50)  NOT NULL,  -- 'none', 'bearer', 'oauth2', 'basic'
    api_key_encrypted   TEXT,
    oauth_config        JSONB,
    default_headers     JSONB NOT NULL DEFAULT '{}',
    rate_limit_config   JSONB,
    active              BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Example:**
```json
{
  "id": "salesforce_api",
  "name": "Salesforce API",
  "base_url": "https://login.salesforce.com",
  "auth_type": "oauth2",
  "oauth_config": {
    "token_endpoint": "/services/oauth2/token",
    "client_id": "...",
    "client_secret_encrypted": "..."
  },
  "default_headers": {...}
}
```

**Flow usage:**
```json
{
  "steps": [
    {
      "type": "call",
      "connector": "salesforce_api",
      "connector_type": "http",  // ← Need to specify type
      "operation": "post",
      "params": {...}
    }
  ]
}
```

### Pros ✅

1. Type-safe schema for HTTP attributes
2. Better data validation (not in JSON blob)
3. Easier to query/filter HTTP connectors specifically

### Cons ⚠️

1. **Duplicate infrastructure** — Separate CRUD endpoints
2. **Separate sync service** — More NATS events to manage
3. **Frontend duplication** — Two management UIs
4. **Migration complexity** — Two tables to migrate
5. **Cross-connector complexity** — Flows with both DB + HTTP harder to manage
6. **More code** — Duplicate registration, encryption, sync logic

---

## Recommendation: Unified Table (Approach 1) ✅

**Use the existing `connector_instances` table for HTTP connectors.**

### Why?

1. **Already built** — Infrastructure exists (CRUD, sync, encryption)
2. **Consistent UX** — One place to manage all connectors
3. **Flexible** — `extra_attributes` JSONB can hold any HTTP config
4. **Simpler** — Less code, less maintenance
5. **Proven pattern** — Works for multiple DB types, will work for HTTP

### Implementation

**HTTP connector instance structure:**
```json
{
  "id": "stripe_api",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://api.stripe.com/v1",
    "auth": {
      "type": "bearer",
      "token": "encrypted_sk_test_..."
    },
    "default_headers": {
      "Stripe-Version": "2023-10-16"
    },
    "timeout_ms": 30000,
    "retry": {
      "max_attempts": 3,
      "backoff_ms": 1000
    }
  }
}
```

**Enhanced HTTP connector in code:**
```rust
pub struct HttpConnector {
    client: reqwest::Client,
    base_url: Option<String>,
    default_headers: HashMap<String, String>,
    auth_config: Option<AuthConfig>,
}

pub enum AuthConfig {
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
    OAuth2 { 
        token_url: String,
        client_id: String,
        client_secret: String,
        cached_token: Option<String>,
    },
    ApiKey { 
        header_name: String,  // e.g., "X-API-Key"
        api_key: String,
    },
}
```

---

## Migration Path

### Phase 1: Current State (Database Only)
```
connector_instances:
  - postgres_dev
  - postgres_prod
  - mysql_analytics
```

### Phase 2: Add HTTP Support (No Breaking Changes)
```
connector_instances:
  - postgres_dev
  - postgres_prod
  - mysql_analytics
  - salesforce_api   ← NEW (connector_type: "http")
  - stripe_api       ← NEW
  - slack_webhook    ← NEW
```

**Same table, same API, same sync!**

### Phase 3: Enhanced HTTP Connector
Update `HttpConnector` to:
1. Accept `ConnectorInstance` with `extra_attributes`
2. Parse auth config
3. Apply default headers
4. Handle OAuth token refresh

---

## Example: OAuth Flow with Unified Table

**1. Register OAuth connector:**
```bash
curl -X POST http://localhost:8081/connector-instances -d '{
  "id": "salesforce_oauth",
  "name": "Salesforce OAuth",
  "connector_type": "http",
  "extra_attributes": {
    "base_url": "https://login.salesforce.com",
    "auth": {
      "type": "oauth2",
      "token_endpoint": "/services/oauth2/token",
      "client_id": "3MVG9...",
      "client_secret": "encrypted_...",
      "grant_type": "password"
    }
  }
}'
```

**2. Create flow:**
```json
{
  "id": "salesforce-sync",
  "trigger": {"type": "http", "path": "/sync-salesforce", "method": "POST"},
  "steps": [
    {
      "type": "call",
      "name": "get_token",
      "connector": "salesforce_oauth",
      "operation": "oauth_token",
      "params": {
        "username": "{{trigger.body.username}}",
        "password": "{{trigger.body.password}}"
      }
    },
    {
      "type": "call",
      "name": "get_accounts",
      "connector": "salesforce_oauth",
      "operation": "get",
      "params": {
        "path": "/services/data/v57.0/sobjects/Account",
        "headers": {
          "Authorization": "Bearer {{get_token.access_token}}"
        }
      }
    },
    {
      "type": "call",
      "name": "save_to_db",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "INSERT INTO accounts (...) VALUES (...)",
        "params": ["{{get_accounts.records}}"]
      }
    }
  ]
}
```

**Single flow uses both HTTP and DB connectors seamlessly!**

---

## Comparison Table

| Aspect | Unified Table | Separate Table |
|--------|--------------|----------------|
| **Infrastructure** | Reuse existing | Build from scratch |
| **Code complexity** | Low (extend existing) | High (duplicate) |
| **Type safety** | Medium (JSON) | High (columns) |
| **Flexibility** | High (any JSON) | Medium (fixed schema) |
| **Migration effort** | None | Significant |
| **Frontend work** | None | Duplicate UI |
| **Maintenance** | Single codebase | Two codebases |
| **Cross-connector flows** | Easy | Complex |
| **Recommendation** | ✅ **Use this** | ❌ Avoid |

---

## Next Steps

1. **Extend `extra_attributes` schema** for HTTP connectors
2. **Update `HttpConnector`** to parse connector instances
3. **Add auth types** (bearer, oauth2, api-key, basic)
4. **Keep existing API** — no breaking changes
5. **Document HTTP patterns** — show examples

**No schema changes needed!** Just extend the code to handle `connector_type: "http"`.

---

## Summary

✅ **Recommendation: Use unified `connector_instances` table**

**Benefits:**
- Reuse existing infrastructure (CRUD, sync, encryption)
- Single API, single UI, single source of truth
- Works for database + HTTP + future connector types
- Cross-connector flows are seamless
- Less code, less maintenance

**Implementation:**
- HTTP configs go in `extra_attributes` (JSONB)
- Enhanced `HttpConnector` parses instance config
- Same registration flow as database connectors

**Your platform will support both DB and HTTP connectors with a unified, elegant architecture!** 🔄🔌✅
