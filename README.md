# Integration Platform

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)

A production-grade integration platform built in Rust, providing a visual flow designer, multi-database connectors, event-driven architecture, and enterprise security features.

## Overview

The platform enables you to build, manage, and execute integration flows through a drag-and-drop UI or REST API. Flows connect HTTP endpoints, databases, and cloud services into automated pipelines.

```
┌─────────────────────────────────────────────────────────────────┐
│                    React Flow Designer UI                        │
│              (Flows · Connectors · Projects · Users)             │
└────────────────────────────┬────────────────────────────────────┘
                             │
          ┌──────────────────┼──────────────────┐
          ▼                  ▼                  ▼
┌─────────────────┐  ┌──────────────┐  ┌──────────────┐
│  Control Plane  │  │     NATS     │  │    Redis     │
│   Port 8081     │  │  Event Bus   │  │  Rate Limit  │
│                 │  │  Port 4222   │  │  Port 6379   │
│ - Flow CRUD     │  └──────┬───────┘  └──────────────┘
│ - Connector Mgmt│         │
│ - RBAC / Auth   │         ▼
│ - Audit Logs    │  ┌──────────────┐
│ - Keycloak SSO  │  │  Data Plane  │
└─────────────────┘  │  Port 8080   │
          │          │              │
          └──────────► - Execute Flows
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

## Features

- **Visual Flow Designer** — React-based drag-and-drop editor powered by React Flow
- **Multi-Connector Support** — HTTP, PostgreSQL, MySQL, MSSQL, Oracle, AWS S3
- **Event-Driven Sync** — NATS-based real-time flow distribution across data planes
- **RBAC & SSO** — Role-based access control with Keycloak OIDC integration
- **Rate Limiting** — Redis-backed per-IP / per-user / per-flow throttling
- **Circuit Breaker** — Automatic failure protection with configurable thresholds
- **Prometheus Metrics** — Full observability at `/metrics`
- **Audit Logs** — Tamper-evident audit trail for all actions
- **Graph Executor** — DAG-based flow execution with parallel step support
- **Loop Executor** — Iterate over collections within a flow
- **Retry Logic** — Configurable exponential backoff per step
- **Project Namespacing** — Multi-tenant isolation via projects
- **Scheduler** — Cron-based flow triggering

## Architecture

### Crates

| Crate | Description |
|---|---|
| `crates/common` | Shared types, traits, and error definitions |
| `crates/integration-runtime` | Flow executor, connectors, transformers, graph/loop executors |
| `crates/control-plane` | Management API, RBAC, Keycloak SSO, audit, scheduler |
| `crates/data-plane` | HTTP trigger handling, flow execution, rate limiting, circuit breaker |

### Services

| Service | Port | Role |
|---|---|---|
| Control Plane | 8081 | Flow & connector management, auth, audit |
| Data Plane | 8080 | Flow execution engine, metrics |
| PostgreSQL | 5432 | Primary datastore |
| Redis | 6379 | Rate limiting, caching |
| NATS | 4222 | Event bus (flow sync, config distribution) |
| NATS Monitor | 8222 | NATS monitoring UI |
| Keycloak | 8180 | SSO / OIDC identity provider |

## Quick Start

### Prerequisites

- Docker and Docker Compose
- 8 GB RAM recommended (Rust builds are memory-intensive)
- 10 GB free disk space

### 1. Start the Stack

```bash
# Copy and set secrets (or use the insecure defaults for local dev)
export JWT_SECRET="your-secret-here"
export ENCRYPTION_KEY="64-hex-chars-here"
export KEYCLOAK_CLIENT_SECRET="your-keycloak-secret"

