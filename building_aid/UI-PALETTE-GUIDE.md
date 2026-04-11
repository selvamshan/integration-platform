# Flow Designer UI Support - Connector/Trigger Palette & Auto-API

## Overview

The Control Plane now maintains a **registry of connectors and triggers** in the database, enabling frontend flow designer UIs to display a palette of available components. Additionally, **API definitions are automatically created and updated** when flows with HTTP triggers are created, modified, or deleted.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Frontend Flow Designer UI                  │
│  ┌───────────────────────────────────────────────────┐ │
│  │  Palette (loaded from Control Plane)              │ │
│  │  ┌──────────────┐  ┌──────────────┐              │ │
│  │  │  Triggers    │  │  Connectors  │              │ │
│  │  │  🌐 HTTP     │  │  🌐 HTTP     │              │ │
│  │  │  ⏰ Schedule │  │  🐘 PostgreSQL│              │ │
│  │  └──────────────┘  └──────────────┘              │ │
│  └───────────────────────────────────────────────────┘ │
└────────────────────┬────────────────────────────────────┘
                     │ GET /triggers, /connectors
                     │ POST /flows
                     ▼
┌─────────────────────────────────────────────────────────┐
│                Control Plane (Port 8081)                 │
│  ┌───────────────────────────────────────────────────┐ │
│  │  Connector Registry (PostgreSQL)                  │ │
│  │  - HTTP Connector (GET, POST operations)         │ │
│  │  - PostgreSQL Connector (query, execute ops)     │ │
│  └───────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────┐ │
│  │  Trigger Registry (PostgreSQL)                    │ │
│  │  - HTTP Trigger (path, method config)            │ │
│  │  - Schedule Trigger (cron config)                │ │
│  └───────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────┐ │
│  │  Auto-API Management                              │ │
│  │  - Creates API definition on flow creation       │ │
│  │  - Updates endpoints on flow modification        │ │
│  │  - Removes endpoints on flow deletion            │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## Features

### 1. Connector Registry (for UI Palette)

**Persisted in Database:** `connector_definitions` table

**API Endpoints:**
- `GET /connectors` - List all available connectors
- `GET /connectors/:id` - Get specific connector details

**Built-in Connectors:**

#### HTTP/REST Connector
```json
{
  "id": "http-connector",
  "name": "HTTP/REST",
  "connector_type": "http",
  "description": "Make HTTP GET/POST requests to external APIs",
  "icon": "🌐",
  "operations": [
    {
      "name": "get",
      "description": "Make HTTP GET request",
      "parameters": [
        {
          "name": "url",
          "param_type": "string",
          "required": true,
          "description": "Target URL"
        }
      ]
    },
    {
      "name": "post",
      "description": "Make HTTP POST request",
      "parameters": [
        {
          "name": "url",
          "param_type": "string",
          "required": true,
          "description": "Target URL"
        },
        {
          "name": "body",
          "param_type": "object",
          "required": false,
          "description": "Request body (JSON)",
          "default_value": {}
        }
      ]
    }
  ],
  "enabled": true
}
```

#### PostgreSQL Connector
```json
{
  "id": "postgres-connector",
  "name": "PostgreSQL",
  "connector_type": "postgres",
  "description": "Execute SQL queries on PostgreSQL database",
  "icon": "🐘",
  "operations": [
    {
      "name": "query",
      "description": "Execute SELECT query",
      "parameters": [
        {
          "name": "sql",
          "param_type": "string",
          "required": true,
          "description": "SQL SELECT statement"
        }
      ]
    },
    {
      "name": "execute",
      "description": "Execute INSERT/UPDATE/DELETE",
      "parameters": [
        {
          "name": "sql",
          "param_type": "string",
          "required": true,
          "description": "SQL statement"
        }
      ]
    }
  ],
  "enabled": true
}
```

### 2. Trigger Registry (for UI Palette)

**Persisted in Database:** `trigger_definitions` table

