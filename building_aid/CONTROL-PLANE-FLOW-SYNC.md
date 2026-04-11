# Control Plane Flow Sync - Push Model

## Overview

The **Control Plane pushes all existing flows to the Data Plane** when it starts up or restarts. This ensures the Data Plane always has access to all flows without directly querying the database.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Data Plane Startup                    │
│                                                          │
│  1. Connect to NATS                                     │
│  2. Initialize Connectors                               │
│  3. Subscribe to config.* events                        │
│  4. 📡 Send registration to Control Plane               │
│  5. ⏳ Wait for flows to be pushed                       │
│  6. ✅ Flows received and loaded                         │
│  7. Start HTTP server                                   │
│                                                          │
└─────────────────────────────────────────────────────────┘
                           │
                           │ NATS: dataplane.register
                           ▼
┌─────────────────────────────────────────────────────────┐
│               Control Plane Flow Sync Service            │
│                                                          │
│  1. Listens on: dataplane.register                      │
│  2. Receives: Data Plane node ID                        │
│  3. Loads: All flows from database                      │
│  4. 📤 Pushes: Each flow via NATS                        │
│  5. ✅ Logs: "Pushed N flows to Data Plane"             │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## How It Works

### 1. Control Plane Startup

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Connect to database
    let db = connect_to_database().await?;
    
    // Connect to NATS
    let nats = connect_to_nats().await?;
    
    // Load all flows from database into memory
    load_flows_from_database(state.clone()).await?;
    
    // Start Flow Sync Service
    tokio::spawn(flow_sync_service(state.clone()));
    
    // Start HTTP server
    start_server().await?;
}
```

**Control Plane loads flows into memory:**
```
📥 Loading flows from database into memory...
  ➕ Loaded: User Lookup Flow (flow-1)
  ➕ Loaded: Product Search (flow-2)
  ➕ Loaded: Order Processing (flow-3)
✅ Loaded 3 flows into memory
```

**Flow Sync Service starts:**
```
🔄 Starting Flow Sync Service...
✅ Flow Sync Service listening for Data Plane registrations
```

### 2. Data Plane Startup

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Generate unique ID
    let node_id = format!("data-plane-{}", Uuid::new_v4());
    
    // Connect to NATS
    let nats = connect_to_nats().await?;
    
    // Subscribe to config events FIRST
    tokio::spawn(listen_for_config_updates(state.clone()));
    
    // Wait for subscription to be ready
    sleep(Duration::from_secs(1)).await;
    
    // Register with Control Plane
    nats.publish("dataplane.register", node_id.as_bytes()).await?;
    
    // Wait for flows to arrive
    sleep(Duration::from_secs(2)).await;
    
    // Start HTTP server
    start_server().await?;
}
```

**Data Plane logs:**
```
🚀 Starting Data Plane with Control Plane Flow Sync
📋 Node ID: data-plane-a1b2c3d4-...
Connecting to NATS...
✅ NATS connected
✅ Connectors initialized
🎧 Starting config event listener...
✅ Subscribed to config.* events
📡 Registering with Control Plane...
✅ Registration sent to Control Plane
⏳ Waiting for flows to be pushed from Control Plane...
```

### 3. Control Plane Receives Registration

**Control Plane Flow Sync Service:**
```
📡 Data Plane registered: data-plane-a1b2c3d4-...
📤 Pushing all flows to Data Plane: data-plane-a1b2c3d4-...
  ✅ Pushed: User Lookup Flow (flow-1)
  ✅ Pushed: Product Search (flow-2)
  ✅ Pushed: Order Processing (flow-3)
✅ Successfully pushed 3 flows to Data Plane: data-plane-a1b2c3d4-...
```

### 4. Data Plane Receives Flows

**Data Plane event listener:**
```
📥 Received event from config.flow.created
  ➕ Adding flow: User Lookup Flow (flow-1)
  ✅ Flow registered (total: 1)
📥 Received event from config.flow.created
  ➕ Adding flow: Product Search (flow-2)
  ✅ Flow registered (total: 2)
📥 Received event from config.flow.created
  ➕ Adding flow: Order Processing (flow-3)
  ✅ Flow registered (total: 3)
```

**After sync:**
```
✅ Received 3 flows from Control Plane
🌐 Data Plane listening on 0.0.0.0:8080
```

## Complete Flow Lifecycle

### Scenario 1: Normal Operation

```bash
# 1. Both services running
# 2. Create flow in Control Plane
curl -X POST http://localhost:8081/flows -d '{...}'

# Control Plane:
# ✅ Saved to database
# ✅ Added to in-memory cache
# ✅ Published to NATS: config.flow.created

# Data Plane:
# ✅ Received via NATS event
# ✅ Flow immediately available
```

### Scenario 2: Data Plane Restart (Main Use Case)

