# Data Plane Startup Flow Loading

## Problem

Previously, the Data Plane only received flows via NATS events. This caused issues:

❌ **Before:** 
1. Control Plane creates Flow A
2. Flow A published via NATS → Data Plane receives it
3. Data Plane restarts
4. Flow A is lost (only in memory, not reloaded)
5. Flow A cannot be executed until Control Plane publishes it again

## Solution

The Data Plane now **loads all existing flows from the database on startup**.

✅ **After:**
1. Control Plane creates Flow A (saved to database)
2. Flow A published via NATS → Data Plane receives it
3. Data Plane restarts
4. **Data Plane loads ALL flows from database on startup**
5. Flow A immediately available and executable

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Data Plane Startup Sequence                 │
│                                                          │
│  1. Connect to Database                                 │
│     ↓                                                    │
│  2. Connect to NATS                                     │
│     ↓                                                    │
│  3. Initialize Connectors (HTTP, PostgreSQL)            │
│     ↓                                                    │
│  4. 📥 LOAD ALL FLOWS FROM DATABASE ← NEW!              │
│     ↓                                                    │
│  5. Subscribe to NATS events (for new updates)          │
│     ↓                                                    │
│  6. Start HTTP server                                   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## Implementation

### Data Plane Startup

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Connect to database
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    
    // 2. Connect to NATS
    let nats = async_nats::connect(&nats_url).await?;
    
    // 3. Initialize connectors
    let executor = FlowExecutor::new();
    // ... register connectors
    
    // 4. Create state
    let state = Arc::new(AppState {
        executor,
        flows: Arc::new(RwLock::new(HashMap::new())),
        nats,
        db,
    });
    
    // 5. ⭐ LOAD EXISTING FLOWS FROM DATABASE
    load_flows_from_database(state.clone()).await?;
    
    // 6. Start NATS listener for new updates
    tokio::spawn(listen_for_config_updates(state.clone()));
    
    // 7. Start HTTP server
    axum::serve(listener, app).await?;
}
```

### Database Flow Loading

```rust
async fn load_flows_from_database(state: Arc<AppState>) -> Result<()> {
    tracing::info!("📥 Loading existing flows from database...");
    
    // Query all flows from flow_definitions table
    let rows = sqlx::query("SELECT config FROM flow_definitions")
        .fetch_all(&state.db)
        .await?;
    
    let mut flows = state.flows.write().await;
    let mut count = 0;
    
    for row in rows {
        let config: serde_json::Value = row.try_get("config")?;
        
        match serde_json::from_value::<FlowDefinition>(config) {
            Ok(flow) => {
                tracing::info!("  ➕ Loaded flow: {} ({})", flow.name, flow.id);
                flows.insert(flow.id.clone(), flow);
                count += 1;
            }
            Err(e) => {
                tracing::error!("  ❌ Failed to deserialize flow: {}", e);
            }
        }
    }
    
    tracing::info!("✅ Loaded {} flows from database", count);
    Ok(())
}
```

## Dual Source Flow Management

The Data Plane now receives flows from **two sources**:

### 1. Database (on startup)
- **When:** Data Plane starts up
- **Source:** PostgreSQL `flow_definitions` table
- **Purpose:** Recover existing flows after restart
- **Logs:** `📥 Loading existing flows from database...`

### 2. NATS Events (runtime)
- **When:** Flows created/updated/deleted while running
- **Source:** NATS message bus
- **Purpose:** Real-time updates without restart
- **Logs:** `📥 Received event from config.flow.created`

## Example Scenarios

### Scenario 1: Normal Operation

```bash
# 1. Data Plane is running
# 2. Create flow in Control Plane
curl -X POST http://localhost:8081/flows -d '{...}'

# Control Plane:
# ✅ Saved to database
# ✅ Published to NATS

# Data Plane:
# ✅ Received via NATS event
# ✅ Flow immediately available
```

### Scenario 2: Data Plane Restart (Previous Behavior - BROKEN)

```bash
# 1. Create flows A, B, C while Data Plane is running
# 2. Data Plane receives A, B, C via NATS
# 3. Restart Data Plane
docker-compose restart data-plane

# ❌ OLD BEHAVIOR:
# - Flows A, B, C lost
# - Must recreate or republish

# User tries to execute flow:
curl -X POST http://localhost:8080/flows/A/execute -d '{}'
# Error: Flow not found
```

### Scenario 3: Data Plane Restart (New Behavior - FIXED)

```bash
# 1. Create flows A, B, C while Data Plane is running
# 2. Data Plane receives A, B, C via NATS
# 3. Restart Data Plane
docker-compose restart data-plane

