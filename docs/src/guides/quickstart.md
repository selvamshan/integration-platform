# Quick Start

Get the integration platform running in 5 minutes.

## Prerequisites

- Docker and Docker Compose
- 4 GB+ memory allocated to Docker (8 GB recommended)
- 10 GB free disk space

## Step 1: Start the Platform

```bash
# Recommended: smart build script (checks resources, picks best build method)
./build.sh

# Or manually with Make
make build && make up

# Or directly with Docker Compose
docker-compose up --build -d
```

Wait about 30 seconds for all services to start.

> **If you get exit code 101:** Docker needs more memory. See [Troubleshooting](./troubleshooting.md).

## Step 2: Verify Services

```bash
curl http://localhost:8081/health   # Control Plane
curl http://localhost:8080/health   # Data Plane
```

Expected response:

```json
{
  "status": "healthy",
  "service": "data-plane",
  "timestamp": "2024-02-09T..."
}
```

## Step 3: Trigger Your First Flow

```bash
curl http://localhost:8080/api/trigger/users
```

This automatically creates a flow, queries PostgreSQL, and returns results:

```json
{
  "rows": [
    { "id": 1, "name": "Alice Johnson", "email": "alice@example.com" },
    { "id": 2, "name": "Bob Smith",    "email": "bob@example.com" }
  ],
  "count": 5
}
```

## Step 4: Run the Full Test Suite

```bash
make test
```

## Common Commands

| Command | Description |
|---------|-------------|
| `make up` | Start all services |
| `make down` | Stop all services |
| `make restart` | Restart services |
| `make logs` | Tail logs |
| `make test` | Run tests |
| `make clean` | Remove containers and volumes |

## Next Steps

- [Build custom flows](./flows.md)
- [Add database connectors](../connectors/postgres.md)
- [Set up RBAC](./rbac.md)
- [Deploy to production](./deployment.md)
