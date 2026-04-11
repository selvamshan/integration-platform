# Quick Start Guide

Get the integration platform running in 5 minutes!

## Prerequisites

- Docker and Docker Compose
- **4GB+ memory allocated to Docker** (8GB recommended)
- 10GB free disk space

## Step 1: Start the Platform

**Recommended: Use the smart build script**

```bash
./build.sh
```

This will automatically:
- Check your system resources
- Choose the best build method
- Build and start all services
- Run health checks

**Alternative: Manual Docker build**

```bash
# Using Make
make build
make up

# Or using Docker Compose
docker-compose up --build -d
```

⚠️ **If build fails with exit code 101**: This means Docker needs more memory. See TROUBLESHOOTING.md for solutions.

Wait about 30 seconds for all services to start.

## Step 2: Verify Services

```bash
# Check health
curl http://localhost:8081/health  # Control Plane
curl http://localhost:8080/health  # Data Plane
```

Expected response:
```json
{
  "status": "healthy",
  "service": "data-plane",
  "timestamp": "2024-02-09T..."
}
```

## Step 3: Run Your First Flow

The easiest way - just make a GET request:

```bash
curl http://localhost:8080/api/trigger/users
```

This will:
1. ✅ Create a flow automatically
2. ✅ Connect to PostgreSQL
3. ✅ Query the users table
4. ✅ Return the results

Expected response:
```json
{
  "rows": [
    {
      "id": 1,
      "name": "Alice Johnson",
      "email": "alice@example.com"
    },
    ...
  ],
  "count": 5
}
```

## Step 4: View the Logs

```bash
# View all logs
make logs

# Or with Docker Compose
docker-compose logs -f data-plane
```

You'll see detailed execution logs:
```
🚀 Executing flow: HTTP Trigger: users
📝 [trigger] HTTP GET triggered on /users
🔌 [fetch_data] Calling connector: postgres - query
📊 Executing query: SELECT * FROM users LIMIT 10
   Rows returned: 5
   ✅ Connector call completed
✅ Flow completed: HTTP Trigger: users
```

## Step 5: Run the Test Suite

```bash
make test
```

This will test all endpoints and show you different ways to use the platform.

## That's It! 🎉

You now have a running integration platform with:
- ✅ HTTP Triggers
- ✅ PostgreSQL Connector
- ✅ Flow Execution Engine
- ✅ Structured Logging
- ✅ Sample Data

## Common Commands

```bash
# Start services
make up

# Stop services
make down

# Restart services
make restart

# View logs
make logs

# Run tests
make test

# Clean everything
make clean
```

## Next Steps

1. **Execute custom flows** - POST to `/flows/{id}/execute`
2. **Create APIs** - POST to control plane `/apis`
3. **Query database** - Use the postgres connector
4. **Make HTTP calls** - Use the http connector

See README.md for detailed documentation!
