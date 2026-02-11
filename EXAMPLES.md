# Integration Platform Examples

This file contains real-world examples of using the integration platform.

## Example 1: Simple HTTP Trigger

**Scenario**: Expose a database query via HTTP GET

```bash
# Just make a GET request to any path under /api/trigger/
curl http://localhost:8080/api/trigger/users
```

**What happens**:
1. Flow is auto-created with HTTP trigger
2. PostgreSQL query executes: `SELECT * FROM users LIMIT 10`
3. Results are returned as JSON

**Response**:
```json
{
  "rows": [
    {"id": 1, "name": "Alice Johnson", "email": "alice@example.com"},
    {"id": 2, "name": "Bob Smith", "email": "bob@example.com"}
  ],
  "count": 2
}
```

## Example 2: Execute Flow with Custom Logic

**Scenario**: Run a pre-defined flow with custom input

```bash
curl -X POST http://localhost:8080/flows/my-etl-flow/execute \
  -H "Content-Type: application/json" \
  -d '{
    "source": "users",
    "limit": 5
  }'
```

**What happens**:
1. Flow "my-etl-flow" executes
2. Logs start message
3. Queries database
4. Logs completion
5. Returns results

**Logs you'll see**:
```
🚀 Executing flow: Sample Flow my-etl-flow
📝 [start] Flow execution started
🔌 [fetch_users] Calling connector: postgres - query
📊 Executing query: SELECT id, name, email FROM users LIMIT 5
   Rows returned: 5
   ✅ Connector call completed
📝 [complete] Flow execution completed
✅ Flow completed: Sample Flow my-etl-flow
```

## Example 3: Database Query Flow

**Scenario**: Query specific user data

Create a custom flow:

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user-lookup-flow",
    "name": "User Lookup Flow",
    "trigger": {
      "type": "http",
      "path": "/api/users/lookup",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "start",
        "message": "Looking up user..."
      },
      {
        "type": "call",
        "name": "query_user",
        "connector": "postgres",
        "operation": "query",
        "params": {
          "sql": "SELECT * FROM users WHERE email = 'alice@example.com'"
        }
      },
      {
        "type": "log",
        "name": "found",
        "message": "User found!"
      }
    ]
  }'
```

Execute it:

```bash
curl -X POST http://localhost:8080/flows/user-lookup-flow/execute \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com"}'
```

## Example 4: HTTP Connector - External API Call

**Scenario**: Fetch data from an external API

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "github-flow",
    "name": "Fetch GitHub User",
    "trigger": {
      "type": "http",
      "path": "/api/github",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "start",
        "message": "Fetching GitHub user..."
      },
      {
        "type": "call",
        "name": "get_github_user",
        "connector": "http",
        "operation": "get",
        "params": {
          "url": "https://api.github.com/users/octocat"
        }
      },
      {
        "type": "log",
        "name": "complete",
        "message": "GitHub user fetched successfully"
      }
    ]
  }'
```

Execute it:

```bash
curl -X POST http://localhost:8080/flows/github-flow/execute \
  -H "Content-Type: application/json" \
  -d '{}'
```

## Example 5: Multi-Step Flow (Database + HTTP)

**Scenario**: Fetch user from DB, then enrich with external data

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "enrich-user-flow",
    "name": "Enrich User Data",
    "trigger": {
      "type": "http",
      "path": "/api/enrich",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "start",
        "message": "Starting user enrichment..."
      },
      {
        "type": "call",
        "name": "get_local_user",
        "connector": "postgres",
        "operation": "query",
        "params": {
          "sql": "SELECT * FROM users WHERE id = 1"
        }
      },
      {
        "type": "log",
        "name": "fetched",
        "message": "User fetched from database"
      },
      {
        "type": "call",
        "name": "get_external_data",
        "connector": "http",
        "operation": "get",
        "params": {
          "url": "https://jsonplaceholder.typicode.com/users/1"
        }
      },
      {
        "type": "log",
        "name": "enriched",
        "message": "User data enriched with external source"
      }
    ]
  }'
```

## Example 6: Insert Data Flow

**Scenario**: Insert new user into database

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "create-user-flow",
    "name": "Create New User",
    "trigger": {
      "type": "http",
      "path": "/api/users/create",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "start",
        "message": "Creating new user..."
      },
      {
        "type": "call",
        "name": "insert_user",
        "connector": "postgres",
        "operation": "execute",
        "params": {
          "sql": "INSERT INTO users (name, email) VALUES ('\''John Doe'\'', '\''john@example.com'\'')"
        }
      },
      {
        "type": "log",
        "name": "created",
        "message": "User created successfully"
      }
    ]
  }'
```