**API Endpoints:**
- `GET /triggers` - List all available triggers
- `GET /triggers/:id` - Get specific trigger details

**Built-in Triggers:**

#### HTTP Trigger
```json
{
  "id": "http-trigger",
  "name": "HTTP Request",
  "trigger_type": "http",
  "description": "Trigger flow on HTTP GET/POST request",
  "icon": "🌐",
  "config_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "URL path"
      },
      "method": {
        "type": "string",
        "enum": ["GET", "POST", "PUT", "DELETE"]
      }
    },
    "required": ["path", "method"]
  },
  "enabled": true
}
```

#### Schedule Trigger
```json
{
  "id": "schedule-trigger",
  "name": "Schedule",
  "trigger_type": "schedule",
  "description": "Trigger flow on schedule (cron)",
  "icon": "⏰",
  "config_schema": {
    "type": "object",
    "properties": {
      "cron": {
        "type": "string",
        "description": "Cron expression"
      }
    },
    "required": ["cron"]
  },
  "enabled": true
}
```

### 3. Automatic API Definition Management

When you create/update/delete flows with HTTP triggers, the Control Plane automatically manages API definitions.

#### Auto-Create API on Flow Creation

**Example:**
```bash
# Create flow with HTTP trigger
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user-lookup",
    "name": "User Lookup Flow",
    "trigger": {
      "type": "http",
      "path": "/api/users",
      "method": "GET"
    },
    "steps": [...]
  }'
```

**What happens automatically:**
1. ✅ Flow saved to `flow_definitions` table
2. ✅ API definition created/updated in `api_definitions` table
3. ✅ Endpoint added: `GET /api/users → user-lookup flow`
4. ✅ Events published to NATS
5. ✅ Data Plane receives and registers flow

**Auto-Generated API:**
```json
{
  "id": "auto-generated-uuid",
  "name": "Auto-Generated API",
  "version": "1.0",
  "base_path": "/api",
  "endpoints": [
    {
      "path": "/api/users",
      "method": "GET",
      "flow_id": "user-lookup"
    }
  ]
}
```

#### Auto-Update API on Flow Modification

```bash
# Update flow with different HTTP path
curl -X PUT http://localhost:8081/flows/user-lookup \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user-lookup",
    "name": "User Lookup Flow V2",
    "trigger": {
      "type": "http",
      "path": "/api/v2/users",  # Changed path
      "method": "GET"
    },
    "steps": [...]
  }'
```

**What happens:**
1. ✅ Flow updated in database
2. ✅ API endpoint automatically updated: `GET /api/v2/users → user-lookup`
3. ✅ Old endpoint removed if path changed
4. ✅ Update event published

#### Auto-Remove Endpoint on Flow Deletion

```bash
# Delete flow
curl -X DELETE http://localhost:8081/flows/user-lookup
```

**What happens:**
1. ✅ Flow deleted from database
2. ✅ Endpoint automatically removed from API definition
3. ✅ Deletion event published

## API Reference

### Connector Registry APIs

#### List All Connectors
```bash
GET /connectors

Response:
{
  "connectors": [
    {
      "id": "http-connector",
      "name": "HTTP/REST",
      "connector_type": "http",
      "description": "...",
      "operations": [...]
    },
    {
      "id": "postgres-connector",
      "name": "PostgreSQL",
      ...
    }
  ],
  "count": 2
}
```

#### Get Connector Details
```bash
GET /connectors/http-connector

Response:
{
  "id": "http-connector",
  "name": "HTTP/REST",
  "connector_type": "http",
  "description": "Make HTTP GET/POST requests to external APIs",
  "icon": "🌐",
  "operations": [
    {
      "name": "get",
      "description": "Make HTTP GET request",
      "parameters": [
        {
          "name": "url",
          "param_type": "string",
          "required": true,
          "description": "Target URL"
        }
      ]
    }
  ],
  "config_schema": {...},
  "enabled": true
}
```

### Trigger Registry APIs