docker-compose up --build -d
```

Wait until all services report healthy:

```bash
docker-compose ps
```

### 2. Access the UI

Open [http://localhost:5173](http://localhost:5173) (Vite dev server) or serve the built frontend.

Default Keycloak admin: `admin` / `admin123` at [http://localhost:8180](http://localhost:8180).

### 3. Health Checks

```bash
curl http://localhost:8081/health   # Control Plane
curl http://localhost:8080/health   # Data Plane
```

## Connectors

### HTTP

```json
{
  "type": "call",
  "connector": "http",
  "operation": "get",
  "params": { "url": "https://api.example.com/data" }
}
```

Operations: `get`, `post`

### PostgreSQL / MySQL / MSSQL / Oracle

```json
{
  "type": "call",
  "connector": "postgres",
  "operation": "query",
  "params": {
    "sql": "SELECT * FROM orders WHERE status = $1",
    "params": ["pending"]
  }
}
```

Operations: `query`, `execute`

### AWS S3

```json
{
  "type": "call",
  "connector": "s3",
  "operation": "get_object",
  "params": { "bucket": "my-bucket", "key": "data/file.csv" }
}
```

## Flow Definition

```json
{
  "id": "sync-orders",
  "name": "Sync Orders",
  "trigger": {
    "type": "http",
    "path": "/api/sync-orders",
    "method": "POST"
  },
  "steps": [
    {
      "type": "call",
      "name": "fetch_orders",
      "connector": "mysql",
      "operation": "query",
      "params": { "sql": "SELECT * FROM orders WHERE synced = 0" }
    },
    {
      "type": "call",
      "name": "insert_postgres",
      "connector": "postgres",
      "operation": "execute",
      "params": { "sql": "INSERT INTO orders_mirror SELECT * FROM json_populate_recordset(...)" }
    }
  ],
  "rate_limit": {
    "max_requests": 100,
    "window_seconds": 60,
    "key_type": "per_ip"
  },
  "circuit_breaker": {
    "failure_threshold": 5,
    "window_seconds": 60,
    "timeout_seconds": 30,
    "success_threshold": 3
  }
}
```

### Create a Flow

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d @flow.json
```

Once created, the flow is automatically distributed to all Data Plane instances via NATS.

### Execute a Flow

```bash
curl -X POST http://localhost:8080/flows/sync-orders/execute \
  -H "Content-Type: application/json" \
  -d '{"since": "2024-01-01"}'
```

## API Reference

### Control Plane (`:8081`)

| Method | Path | Description |
|---|---|---|
| GET | `/health` | Health check |
| POST | `/auth/login` | Login (JWT) |
| GET | `/flows` | List flows |
| POST | `/flows` | Create flow |
| PUT | `/flows/:id` | Update flow |
| DELETE | `/flows/:id` | Delete flow |
| GET | `/connectors` | List connector definitions |
| POST | `/connectors` | Register connector |
| GET | `/connector-instances` | List connector instances |
| POST | `/connector-instances` | Create connector instance |
| GET | `/projects` | List projects |
| GET | `/users` | List users |
| GET | `/audit-logs` | Query audit logs |
| GET | `/rate-limits` | Rate limit statistics |
| GET | `/rate-limits/:flow_id` | Per-flow rate limit stats |

### Data Plane (`:8080`)

| Method | Path | Description |
|---|---|---|
| GET | `/health` | Health check |
| POST | `/flows/:id/execute` | Execute a flow |
| GET | `/api/trigger/:path` | HTTP trigger (GET) |
| POST | `/api/trigger/:path` | HTTP trigger (POST) |
| GET | `/metrics` | Prometheus metrics |
| GET | `/circuit-breakers` | Circuit breaker states |

## Observability

### Prometheus Metrics

```bash
curl http://localhost:8080/metrics
```

Key metrics:

```
http_requests_total
http_request_duration_seconds
flow_executions_total
flow_executions_success_total
flow_executions_failed_total
flow_execution_duration_seconds
rate_limit_checks_total
rate_limit_blocked_total
circuit_breaker_state{flow_id="..."}
circuit_breaker_opens_total
flows_loaded
redis_operations_total
```

### Sample PromQL

```promql
# Request rate
rate(http_requests_total[5m])

# Flow success rate
rate(flow_executions_success_total[5m]) / rate(flow_executions_total[5m]) * 100

# P95 latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

## Development

### Build Locally

```bash
# Install system dependencies (Ubuntu/Debian)
sudo apt-get install libpq-dev

# Build all crates
cargo build --release

# Start infrastructure only
docker-compose up -d postgres redis nats

# Run services
export DATABASE_URL=postgresql://platform:platform123@localhost:5432/integration_platform
export REDIS_URL=redis://localhost:6379
export NATS_URL=nats://localhost:4222
export JWT_SECRET=dev-secret
export ENCRYPTION_KEY=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

cargo run -p control-plane &
cargo run -p data-plane
```

### Frontend

```bash
cd frontend
npm install
npm run dev    # http://localhost:5173
```

### Run Tests

```bash
# Unit and integration tests (requires running infrastructure)
cargo test

