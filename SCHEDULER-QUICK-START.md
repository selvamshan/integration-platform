# Scheduler Trigger - Quick Start Guide

Get scheduled flows running in 5 steps.

---

## Step 1: Add Dependencies

In `crates/data-plane/Cargo.toml`:

```toml
[dependencies]
tokio-cron-scheduler = "0.10"
cron = "0.12"
chrono = "0.4"
chrono-tz = "0.8"
uuid = { version = "1.0", features = ["v4", "serde"] }
```

---

## Step 2: Copy Scheduler Implementation

```bash
cp implementations/scheduler.rs your-project/crates/data-plane/src/
```

Add to `src/lib.rs` or `src/main.rs`:
```rust
mod scheduler;
use scheduler::FlowScheduler;
```

---

## Step 3: Initialize Scheduler in main.rs

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Create scheduler
    let scheduler = Arc::new(FlowScheduler::new("UTC").await?);
    
    // Start scheduler
    scheduler.start().await?;
    
    // Add to app state
    let state = AppState {
        scheduler,
        // ... other fields
    };
    
    // Load and schedule flows
    load_scheduled_flows(&state).await?;
    
    // ... rest of your app
    
    // Cleanup on shutdown
    state.scheduler.shutdown().await?;
    
    Ok(())
}
```

---

## Step 4: Schedule Flows

```rust
async fn load_scheduled_flows(state: &AppState) -> Result<()> {
    // Load flows from database
    let flows = load_flows_from_db().await?;
    
    for flow in flows {
        if let Some(trigger) = &flow.trigger {
            if trigger["type"] == "schedule" {
                let cron = trigger["cron"].as_str().unwrap();
                
                // Create executor closure
                let state_clone = state.clone();
                let executor = move |flow_id: String, context: Value| {
                    let state = state_clone.clone();
                    tokio::spawn(async move {
                        execute_flow(&state, &flow_id, context).await
                    })
                };
                
                // Schedule the flow
                state.scheduler.schedule_flow(
                    flow.id.clone(),
                    flow.name.clone(),
                    cron,
                    executor,
                ).await?;
            }
        }
    }
    
    Ok(())
}
```

---

## Step 5: Test with Example Flow

Create a test flow with schedule trigger:

```json
{
  "id": "test-schedule",
  "name": "Test Scheduled Flow",
  "trigger": {
    "type": "schedule",
    "cron": "*/1 * * * *"
  },
  "steps": [
    {
      "type": "log",
      "message": "Scheduled execution at {{trigger.scheduled_time}}"
    }
  ]
}
```

---

## Common Cron Patterns

```
*/1 * * * *     Every minute (testing)
*/5 * * * *     Every 5 minutes
0 * * * *       Every hour
0 */6 * * *     Every 6 hours
0 0 * * *       Daily at midnight
0 3 * * *       Daily at 3 AM
0 9 * * 1       Every Monday at 9 AM
0 0 1 * *       First day of month
0 0 * * 0       Every Sunday
```

---

## Frontend Schedule UI (Optional)

```typescript
// Add to FlowDesigner
<div className="schedule-config">
  <h3>Schedule</h3>
  <input 
    type="text" 
    placeholder="0 0 * * *"
    value={schedule.cron}
    onChange={(e) => setSchedule({...schedule, cron: e.target.value})}
  />
  
  <select onChange={(e) => setSchedule({...schedule, cron: e.target.value})}>
    <option value="*/5 * * * *">Every 5 minutes</option>
    <option value="0 * * * *">Every hour</option>
    <option value="0 0 * * *">Daily at midnight</option>
    <option value="0 3 * * *">Daily at 3 AM</option>
  </select>
</div>
```

---

## Testing

```bash
# Build and run
cargo build --release
./target/release/data-plane

# You should see:
# ✅ Flow scheduler started (timezone: UTC)
# 📅 Scheduled flow: Test Flow (test-schedule) with cron: */1 * * * *

# Wait for execution:
# ⏰ Executing scheduled flow: Test Flow at 2024-03-08 10:00:00 UTC
# ✅ Scheduled flow completed: Test Flow
```

---

## Monitoring

Check logs for:
- `📅 Scheduled flow:` - Flow registered
- `⏰ Executing scheduled flow:` - Execution started
- `✅ Scheduled flow completed:` - Success
- `❌ Scheduled flow failed:` - Errors

---

## Database Updates

Add schedule tracking:

```sql
ALTER TABLE flows
ADD COLUMN schedule_cron VARCHAR(255),
ADD COLUMN schedule_enabled BOOLEAN DEFAULT FALSE,
ADD COLUMN last_execution TIMESTAMPTZ;
```

---

## Summary

✅ **Step 1:** Add dependencies  
✅ **Step 2:** Copy scheduler.rs  
✅ **Step 3:** Initialize in main  
✅ **Step 4:** Load and schedule flows  
✅ **Step 5:** Test with example  

**Your flows now run on schedule!** ⏰✅