```bash
# 1. Create flows A, B, C
curl -X POST http://localhost:8081/flows -d '{"id": "A", ...}'
curl -X POST http://localhost:8081/flows -d '{"id": "B", ...}'
curl -X POST http://localhost:8081/flows -d '{"id": "C", ...}'

# 2. Flows exist in:
#    - Database ✅
#    - Control Plane memory ✅
#    - Data Plane memory ✅

# 3. Restart Data Plane
docker-compose restart data-plane

# Data Plane startup logs:
# 📡 Registering with Control Plane...
# ⏳ Waiting for flows to be pushed...

# Control Plane logs:
# 📡 Data Plane registered: data-plane-xyz
# 📤 Pushing all flows to Data Plane: data-plane-xyz
#   ✅ Pushed: Flow A
#   ✅ Pushed: Flow B
#   ✅ Pushed: Flow C
# ✅ Successfully pushed 3 flows

# Data Plane logs:
# 📥 Received event from config.flow.created
#   ➕ Adding flow: Flow A
# 📥 Received event from config.flow.created
#   ➕ Adding flow: Flow B
# 📥 Received event from config.flow.created
#   ➕ Adding flow: Flow C
# ✅ Received 3 flows from Control Plane

# 4. Verify flows available
curl http://localhost:8080/flows
# Response: {"flows": [...], "count": 3}

# 5. Execute flow
curl -X POST http://localhost:8080/flows/A/execute -d '{}'
# ✅ Works immediately!
```

### Scenario 3: Create Flows While Data Plane is Down

```bash
# 1. Stop Data Plane
docker-compose stop data-plane

# 2. Create flows while it's down
curl -X POST http://localhost:8081/flows -d '{"id": "flow-1", ...}'
curl -X POST http://localhost:8081/flows -d '{"id": "flow-2", ...}'

# Control Plane logs:
# ✅ Flow created and pushed: flow-1
# ✅ Flow created and pushed: flow-2
# (NATS events published, but nobody listening)

# 3. Start Data Plane
docker-compose start data-plane

# Data Plane startup:
# 📡 Registering with Control Plane...

# Control Plane:
# 📡 Data Plane registered: data-plane-new
# 📤 Pushing all flows to Data Plane...
#   ✅ Pushed: flow-1
#   ✅ Pushed: flow-2

# Data Plane:
# 📥 Received event from config.flow.created
#   ➕ Adding flow: flow-1
# 📥 Received event from config.flow.created
#   ➕ Adding flow: flow-2
# ✅ Received 2 flows from Control Plane

# Both flows immediately available!
```

### Scenario 4: Control Plane Restart

```bash
# 1. Flows exist in database
# 2. Restart Control Plane
docker-compose restart control-plane

# Control Plane startup:
# 📥 Loading flows from database into memory...
#   ➕ Loaded: Flow A
#   ➕ Loaded: Flow B
#   ➕ Loaded: Flow C
# ✅ Loaded 3 flows into memory
# 🔄 Starting Flow Sync Service...
# ✅ Flow Sync Service listening...

# Data Plane is still running with flows in memory
# No need to re-sync (flows already there)

# New flows created after Control Plane restart
# will be distributed normally via NATS
```

### Scenario 5: Multiple Data Plane Instances

```bash
# 1. Create flows
curl -X POST http://localhost:8081/flows -d '{"id": "flow-1", ...}'

# 2. Start first Data Plane
docker-compose up -d data-plane-1

# Control Plane:
# 📡 Data Plane registered: data-plane-aaa
# 📤 Pushing all flows to Data Plane: data-plane-aaa
# ✅ Successfully pushed 1 flows

# 3. Start second Data Plane
docker-compose up -d data-plane-2

# Control Plane:
# 📡 Data Plane registered: data-plane-bbb
# 📤 Pushing all flows to Data Plane: data-plane-bbb
# ✅ Successfully pushed 1 flows

# Both instances have the same flows!
```

## Benefits

### 1. Separation of Concerns
- ✅ Data Plane doesn't need database credentials
- ✅ Data Plane doesn't need database access
- ✅ Control Plane is the single source of truth

### 2. Resilience
- ✅ Data Plane can restart without losing flows
- ✅ Flows survive Data Plane container restarts
- ✅ No manual intervention needed

### 3. Scalability
- ✅ Each Data Plane instance gets all flows on startup
- ✅ Multiple Data Plane instances stay in sync
- ✅ New Data Planes automatically receive all flows

### 4. Security
- ✅ Database access limited to Control Plane only
- ✅ Data Plane only needs NATS connection
- ✅ Clear security boundary

### 5. Simplicity
- ✅ Data Plane has simpler architecture
- ✅ No database queries in Data Plane
- ✅ All communication via NATS

## NATS Subjects

### Registration
```
Subject: dataplane.register
Publisher: Data Plane
Subscriber: Control Plane
Payload: Node ID (string)
Purpose: Data Plane announces itself and requests flows
```

### Flow Distribution
```
Subject: config.flow.created
Publisher: Control Plane
Subscriber: Data Plane
Payload: FlowDefinition (JSON)
Purpose: Distribute flows to Data Plane
```