# ✅ NEW BEHAVIOR (on startup):
# Data Plane logs:
# 📥 Loading existing flows from database...
#   ➕ Loaded flow: Flow A (flow-a-id)
#   ➕ Loaded flow: Flow B (flow-b-id)
#   ➕ Loaded flow: Flow C (flow-c-id)
# ✅ Loaded 3 flows from database

# User can immediately execute:
curl -X POST http://localhost:8080/flows/A/execute -d '{}'
# ✅ Works!
```

### Scenario 4: Data Plane Starts Before Control Plane

```bash
# 1. Start Data Plane first
docker-compose up -d data-plane

# Data Plane logs:
# 📥 Loading existing flows from database...
# ✅ Loaded 0 flows from database (table is empty)

# 2. Start Control Plane
docker-compose up -d control-plane

# 3. Create flow
curl -X POST http://localhost:8081/flows -d '{...}'

# Data Plane:
# 📥 Received event from config.flow.created
# ✅ Flow available (via NATS)
```

### Scenario 5: Create Flow While Data Plane is Down

```bash
# 1. Stop Data Plane
docker-compose stop data-plane

# 2. Create flows while Data Plane is down
curl -X POST http://localhost:8081/flows -d '{"id": "flow-1", ...}'
curl -X POST http://localhost:8081/flows -d '{"id": "flow-2", ...}'
curl -X POST http://localhost:8081/flows -d '{"id": "flow-3", ...}'

# Control Plane logs:
# ✅ Flow created and published: flow-1
# ✅ Flow created and published: flow-2
# ✅ Flow created and published: flow-3
# (NATS events published but Data Plane not listening)

# 3. Start Data Plane
docker-compose start data-plane

# Data Plane logs (on startup):
# 📥 Loading existing flows from database...
#   ➕ Loaded flow: flow-1
#   ➕ Loaded flow: flow-2
#   ➕ Loaded flow: flow-3
# ✅ Loaded 3 flows from database

# All 3 flows immediately available!
curl -X POST http://localhost:8080/flows/flow-1/execute -d '{}'
# ✅ Works!
```

## Startup Logs

### Successful Startup with Existing Flows

```
🚀 Starting Data Plane with Database Flow Loading
Connecting to database...
✅ Database connected
Connecting to NATS at nats://nats:4222...
✅ NATS connected
✅ Connectors initialized
📥 Loading existing flows from database...
  ➕ Loaded flow: User Lookup Flow (user-lookup-flow)
  ➕ Loaded flow: Product Search (product-search)
  ➕ Loaded flow: Order Processing (order-process)
✅ Loaded 3 flows from database
🎧 Starting config event listener...
✅ Subscribed to config.* events
🌐 Data Plane listening on 0.0.0.0:8080
```

### Startup with No Existing Flows

```
🚀 Starting Data Plane with Database Flow Loading
Connecting to database...
✅ Database connected
Connecting to NATS at nats://nats:4222...
✅ NATS connected
✅ Connectors initialized
📥 Loading existing flows from database...
✅ Loaded 0 flows from database
🎧 Starting config event listener...
✅ Subscribed to config.* events
🌐 Data Plane listening on 0.0.0.0:8080
```

## Runtime Events Still Work

After startup, the Data Plane continues to receive real-time updates:

```
# Flow created while running
📥 Received event from config.flow.created
➕ Adding flow: New Flow (new-flow-id)
✅ Flow registered in data plane

# Flow updated while running
📥 Received event from config.flow.updated
🔄 Updating flow: Updated Flow (flow-id)
✅ Flow updated in data plane

# Flow deleted while running
📥 Received event from config.flow.deleted
➖ Removing flow: flow-id
✅ Flow removed from data plane
```

## New Endpoint: List Flows

The Data Plane now exposes an endpoint to list loaded flows:

```bash
# Check what flows are loaded
curl http://localhost:8080/flows