Execute:

```bash
curl -X POST http://localhost:8080/flows/create-user-flow/execute \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Doe",
    "email": "john@example.com"
  }'
```

## Example 7: Create API Definition

**Scenario**: Define an API with multiple endpoints

```bash
curl -X POST http://localhost:8081/apis \
  -H "Content-Type: application/json" \
  -d '{
    "name": "User Management API",
    "version": "1.0",
    "base_path": "/api/v1",
    "endpoints": [
      {
        "path": "/users",
        "method": "GET",
        "flow_id": "list-users-flow"
      },
      {
        "path": "/users",
        "method": "POST",
        "flow_id": "create-user-flow"
      },
      {
        "path": "/users/:id",
        "method": "GET",
        "flow_id": "get-user-flow"
      }
    ]
  }'
```

## Example 8: Logging Flow

**Scenario**: Flow with detailed logging at each step

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "logging-example",
    "name": "Logging Example Flow",
    "trigger": {
      "type": "http",
      "path": "/api/log-example",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "step1",
        "message": "Step 1: Received request"
      },
      {
        "type": "log",
        "name": "step2",
        "message": "Step 2: Processing data"
      },
      {
        "type": "call",
        "name": "database",
        "connector": "postgres",
        "operation": "query",
        "params": {
          "sql": "SELECT COUNT(*) as total FROM users"
        }
      },
      {
        "type": "log",
        "name": "step3",
        "message": "Step 3: Query completed"
      },
      {
        "type": "log",
        "name": "step4",
        "message": "Step 4: Sending response"
      }
    ]
  }'
```

Execute and watch the logs:

```bash
# In one terminal
docker-compose logs -f data-plane

# In another terminal
curl -X POST http://localhost:8080/flows/logging-example/execute \
  -H "Content-Type: application/json" \
  -d '{}'
```

## Example 9: POST to External API

**Scenario**: Send data to an external API

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "webhook-flow",
    "name": "Webhook Notification Flow",
    "trigger": {
      "type": "http",
      "path": "/api/notify",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "start",
        "message": "Sending webhook notification..."
      },
      {
        "type": "call",
        "name": "send_webhook",
        "connector": "http",
        "operation": "post",
        "params": {
          "url": "https://webhook.site/your-unique-id",
          "body": {
            "event": "user.created",
            "data": {
              "user_id": 123,
              "name": "John Doe"
            }
          }
        }
      },
      {
        "type": "log",
        "name": "sent",
        "message": "Webhook sent successfully"
      }
    ]
  }'
```

## Example 10: List All Resources

**Scenario**: Check what's configured in the platform

```bash
# List all APIs
curl http://localhost:8081/apis

# List all flows
curl http://localhost:8081/flows

# Check data plane health
curl http://localhost:8080/health

# Check control plane health
curl http://localhost:8081/health
```

## Testing Tips

1. **Watch logs in real-time**:
   ```bash
   docker-compose logs -f
   ```

2. **View only data plane logs**:
   ```bash
   docker-compose logs -f data-plane
   ```

3. **View only database logs**:
   ```bash
   docker-compose logs -f postgres
   ```

4. **Check database directly**:
   ```bash
   docker-compose exec postgres psql -U platform -d integration_platform
   
   # Then run SQL:
   SELECT * FROM users;
   ```

5. **Pretty print JSON responses**:
   ```bash
   curl http://localhost:8080/api/trigger/users | jq '.'
   ```

## Common Patterns

### Pattern 1: ETL (Extract, Transform, Load)

1. Extract: Query source database
2. Transform: Use transform step (or external service)
3. Load: Insert into target database

### Pattern 2: API Gateway

1. Receive HTTP request
2. Validate/transform request
3. Route to appropriate backend
4. Transform response
5. Return to client

### Pattern 3: Event Processing

1. Trigger on HTTP/schedule
2. Fetch data
3. Process/enrich
4. Send to downstream systems (webhooks, queues)

### Pattern 4: Data Sync

1. Query source system
2. Compare with destination
3. Update/insert as needed
4. Log results

## Troubleshooting

If a flow fails, check:
1. Logs: `docker-compose logs -f data-plane`
2. Database connection: `docker-compose exec postgres pg_isready`
3. Service health: `curl http://localhost:8080/health`
4. Input payload: Make sure JSON is valid