#### List All Triggers
```bash
GET /triggers

Response:
{
  "triggers": [
    {
      "id": "http-trigger",
      "name": "HTTP Request",
      "trigger_type": "http",
      "description": "...",
      "config_schema": {...}
    },
    {
      "id": "schedule-trigger",
      "name": "Schedule",
      ...
    }
  ],
  "count": 2
}
```

#### Get Trigger Details
```bash
GET /triggers/http-trigger

Response:
{
  "id": "http-trigger",
  "name": "HTTP Request",
  "trigger_type": "http",
  "description": "Trigger flow on HTTP GET/POST request",
  "icon": "🌐",
  "config_schema": {
    "type": "object",
    "properties": {
      "path": {"type": "string"},
      "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"]}
    }
  },
  "enabled": true
}
```

## Frontend Integration Guide

### Step 1: Load Palette Data

```javascript
// Load connectors for palette
const connectorsResponse = await fetch('http://localhost:8081/connectors');
const { connectors } = await connectorsResponse.json();

// Load triggers for palette
const triggersResponse = await fetch('http://localhost:8081/triggers');
const { triggers } = await triggersResponse.json();

// Display in UI palette
connectors.forEach(connector => {
  displayConnector({
    icon: connector.icon,
    name: connector.name,
    operations: connector.operations
  });
});

triggers.forEach(trigger => {
  displayTrigger({
    icon: trigger.icon,
    name: trigger.name,
    configSchema: trigger.config_schema
  });
});
```

### Step 2: Create Flow from UI

```javascript
// User designs flow in UI
const flowDesign = {
  id: generateId(),
  name: "My Flow",
  trigger: {
    type: "http",
    path: "/api/my-endpoint",
    method: "GET"
  },
  steps: [
    {
      type: "call",
      name: "database_query",
      connector: "postgres",  // From connector palette
      operation: "query",      // From connector operations
      params: {
        sql: "SELECT * FROM users"
      }
    }
  ]
};

// Submit to Control Plane
const response = await fetch('http://localhost:8081/flows', {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify(flowDesign)
});

// API definition automatically created!
// Flow immediately available on Data Plane!
```

### Step 3: Validate Parameters

```javascript
// Get connector to validate parameters
const connector = connectors.find(c => c.id === 'postgres-connector');
const operation = connector.operations.find(o => o.name === 'query');

// Check required parameters
operation.parameters.forEach(param => {
  if (param.required && !userInput[param.name]) {
    showError(`${param.name} is required`);
  }
  
  // Validate type
  if (param.param_type === 'string' && typeof userInput[param.name] !== 'string') {
    showError(`${param.name} must be a string`);
  }
});
```

## Database Schema

### connector_definitions Table
```sql
CREATE TABLE connector_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    connector_type VARCHAR(100) NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### trigger_definitions Table
```sql
CREATE TABLE trigger_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    trigger_type VARCHAR(100) NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### api_definitions Table (Auto-Updated)
```sql
CREATE TABLE api_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    base_path VARCHAR(255) NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(name, version)
);
```

## Complete Example

### 1. UI Loads Palette

```bash
# Get available connectors
curl http://localhost:8081/connectors

# Get available triggers
curl http://localhost:8081/triggers
```

### 2. User Creates Flow in UI

Flow Designer UI sends:
```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "payment-flow",
    "name": "Payment Processing Flow",
    "trigger": {
      "type": "http",
      "path": "/api/payments",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "start",
        "message": "Processing payment"
      },
      {
        "type": "call",
        "name": "validate",
        "connector": "postgres",
        "operation": "query",
        "params": {
          "sql": "SELECT * FROM accounts WHERE id = $1"
        }
      },
      {
        "type": "call",
        "name": "external_api",
        "connector": "http",
        "operation": "post",
        "params": {
          "url": "https://payment-gateway.com/process",
          "body": {"amount": 100}
        }
      }
    ]
  }'
```

### 3. Auto-Magic Happens

**Control Plane automatically:**
- ✅ Saves flow to database
- ✅ Creates/updates API definition with endpoint `POST /api/payments → payment-flow`
- ✅ Publishes events to NATS
- ✅ Returns flow to UI