# Specific connector tests
cargo test -p integration-runtime mysql
cargo test -p integration-runtime postgres
```

### Project Structure

```
integration-platform/
├── Cargo.toml                      # Workspace
├── docker-compose.yml              # Full stack
├── docker-compose.test.yml         # Test stack
├── Dockerfile.control-plane
├── Dockerfile.data-plane
├── crates/
│   ├── common/                     # Shared types & traits
│   ├── integration-runtime/        # Flow engine & connectors
│   │   └── src/
│   │       ├── connectors/
│   │       │   ├── aws/s3.rs
│   │       │   ├── http.rs
│   │       │   ├── postgres.rs
│   │       │   ├── mysql.rs
│   │       │   ├── mssql.rs
│   │       │   └── oracle.rs
│   │       ├── graph_executor.rs   # DAG execution
│   │       ├── loop_executor.rs    # Collection iteration
│   │       └── transformers/       # Data transformations
│   ├── control-plane/
│   │   └── src/
│   │       ├── handlers/           # REST API handlers
│   │       ├── rbac.rs             # Role-based access control
│   │       ├── keycloak.rs         # OIDC / SSO
│   │       └── audit.rs            # Audit logging
│   └── data-plane/
│       └── src/
│           ├── rate_limit.rs
│           ├── circuit_breaker.rs
│           ├── retry.rs
│           ├── metrics.rs
│           └── scheduler.rs
├── frontend/                       # React + Vite + Tailwind
│   └── src/
│       ├── pages/                  # Flows, Connectors, Users, Projects, Audit
│       └── components/             # Flow editor, connector forms, layout
├── db/                             # SQL migrations
└── examples/                       # Example flow definitions
```

## Environment Variables

### Control Plane

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | required |
| `REDIS_URL` | Redis connection string | required |
| `NATS_URL` | NATS connection string | required |
| `JWT_SECRET` | JWT signing secret | required |
| `ENCRYPTION_KEY` | AES-GCM key (64 hex chars) | required |
| `KEYCLOAK_SERVER_URL` | Keycloak base URL | required |
| `KEYCLOAK_REALM` | Keycloak realm name | `integration-platform` |
| `KEYCLOAK_CLIENT_ID` | OIDC client ID | `control-plane` |
| `KEYCLOAK_CLIENT_SECRET` | OIDC client secret | required |
| `RUST_LOG` | Log level | `info` |

### Data Plane

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | required |
| `REDIS_URL` | Redis connection string | required |
| `NATS_URL` | NATS connection string | required |
| `JWT_SECRET` | JWT signing secret (same as control plane) | required |
| `ENCRYPTION_KEY` | AES-GCM key (same as control plane) | required |
| `RUST_LOG` | Log level | `info` |

## Troubleshooting

```bash
# View logs
docker-compose logs -f control-plane
docker-compose logs -f data-plane

# Service status
docker-compose ps

# Reset everything
docker-compose down -v
docker-compose up --build

# Out-of-memory during Docker build — use a local build instead
cargo build --release
```

See [building_aid/TROUBLESHOOTING.md](building_aid/TROUBLESHOOTING.md) for detailed solutions.

## Additional Guides

| Guide | Topic |
|---|---|
| [QUICKSTART.md](building_aid/QUICKSTART.md) | Step-by-step first run |
| [HTTP-CONNECTOR-GUIDE.md](building_aid/HTTP-CONNECTOR-GUIDE.md) | HTTP connector details |
| [GRAPH_EXECUTOR.md](building_aid/GRAPH_EXECUTOR.md) | DAG-based flow execution |
| [CIRCUIT-BREAKER.md](building_aid/CIRCUIT-BREAKER.md) | Circuit breaker configuration |
| [RATE-LIMITING.md](building_aid/RATE-LIMITING.md) | Rate limiting setup |
| [METRICS.md](building_aid/METRICS.md) | Prometheus & Grafana setup |
| [RBAC-SETUP.md](building_aid/RBAC-SETUP.md) | Role-based access control |
| [RBAC-KEYCLOAK.md](building_aid/RBAC-KEYCLOAK.md) | Keycloak SSO integration |
| [SCHEDULER-QUICK-START.md](building_aid/SCHEDULER-QUICK-START.md) | Cron-triggered flows |
| [EVENT-DRIVEN-ARCHITECTURE.md](building_aid/EVENT-DRIVEN-ARCHITECTURE.md) | NATS event bus internals |
| [DEPLOYMENT.md](building_aid/DEPLOYMENT.md) | Production deployment |
| [AWS-DEPLOYMENT.md](building_aid/AWS-DEPLOYMENT.md) | AWS deployment guide |
| [AUDIT-LOG-IMPLEMENTATION.md](building_aid/AUDIT-LOG-IMPLEMENTATION.md) | Audit logging details |
| [TRANSFORMATION-GUIDE.md](building_aid/TRANSFORMATION-GUIDE.md) | Data transformation reference |

## License

Licensed under the [GNU Affero General Public License v3.0](LICENSE).

If you use this software to provide a hosted or managed service, you must make your complete source code available to users under the same license.