### Flow Updates (Runtime)
```
Subject: config.flow.updated
Publisher: Control Plane
Subscriber: Data Plane
Payload: FlowDefinition (JSON)
Purpose: Update existing flows
```

### Flow Deletion
```
Subject: config.flow.deleted
Publisher: Control Plane
Subscriber: Data Plane
Payload: flow_id (JSON)
Purpose: Remove flows from Data Plane
```

## Testing

### Test 1: Create Flow and Restart Data Plane

```bash
# 1. Create flow
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-sync",
    "name": "Test Sync",
    "trigger": {"type": "http", "path": "/test", "method": "GET"},
    "steps": [{"type": "log", "name": "test", "message": "Testing sync"}]
  }'

# 2. Verify it works
curl -X POST http://localhost:8080/flows/test-sync/execute -d '{}'
# ✅ Works

# 3. Check Control Plane logs
docker-compose logs control-plane | grep "Flow created"
# ✅ Flow created and pushed: test-sync

# 4. Restart Data Plane
docker-compose restart data-plane

# 5. Check Data Plane logs
docker-compose logs data-plane | tail -30
# Should show:
# 📡 Registering with Control Plane...
# ✅ Registration sent
# ⏳ Waiting for flows...
# 📥 Received event from config.flow.created
#   ➕ Adding flow: Test Sync (test-sync)
# ✅ Received 1 flows from Control Plane

# 6. Test again (should still work!)
curl -X POST http://localhost:8080/flows/test-sync/execute -d '{}'
# ✅ Still works!
```

### Test 2: Create Multiple Flows While Down

```bash
# 1. Stop Data Plane
docker-compose stop data-plane

# 2. Create multiple flows
for i in 1 2 3 4 5; do
  curl -X POST http://localhost:8081/flows -d "{
    \"id\": \"flow-$i\",
    \"name\": \"Flow $i\",
    \"trigger\": {\"type\": \"http\", \"path\": \"/flow$i\", \"method\": \"GET\"},
    \"steps\": []
  }"
done

# 3. Start Data Plane
docker-compose start data-plane

# 4. Check Control Plane logs
docker-compose logs control-plane | grep "Pushed"
# Should show:
# ✅ Pushed: Flow 1
# ✅ Pushed: Flow 2
# ✅ Pushed: Flow 3
# ✅ Pushed: Flow 4
# ✅ Pushed: Flow 5
# ✅ Successfully pushed 5 flows

# 5. Verify on Data Plane
curl http://localhost:8080/flows | jq '.count'
# Output: 5
```

### Test 3: Check Health with Flow Count

```bash
# Health endpoint shows flow count
curl http://localhost:8080/health | jq '.'

# Response:
{
  "status": "healthy",
  "service": "data-plane",
  "node_id": "data-plane-a1b2c3d4-...",
  "flows_loaded": 5,
  "timestamp": "2024-02-11T..."
}
```

## Monitoring

### Control Plane Logs

```bash
# Watch for Data Plane registrations
docker-compose logs -f control-plane | grep "Data Plane registered"

# Watch flow pushes
docker-compose logs -f control-plane | grep "Pushed"
```

### Data Plane Logs

```bash
# Watch for registration
docker-compose logs -f data-plane | grep "Registering"

# Watch for flows being received
docker-compose logs -f data-plane | grep "Received event"

# Check final count
docker-compose logs data-plane | grep "Received.*flows from Control Plane"
```

### NATS Monitoring

```bash
# Monitor NATS traffic
docker-compose logs -f nats

# Check subscriptions
curl http://localhost:8222/subsz
```

## Summary

### What Changed

1. ✅ **Control Plane**: Loads flows from database on startup
2. ✅ **Control Plane**: Runs Flow Sync Service listening for registrations
3. ✅ **Data Plane**: Sends registration message on startup
4. ✅ **Control Plane**: Pushes all flows when registration received
5. ✅ **Data Plane**: Receives flows via NATS events
6. ✅ **Data Plane**: No direct database access needed

### Communication Flow

```
Data Plane                    NATS                    Control Plane
     │                          │                            │
     │ 1. Subscribe config.*    │                            │
     ├─────────────────────────►│                            │
     │                          │                            │
     │ 2. Publish registration  │                            │
     ├─────────────────────────►│ 3. Receive registration   │
     │                          ├───────────────────────────►│
     │                          │                            │
     │                          │ 4. Load flows from DB      │
     │                          │◄───────────────────────────┤
     │                          │                            │
     │                          │ 5. Push flow 1             │
     │ 6. Receive flow 1       ◄├────────────────────────────┤
     ├◄────────────────────────┤                            │
     │                          │ 7. Push flow 2             │
     │ 8. Receive flow 2       ◄├────────────────────────────┤
     ├◄────────────────────────┤                            │
     │                          │ 9. Push flow N             │
     │ 10. Receive flow N      ◄├────────────────────────────┤
     ├◄────────────────────────┤                            │
     │                          │                            │
     ✅ All flows loaded        │                            │
```

**Control Plane pushes, Data Plane receives - perfect separation!** 🎉