# Response:
{
  "flows": [
    {
      "id": "user-lookup",
      "name": "User Lookup Flow",
      "trigger": {"type": "http", "path": "/api/users", "method": "GET"},
      "steps": [...]
    },
    {
      "id": "product-search",
      "name": "Product Search",
      ...
    }
  ],
  "count": 2
}
```

## Benefits

### 1. Resilience
- ✅ Data Plane can restart without losing flows
- ✅ Flows survive container restarts
- ✅ No manual intervention needed

### 2. Consistency
- ✅ Database is source of truth
- ✅ Data Plane always syncs with database on startup
- ✅ NATS provides real-time updates

### 3. Deployment Flexibility
- ✅ Can deploy Data Plane before Control Plane
- ✅ Can deploy multiple Data Plane instances
- ✅ All instances load same flows from database

### 4. Development Experience
- ✅ Create flows once, they persist
- ✅ Restart services during development without losing state
- ✅ Easy testing and debugging

## Testing

### Test 1: Create Flow and Restart

```bash
# 1. Create flow
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-persistence",
    "name": "Test Persistence",
    "trigger": {"type": "http", "path": "/test", "method": "GET"},
    "steps": [{"type": "log", "name": "test", "message": "Testing"}]
  }'

# 2. Verify it works
curl -X POST http://localhost:8080/flows/test-persistence/execute -d '{}'
# ✅ Works

# 3. Restart Data Plane
docker-compose restart data-plane

# 4. Wait for startup (check logs)
docker-compose logs data-plane | grep "Loaded"
# Output: ✅ Loaded 1 flows from database

# 5. Test again (should still work!)
curl -X POST http://localhost:8080/flows/test-persistence/execute -d '{}'
# ✅ Still works!
```

### Test 2: Create Flows While Down

```bash
# 1. Stop Data Plane
docker-compose stop data-plane

# 2. Create multiple flows
for i in 1 2 3; do
  curl -X POST http://localhost:8081/flows -d "{
    \"id\": \"flow-$i\",
    \"name\": \"Flow $i\",
    \"trigger\": {\"type\": \"http\", \"path\": \"/flow$i\", \"method\": \"GET\"},
    \"steps\": []
  }"
done

# 3. Start Data Plane
docker-compose start data-plane

# 4. Check logs
docker-compose logs data-plane | tail -20
# Should show: ✅ Loaded 3 flows from database

# 5. Verify all flows available
curl http://localhost:8080/flows | jq '.count'
# Output: 3
```

### Test 3: List Loaded Flows

```bash
# Check what's loaded
curl http://localhost:8080/flows | jq '.flows[] | {id, name}'

# Output:
# {
#   "id": "flow-1",
#   "name": "Flow 1"
# }
# {
#   "id": "flow-2",
#   "name": "Flow 2"
# }
# {
#   "id": "flow-3",
#   "name": "Flow 3"
# }
```

## Monitoring

### Health Check

```bash
curl http://localhost:8080/health

# Response includes flow count:
{
  "status": "healthy",
  "service": "data-plane",
  "timestamp": "2024-02-11T..."
}
```

### Flow Count

```bash
# Get current flow count
curl http://localhost:8080/flows | jq '.count'
```

### Startup Verification

```bash
# Check if flows were loaded on startup
docker-compose logs data-plane | grep "Loaded.*flows from database"

# Example output:
# ✅ Loaded 5 flows from database
```

## Summary

### What Changed

1. ✅ **Data Plane connects to database** on startup
2. ✅ **Loads all flows** from `flow_definitions` table
3. ✅ **Flows persist** across restarts
4. ✅ **New endpoint** `GET /flows` to list loaded flows
5. ✅ **Dual source** - database (startup) + NATS (runtime)

### What Stayed The Same

1. ✅ **NATS events** still work for real-time updates
2. ✅ **Flow creation** in Control Plane unchanged
3. ✅ **Flow execution** API unchanged
4. ✅ **Event-driven** architecture intact

### The Complete Flow Lifecycle

```
Control Plane              Database              NATS              Data Plane
     │                         │                    │                    │
     │ POST /flows             │                    │                    │
     ├─────────────────────────►                    │                    │
     │ Save flow               │                    │                    │
     │◄────────────────────────┤                    │                    │
     │                         │                    │                    │
     │ Publish event           │                    │                    │
     ├──────────────────────────────────────────────►                    │
     │                         │                    │ Event              │
     │                         │                    ├────────────────────►
     │                         │                    │ Add flow (runtime) │
     │                         │                    │                    │
     │                         │                    │    [RESTART]       │
     │                         │                    │                    │
     │                         │        Load flows on startup            │
     │                         │◄───────────────────────────────────────┤
     │                         │ SELECT * FROM flows│                    │
     │                         ├────────────────────────────────────────►
     │                         │  Return flows      │                    │
     │                         │                    │ ✅ Flows available │
```

**Database is source of truth, NATS provides real-time distribution!** 🎉