**Data Plane automatically:**
- ✅ Receives flow via NATS
- ✅ Registers flow
- ✅ Ready to execute

### 4. Execute Flow

```bash
curl -X POST http://localhost:8080/flows/payment-flow/execute \
  -H "Content-Type: application/json" \
  -d '{"amount": 100, "account_id": 123}'
```

### 5. Check Auto-Generated API

```bash
curl http://localhost:8081/apis

# Response shows auto-generated API with our endpoint:
{
  "apis": [
    {
      "id": "...",
      "name": "Auto-Generated API",
      "version": "1.0",
      "base_path": "/api",
      "endpoints": [
        {
          "path": "/api/payments",
          "method": "POST",
          "flow_id": "payment-flow"
        }
      ]
    }
  ]
}
```

## Benefits for Frontend

### 1. Dynamic Palette
- No hardcoded connector/trigger lists
- Automatically get new connectors when added
- See real-time availability status

### 2. Smart Validation
- Parameter types from connector definitions
- Required field validation
- Default values

### 3. Documentation in Code
- Descriptions for each connector/operation
- Parameter descriptions
- Config schemas for validation

### 4. No Manual API Management
- API definitions created automatically
- Endpoints updated on flow changes
- No API/flow synchronization issues

### 5. Instant Deployment
- Create flow → immediately executable
- Update flow → instantly updated everywhere
- Delete flow → automatically cleaned up

## Testing

### Test Connector Registry
```bash
# List connectors
curl http://localhost:8081/connectors | jq '.connectors[] | {name, operations: .operations[].name}'

# Output:
# {
#   "name": "HTTP/REST",
#   "operations": "get"
# }
# {
#   "name": "HTTP/REST",
#   "operations": "post"
# }
# {
#   "name": "PostgreSQL",
#   "operations": "query"
# }
# {
#   "name": "PostgreSQL",
#   "operations": "execute"
# }
```

### Test Trigger Registry
```bash
# List triggers
curl http://localhost:8081/triggers | jq '.triggers[] | {name, type: .trigger_type}'

# Output:
# {
#   "name": "HTTP Request",
#   "type": "http"
# }
# {
#   "name": "Schedule",
#   "type": "schedule"
# }
```

### Test Auto-API Creation
```bash
# 1. Create flow
curl -X POST http://localhost:8081/flows -H "Content-Type: application/json" -d '{
  "id": "test-flow",
  "name": "Test",
  "trigger": {"type": "http", "path": "/test", "method": "GET"},
  "steps": []
}'

# 2. Check APIs - should have auto-generated entry
curl http://localhost:8081/apis | jq '.apis[] | .endpoints[] | select(.flow_id=="test-flow")'

# Output:
# {
#   "path": "/test",
#   "method": "GET",
#   "flow_id": "test-flow"
# }

# 3. Delete flow
curl -X DELETE http://localhost:8081/flows/test-flow

# 4. Check APIs - endpoint should be removed
curl http://localhost:8081/apis | jq '.apis[] | .endpoints[] | select(.flow_id=="test-flow")'
# (no output - endpoint removed)
```

## Summary

✅ **Connectors persisted in database** - Frontend can load palette
✅ **Triggers persisted in database** - Frontend can load palette  
✅ **API definitions auto-created** - When flow with HTTP trigger created
✅ **API definitions auto-updated** - When flow modified
✅ **API definitions auto-cleaned** - When flow deleted
✅ **Complete metadata** - Operations, parameters, schemas, descriptions
✅ **Event-driven sync** - All changes propagate to Data Plane

Your frontend flow designer can now:
1. Load connectors/triggers from `/connectors` and `/triggers`
2. Display them in a palette with icons and descriptions
3. Validate user input against parameter schemas
4. Create flows that are immediately executable
5. Trust that API definitions stay in sync automatically

---

**No manual API management needed - it's all automatic!** 🎉
