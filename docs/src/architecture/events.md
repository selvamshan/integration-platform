# Event-Driven Architecture

The platform uses NATS as its event bus for real-time synchronization between the Control Plane and Data Plane(s).

## NATS Subjects

| Subject | Publisher | Subscriber | Payload |
|---------|-----------|------------|---------|
| `flow.sync` | Control Plane | Data Plane | `FlowSyncEvent { flow_id, action, definition }` |
| `flow.delete` | Control Plane | Data Plane | `FlowDeleteEvent { flow_id }` |
| `connector.sync` | Control Plane | Data Plane | `ConnectorSyncEvent { connector_id, config }` |

## Flow Sync Sequence

```
User creates/updates flow
        │
        ▼
Control Plane saves to PostgreSQL
        │
        ▼
Control Plane publishes flow.sync on NATS
        │
        ├──► Data Plane 1 receives event → updates in-memory cache
        ├──► Data Plane 2 receives event → updates in-memory cache
        └──► Data Plane N receives event → updates in-memory cache
```

All Data Plane instances converge to the same flow state within milliseconds.

## Startup Warm-Up

When a Data Plane starts, it cannot rely on NATS history (NATS Core has no persistence). Instead it:

1. Calls `GET /flows` on the Control Plane REST API
2. Loads all flows into its in-memory cache
3. Subscribes to NATS for future updates

This pattern ensures no flows are missed between restarts.

## Scaling

Because flow state is stored in PostgreSQL and distributed via NATS, you can run multiple Data Plane instances behind a load balancer. Each instance independently maintains the same flow cache.

```
                    Load Balancer
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
    Data Plane 1   Data Plane 2   Data Plane 3
    (cached flows) (cached flows) (cached flows)
          │              │              │
          └──────────────┴──────────────┘
                         │
                    PostgreSQL
```
