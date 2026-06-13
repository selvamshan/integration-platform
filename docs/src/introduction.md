# Integration Platform

A production-grade integration platform built in Rust, providing a visual flow designer, multi-database connectors, event-driven architecture, and enterprise security features.

```
┌─────────────────────────────────────────────────────────────────┐
│                    React Flow Designer UI                        │
│              (Flows · Connectors · Projects · Users)             │
└────────────────────────┬────────────────────────────────────────┘
                         │
          ┌──────────────┼──────────────────┐
          ▼              ▼                  ▼
┌─────────────────┐  ┌──────────────┐  ┌──────────────┐
│  Control Plane  │  │     NATS     │  │    Redis     │
│   Port 8081     │  │  Event Bus   │  │  Rate Limit  │
└─────────────────┘  └──────┬───────┘  └──────────────┘
          │                 │
          └─────────────────▼
                     ┌──────────────┐
                     │  Data Plane  │
                     │  Port 8080   │
                     └──────┬───────┘
                            │
              ┌─────────────▼─────────────┐
              │     Integration Runtime    │
              │  HTTP · PostgreSQL · MySQL │
              │  MSSQL · Oracle · AWS S3  │
              └───────────────────────────┘
```

## Key Features

| Feature | Description |
|---------|-------------|
| **Visual Flow Designer** | React drag-and-drop editor powered by React Flow |
| **Multi-Connector Support** | HTTP, PostgreSQL, MySQL, MSSQL, Oracle, AWS S3 |
| **Event-Driven Sync** | NATS-based real-time flow distribution across data planes |
| **RBAC & SSO** | Role-based access control with Keycloak OIDC |
| **Rate Limiting** | Redis-backed per-IP / per-user / per-flow throttling |
| **Circuit Breaker** | Automatic failure protection with configurable thresholds |
| **Prometheus Metrics** | Full observability at `/metrics` |
| **Audit Logs** | Tamper-evident audit trail for all actions |
| **Graph Executor** | DAG-based parallel flow execution |
| **Loop Executor** | Iterate over collections within a flow |
| **Retry Logic** | Configurable exponential backoff per step |
| **Project Namespacing** | Multi-tenant isolation via projects |

## Workspace Crates

The platform is a Cargo workspace with four crates:

- **`crates/control-plane`** — REST API for managing flows, connectors, users, RBAC
- **`crates/data-plane`** — Executes flows, handles HTTP triggers, enforces rate limits
- **`crates/integration-runtime`** — Connector implementations and flow execution engine
- **`crates/common`** — Shared types used across all crates

## Quick Navigation

- New here? Start with the [Quick Start guide](./guides/quickstart.md).
- Building flows? See [Building Flows](./guides/flows.md).
- Connecting a database? See the [Connectors](./connectors/http.md) section.
- Deploying to production? See [Deployment](./guides/deployment.md).
