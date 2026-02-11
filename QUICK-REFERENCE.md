# Integration Platform - Quick Reference Card

## 🚀 Getting Started

```bash
# Extract
tar -xzf integration-platform.tar.gz
cd integration-platform

# Build & Start (Recommended)
./build.sh

# Or manually
make up
```

## 🔧 Common Commands

```bash
make up          # Start all services
make down        # Stop all services  
make restart     # Restart services
make logs        # View all logs
make test        # Run test suite
make clean       # Remove everything
```

## 🌐 Service Endpoints

| Service        | URL                         | Purpose           |
|----------------|----------------------------|-------------------|
| Data Plane     | http://localhost:8080      | Traffic handling  |
| Control Plane  | http://localhost:8081      | Management API    |
| PostgreSQL     | localhost:5432             | Database          |

## 📡 API Quick Reference

### Data Plane (8080)

**HTTP Trigger (GET)**
```bash
curl http://localhost:8080/api/trigger/users
```

**Execute Flow (POST)**
```bash
curl -X POST http://localhost:8080/flows/my-flow/execute \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}'
```

**Health Check**
```bash
curl http://localhost:8080/health
```

### Control Plane (8081)

**List APIs**
```bash
curl http://localhost:8081/apis
```

**Create API**
```bash
curl -X POST http://localhost:8081/apis \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My API",
    "version": "1.0",
    "base_path": "/api/v1",
    "endpoints": []
  }'
```

**List Flows**
```bash
curl http://localhost:8081/flows
```

## 🔌 Available Connectors

### HTTP Connector
```json
{
  "type": "call",
  "connector": "http",
  "operation": "get",
  "params": {
    "url": "https://api.example.com/data"
  }
}
```

### PostgreSQL Connector
```json
{
  "type": "call",
  "connector": "postgres",
  "operation": "query",
  "params": {
    "sql": "SELECT * FROM users LIMIT 10"
  }
}
```

## 📝 Flow Structure

```json
{
  "id": "my-flow",
  "name": "My Flow",
  "trigger": {
    "type": "http",
    "path": "/api/endpoint",
    "method": "GET"
  },
  "steps": [
    {
      "type": "log",
      "name": "start",
      "message": "Flow started"
    },
    {
      "type": "call",
      "name": "fetch_data",
      "connector": "postgres",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users"
      }
    },
    {
      "type": "log",
      "name": "end",
      "message": "Flow completed"
    }
  ]
}
```

## 🐛 Troubleshooting

### Build Fails (Exit 101)
```bash
# Increase Docker memory to 8GB
# Then: docker system prune -a && make up
```

### Services Won't Start
```bash
docker-compose ps          # Check status
docker-compose logs        # Check logs
docker-compose restart     # Restart
```

### Database Connection Error
```bash
docker-compose restart postgres
sleep 10
docker-compose restart data-plane control-plane
```

### Reset Everything
```bash
make clean     # Remove all containers & data
make up        # Start fresh
```

## 📊 Viewing Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f data-plane
docker-compose logs -f control-plane
docker-compose logs -f postgres

# Last 100 lines
docker-compose logs --tail=100 data-plane
```

## 💾 Database Access

```bash
# Connect to PostgreSQL
docker-compose exec postgres psql -U platform -d integration_platform

# Run query
docker-compose exec postgres psql -U platform -d integration_platform \
  -c "SELECT * FROM users"
```

## 🧪 Testing

```bash
# Run test suite
./test.sh

# Manual tests
curl http://localhost:8080/health
curl http://localhost:8080/api/trigger/users
curl -X POST http://localhost:8080/flows/test/execute \
  -H "Content-Type: application/json" -d '{}'
```

## 📚 Documentation

| File                  | Description                    |
|-----------------------|--------------------------------|
| README.md             | Complete documentation         |
| QUICKSTART.md         | 5-minute quick start           |
| EXAMPLES.md           | Usage examples                 |
| TROUBLESHOOTING.md    | Common issues & solutions      |
| DEPLOYMENT.md         | Deployment guide               |

## ⌨️ Keyboard Shortcuts (if using tmux/screen)

```bash
# Start services in background
make up

# View logs in another terminal
make logs

# Run tests in third terminal  
make test
```

## 🎯 Common Workflows

### Workflow 1: Query Database
```bash
curl http://localhost:8080/api/trigger/users | jq '.'
```

### Workflow 2: Call External API
```bash
curl -X POST http://localhost:8080/flows/external-api/execute \
  -H "Content-Type: application/json" \
  -d '{"url": "https://api.github.com/users/octocat"}'
```

### Workflow 3: Multi-Step Flow
```bash
# 1. Create flow via Control Plane
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d @flow-definition.json

# 2. Execute via Data Plane
curl -X POST http://localhost:8080/flows/my-flow-id/execute \
  -H "Content-Type: application/json" \
  -d '{}'
```

## 🔐 Environment Variables

```bash
DATABASE_URL=postgresql://platform:platform123@postgres:5432/integration_platform
RUST_LOG=data_plane=debug,control_plane=debug,info
```

## 📦 Project Structure

```
integration-platform/
├── crates/
│   ├── common/              # Shared types
│   ├── integration-runtime/ # Flow engine + connectors
│   ├── data-plane/          # Traffic handler
│   └── control-plane/       # Management API
├── docker-compose.yml       # Docker orchestration
├── build.sh                 # Smart build script
├── test.sh                  # Test suite
└── Makefile                 # Convenience commands
```

## 🆘 Getting Help

1. Check logs: `make logs`
2. See TROUBLESHOOTING.md
3. See EXAMPLES.md for usage patterns
4. Check health: `curl http://localhost:8080/health`

---

**Quick Test**
```bash
curl http://localhost:8080/api/trigger/users
```

If this works, everything is running correctly! 🎉
