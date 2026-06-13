# Architecture Overview

The platform is a Cargo workspace split into four crates that communicate over NATS and PostgreSQL.

```
┌─────────────────────────────────────────────────────────────────┐
│                    React Flow Designer UI                        │
│              (Flows · Connectors · Projects · Users)             │
└────────────────────────┬────────────────────────────────────────┘
                         │ REST API
          ┌──────────────┼──────────────────┐
          ▼              ▼                  ▼
┌─────────────────┐  ┌──────────────┐  ┌──────────────┐
│  Control Plane  │  │     NATS     │  │    Redis     │
│   Port 8081     │  │  Event Bus   │  │  Rate Limit  │
│                 │  │  Port 4222   │  │  Port 6379   │
│ - Flow CRUD     │  └──────┬───────┘  └──────────────┘
│ - Connector Mgmt│         │ flow.sync events
│ - RBAC / Auth   │         ▼
│ - Audit Logs    │  ┌──────────────┐
│ - Keycloak SSO  │  │  Data Plane  │
└─────────────────┘  │  Port 8080   │
          │          │              │
          └──────────► - Execute Flows
                     │ - HTTP Triggers
                     │ - Rate Limiting
                     │ - Circuit Breaker
                     │ - Prometheus Metrics
                     └──────┬───────┘
                            │
              ┌─────────────▼─────────────┐
              │     Integration Runtime    │
              │                           │
              │  HTTP · PostgreSQL · MySQL │
              │  MSSQL · Oracle · AWS S3  │
              └─────────────┬─────────────┘
                            │
              ┌─────────────▼─────────────┐
              │        PostgreSQL          │
              │    (Flows · Users · Audit) │
              └───────────────────────────┘
```

## Crate Responsibilities

| Crate | Port | Role |
|-------|------|------|
| `control-plane` | 8081 | CRUD for flows, connectors, users, RBAC enforcement |
| `data-plane` | 8080 | Receives flow definitions via NATS, executes HTTP-triggered flows |
| `integration-runtime` | — | Library crate: connector impls + graph executor |
| `common` | — | Library crate: shared types (`FlowDefinition`, `ConnectorConfig`, …) |

## Data Flow

1. User designs a flow in the UI → saved to PostgreSQL via Control Plane.
2. Control Plane publishes a `flow.sync` event on NATS.
3. Data Plane receives the event and caches the flow in memory.
4. An HTTP request hits the Data Plane trigger endpoint.
5. Data Plane looks up the cached flow and calls Integration Runtime to execute it.
6. Integration Runtime runs each step through the appropriate connector.
7. Results are returned to the caller; execution logs are stored in PostgreSQL.

## Security Boundaries

- The Control Plane is protected by Keycloak JWT validation + RBAC middleware.
- The Data Plane exposes public trigger endpoints (rate-limited via Redis) and an internal metrics endpoint.
- Connector credentials are encrypted with AES-GCM before being stored in PostgreSQL.
