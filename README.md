# Integration Platform - Complete Backend Implementation

A complete integration platform with Data Plane, Control Plane, Integration Runtime, and Connectors.

## Features

- ✅ **Data Plane** - Handles all traffic and flow execution
- ✅ **Control Plane** - API and flow management
- ✅ **Integration Runtime** - Flow execution engine
- ✅ **HTTP Connector** - Make HTTP GET/POST requests
- ✅ **PostgreSQL Connector** - Database queries and operations
- ✅ **Logging** - Structured logging with tracing
- ✅ **Docker Compose** - Complete stack deployment

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Data Plane (Port 8080)                │
│  - Flow Execution                                        │
│  - HTTP Triggers                                         │
│  - Connector Integration                                 │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│               Integration Runtime                        │
│  - Flow Executor                                         │
│  - HTTP Connector                                        │
│  - PostgreSQL Connector                                  │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│               Control Plane (Port 8081)                  │
│  - API Management                                        │
│  - Flow Management                                       │
│  - Database Schema Management                            │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│                PostgreSQL Database                       │
│  - API Definitions                                       │
│  - Flow Definitions                                      │
│  - User Data                                             │
└──────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Docker and Docker Compose
- curl (for testing)

### 1. Start the Platform

```bash
# Build and start all services
docker-compose up --build

# Or run in background
docker-compose up --build -d
```

This will start:
- PostgreSQL database (port 5432)
- Control Plane (port 8081)
- Data Plane (port 8080)

Wait for all services to be healthy. You should see:
```
✅ Database connected
✅ Migrations completed
✅ Sample data inserted
✅ Connectors initialized
```

### 2. Test the Services

#### Health Checks

```bash
# Control Plane health
curl http://localhost:8081/health

# Data Plane health
curl http://localhost:8080/health
```

#### Trigger a Flow with HTTP GET

The simplest way to test - just make a GET request:

```bash
# This will automatically create and execute a flow
curl http://localhost:8080/api/trigger/users
```

Response:
```json
{
  "rows": [
    {
      "id": 1,
      "name": "Alice Johnson",
      "email": "alice@example.com",
      "created_at": "2024-..."
    },
    ...
  ],
  "count": 5
}
```

#### Execute a Custom Flow

```bash
# Execute a flow that queries the database
curl -X POST http://localhost:8080/flows/my-flow/execute \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Hello from integration platform"
  }'
```

Response:
```json
{
  "flow_id": "my-flow",
  "status": "completed",
  "result": {
    "rows": [...],
    "count": 5
  },
  "timestamp": "2024-02-09T..."
}
```

### 3. View Logs

```bash
# View all logs
docker-compose logs -f

# View only data-plane logs
docker-compose logs -f data-plane

# View only control-plane logs
docker-compose logs -f control-plane
```

You'll see detailed execution logs:
```
🚀 Executing flow: Sample Flow my-flow
📝 [start] Flow execution started
   Current payload: {...}
🔌 [fetch_users] Calling connector: postgres - query
📊 Executing query: SELECT id, name, email FROM users LIMIT 5
   Rows returned: 5
   ✅ Connector call completed
📝 [complete] Flow execution completed
✅ Flow completed: Sample Flow my-flow
```

## API Reference

### Control Plane (Port 8081)

#### List APIs
```bash
GET http://localhost:8081/apis
```

#### Create API
```bash
POST http://localhost:8081/apis
Content-Type: application/json

{
  "name": "User API",
  "version": "1.0",
  "base_path": "/api/v1",
  "endpoints": [
    {
      "path": "/users",
      "method": "GET",
      "flow_id": "list-users-flow"
    }
  ]
}
```

#### List Flows
```bash
GET http://localhost:8081/flows
```

### Data Plane (Port 8080)

#### HTTP Trigger (GET)
```bash
GET http://localhost:8080/api/trigger/{path}
```

Example:
```bash
curl http://localhost:8080/api/trigger/users
curl http://localhost:8080/api/trigger/data
```

#### Execute Flow (POST)
```bash
POST http://localhost:8080/flows/{flow_id}/execute
Content-Type: application/json

{
  "key": "value",
  ...
}
```

## Flow Definition Structure

Flows are defined with the following structure:

```json
{
  "id": "unique-flow-id",
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
      "name": "database_query",
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

## Available Connectors

### HTTP Connector

Operations:
- `get` - Make HTTP GET request
- `post` - Make HTTP POST request

Parameters:
```json
{
  "url": "https://api.example.com/endpoint",
  "body": { ... }  // for POST only
}
```

### PostgreSQL Connector

Operations:
- `query` - Execute SELECT query
- `execute` - Execute INSERT/UPDATE/DELETE

Parameters:
```json
{
  "sql": "SELECT * FROM table WHERE id = $1",
  "params": [value1, value2, ...]  // optional
}
```

## Database Schema

The platform automatically creates these tables:

### users
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)
```

Sample data is automatically inserted on first run.

### api_definitions
```sql
CREATE TABLE api_definitions (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    base_path VARCHAR(255) NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)
```

### flow_definitions
```sql
CREATE TABLE flow_definitions (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)
```

## Development

### Project Structure

```
integration-platform/
├── Cargo.toml                    # Workspace configuration
├── docker-compose.yml            # Docker orchestration
├── Dockerfile.control-plane      # Control plane image
├── Dockerfile.data-plane         # Data plane image
└── crates/
    ├── common/                   # Shared types and traits
    │   └── src/lib.rs
    ├── integration-runtime/      # Flow execution engine
    │   └── src/
    │       ├── lib.rs
    │       └── connectors/
    │           ├── http.rs       # HTTP connector
    │           └── postgres.rs   # PostgreSQL connector
    ├── data-plane/               # Traffic handler
    │   └── src/main.rs
    └── control-plane/            # Management API
        └── src/main.rs
```

### Build Locally

```bash
# Build all crates
cargo build --release

# Run control plane
cargo run -p control-plane

# Run data plane (in another terminal)
cargo run -p data-plane
```

### Environment Variables

- `DATABASE_URL` - PostgreSQL connection string
- `RUST_LOG` - Logging level (default: `info`)

## Troubleshooting

### Services not starting

```bash
# Check service status
docker-compose ps

# Restart services
docker-compose restart

# Check logs
docker-compose logs
```

### Database connection errors

```bash
# Check if PostgreSQL is healthy
docker-compose exec postgres pg_isready

# Restart PostgreSQL
docker-compose restart postgres
```

### Reset everything

```bash
# Stop and remove all containers and volumes
docker-compose down -v

# Rebuild and start
docker-compose up --build
```

## Testing Examples

### Test HTTP Connector

```bash
curl -X POST http://localhost:8080/flows/http-test/execute \
  -H "Content-Type: application/json" \
  -d '{
    "connector": "http",
    "operation": "get",
    "url": "https://jsonplaceholder.typicode.com/users/1"
  }'
```

### Test Database Connector

```bash
curl -X POST http://localhost:8080/flows/db-test/execute \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT * FROM users WHERE email LIKE '\''%example.com'\'' LIMIT 3"
  }'
```

## Next Steps

1. **Add more connectors** - Kafka, Redis, S3, etc.
2. **Add authentication** - JWT, OAuth2
3. **Add rate limiting** - Redis-based rate limiter
4. **Add WASM support** - Hot-reloadable policies
5. **Add event bus** - NATS for config distribution
6. **Add monitoring** - Prometheus metrics
7. **Add UI** - Flow designer and API console

## License

MIT

## ⚠️ Important Build Notes

### Docker Build Requirements

Building Rust in Docker requires significant resources:
- **Memory**: Minimum 4GB, recommended 8GB
- **Disk Space**: At least 10GB free
- **Time**: First build takes 10-20 minutes

### Recommended Build Method

Use the smart build script:

```bash
./build.sh
```

This script will:
1. Check your system resources
2. Detect if Rust is installed locally
3. Choose the best build method automatically
4. Handle all setup for you

### Alternative Build Methods

#### Method 1: Increase Docker Memory (Easiest)

**Docker Desktop (Mac/Windows):**
1. Open Docker Desktop
2. Go to Settings → Resources
3. Increase Memory to 8GB
4. Click "Apply & Restart"
5. Run `make up`

#### Method 2: Build Locally (Fastest)

If you have Rust installed:

```bash
# Install dependencies
# Ubuntu/Debian:
sudo apt-get install libpq-dev

# macOS:
brew install postgresql

# Build
cargo build --release

# Start only PostgreSQL in Docker
docker-compose up -d postgres

# Run services
export DATABASE_URL=postgresql://platform:platform123@localhost:5432/integration_platform
cargo run --release -p control-plane &
cargo run --release -p data-plane &
```

#### Method 3: Use Debug Build (Faster Compilation)

Edit both Dockerfiles and remove `--release` flag:

```dockerfile
# Change from:
RUN cargo build --release -p data-plane

# To:
RUN cargo build -p data-plane
```

Debug builds are faster but larger binaries.

### Troubleshooting Build Failures

See **TROUBLESHOOTING.md** for detailed solutions to common issues:
- Docker build exit code 101
- Out of memory errors  
- Port conflicts
- Dependency download failures
- And more...

Quick fix for most issues:
```bash
docker system prune -a
docker-compose build --no-cache
```


## 🌟 Event-Driven Configuration Distribution

### Real-Time Flow Synchronization

The platform now uses **NATS** for event-driven config distribution:

```
Control Plane → NATS → Data Plane(s)
  (Create Flow)   (Event)  (Auto-Register)
```

**How it works:**

1. **Create flow in Control Plane:**
```bash
curl -X POST http://localhost:8081/flows -d '{...}'
```

2. **Flow automatically available on ALL Data Planes** (no restart!)

3. **Execute immediately:**
```bash
curl -X POST http://localhost:8080/flows/my-flow/execute -d '{}'
```

### Benefits

- ✅ **Zero-downtime updates** - Update flows without restart
- ✅ **Instant propagation** - Changes available in milliseconds  
- ✅ **Horizontal scaling** - Add data planes dynamically
- ✅ **Consistency** - All instances have identical config
- ✅ **Decoupled** - Services are independent

### Services

| Service        | Port  | Purpose                          |
|----------------|-------|----------------------------------|
| NATS           | 4222  | Event bus                        |
| NATS Monitor   | 8222  | Monitoring UI                    |
| Control Plane  | 8081  | Flow management + event publish  |
| Data Plane     | 8080  | Flow execution + event subscribe |
| PostgreSQL     | 5432  | Persistent storage               |

### Test Event Distribution

```bash
# Run event-driven test suite
./test-events.sh

# View event flow
docker-compose logs -f control-plane | grep '📤'  # Publishing
docker-compose logs -f data-plane | grep '📥'     # Receiving
```

See **EVENT-DRIVEN-ARCHITECTURE.md** for complete documentation.


## 🎨 **Flow Designer UI Support**

### Connector & Trigger Palette

The Control Plane maintains a **registry of connectors and triggers** in the database for frontend flow designer UIs.

**Load Palette Data:**
```bash
# Get available connectors
curl http://localhost:8081/connectors

# Get available triggers  
curl http://localhost:8081/triggers
```

**Built-in Components:**
- 🌐 **HTTP Connector** - GET, POST operations
- 🐘 **PostgreSQL Connector** - query, execute operations
- 🌐 **HTTP Trigger** - Trigger on HTTP request
- ⏰ **Schedule Trigger** - Trigger on cron schedule

### Automatic API Management

**API definitions are automatically created/updated/deleted** when flows with HTTP triggers change.

**Example:**
```bash
# 1. Create flow with HTTP trigger
curl -X POST http://localhost:8081/flows -d '{
  "id": "my-flow",
  "trigger": {"type": "http", "path": "/api/users", "method": "GET"},
  ...
}'

# 2. API endpoint automatically created: GET /api/users → my-flow

# 3. Update flow path
curl -X PUT http://localhost:8081/flows/my-flow -d '{
  "trigger": {"type": "http", "path": "/api/v2/users", "method": "GET"}
}'

# 4. API endpoint automatically updated: GET /api/v2/users → my-flow

# 5. Delete flow
curl -X DELETE http://localhost:8081/flows/my-flow

# 6. API endpoint automatically removed
```

**Benefits:**
- ✅ No manual API management
- ✅ APIs always in sync with flows
- ✅ Frontend gets connector/trigger metadata
- ✅ Parameter validation schemas included

### Test UI Features

```bash
# Test connector/trigger registry and auto-API
./test-ui-palette.sh
```

See **UI-PALETTE-GUIDE.md** for complete documentation and frontend integration guide.


## 🛡️ **Rate Limiting with Redis**

### Distributed Rate Limiting

The platform includes **Redis-based rate limiting** enforced at the Data Plane level with monitoring in the Control Plane.

**Features:**
- ✅ **Per-flow rate limits** configured in flow definition
- ✅ **Multiple key types:** global, per-IP, per-user, per-flow
- ✅ **Redis persistence** for distributed consistency
- ✅ **Real-time monitoring** via NATS events
- ✅ **Statistics API** in Control Plane
- ✅ **Custom error messages**

### Rate Limit Policy

```json
{
  "id": "my-api",
  "name": "My API",
  "trigger": {"type": "http", "path": "/api/data", "method": "GET"},
  "steps": [...],
  "rate_limit": {
    "max_requests": 100,
    "window_seconds": 60,
    "key_type": "per_ip",
    "message": "Too many requests. Please try again later."
  }
}
```

### Example: Create Rate Limited Flow

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "public-api",
    "name": "Public API",
    "trigger": {"type": "http", "path": "/api/public", "method": "GET"},
    "steps": [...],
    "rate_limit": {
      "max_requests": 10,
      "window_seconds": 60,
      "key_type": "per_ip",
      "message": "Maximum 10 requests per minute"
    }
  }'
```

**When limit exceeded:**
```json
{
  "error": "Maximum 10 requests per minute",
  "flow_id": "public-api",
  "limit": 10,
  "window_seconds": 60
}
```
**HTTP Status:** `429 Too Many Requests`

### Monitor Rate Limits

```bash
# Get all rate limit statistics
curl http://localhost:8081/rate-limits

# Get specific flow statistics
curl http://localhost:8081/rate-limits/my-flow

# Test rate limiting
./test-rate-limiting.sh
```

### Architecture

```
Client Request
    ↓
Data Plane → Rate Limit Middleware → Redis Check
    ↓                                      ↓
  Allow                                 Block (429)
    ↓
Execute Flow
    ↓
NATS Event → Control Plane → Statistics API
```

See **RATE-LIMITING.md** for complete documentation.


## 📊 **Metrics & Observability**

### Prometheus Metrics

The Data Plane exposes **Prometheus metrics** for comprehensive monitoring at `/metrics` endpoint.

**Metrics Categories:**
- **HTTP Metrics** - Request count, duration histograms
- **Flow Execution** - Success/failure rates, execution duration
- **Rate Limiting** - Checks, blocks, allow rates
- **System Health** - Flows loaded, Redis operations

### Available Metrics

```bash
# Access metrics endpoint
curl http://localhost:8080/metrics

# Key metrics:
# - http_requests_total
# - http_request_duration_seconds (histogram)
# - flow_executions_total
# - flow_executions_success_total
# - flow_executions_failed_total
# - flow_execution_duration_seconds (histogram)
# - rate_limit_checks_total
# - rate_limit_blocked_total
# - flows_loaded (gauge)
# - redis_operations_total
```

### Sample PromQL Queries

```promql
# Request rate (QPS)
rate(http_requests_total[5m])

# Flow success rate
rate(flow_executions_success_total[5m]) / rate(flow_executions_total[5m]) * 100

# P95 latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Rate limit block rate
rate(rate_limit_blocked_total[5m]) / rate(rate_limit_checks_total[5m]) * 100
```

### Test Metrics

```bash
# Run metrics test suite
./test-metrics.sh

# This generates load and verifies:
# ✅ Metrics endpoint working
# ✅ HTTP metrics tracked
# ✅ Flow metrics tracked
# ✅ Histogram data available
# ✅ Rate limit metrics tracked
```

### Grafana Dashboard Setup

See **METRICS.md** for:
- Complete metrics reference
- Prometheus configuration
- Grafana dashboard examples
- Alerting rules
- Best practices

**Example Dashboard Panels:**
- Request Rate (QPS)
- Flow Success Rate (%)
- P50/P95/P99 Latency
- Active Flows Count
- Rate Limit Block Rate


## 🔌 **Circuit Breaker Pattern**

### Automatic Failure Protection

The Data Plane includes a **Circuit Breaker** to prevent cascading failures and provide graceful degradation.

**States:**
- **CLOSED** - Normal operation, all requests allowed
- **OPEN** - Too many failures, all requests rejected (HTTP 503)
- **HALF-OPEN** - Testing recovery, limited requests allowed

### Configuration

```json
{
  "id": "external-api",
  "name": "External API Flow",
  "trigger": {...},
  "steps": [...],
  "circuit_breaker": {
    "failure_threshold": 5,
    "window_seconds": 60,
    "timeout_seconds": 30,
    "success_threshold": 3
  }
}
```

**Behavior:**
1. After 5 failures → Circuit OPENS
2. All requests rejected for 30 seconds
3. After timeout → Circuit HALF-OPEN
4. If 3 consecutive successes → Circuit CLOSES
5. If any failure in half-open → Circuit OPENS again

### Circuit Breaker Metrics

```prometheus
# Current state (0=closed, 1=open, 2=half_open)
circuit_breaker_state{flow_id="my-flow"}

# State transitions
circuit_breaker_opens_total
circuit_breaker_closes_total
circuit_breaker_half_opens_total

# Rejected requests
circuit_breaker_rejected_total
```

### Monitoring

```bash
# Check circuit breaker status
curl http://localhost:8080/circuit-breakers

# Test circuit breaker
./test-circuit-breaker.sh
```

**Benefits:**
- ✅ Prevent cascading failures
- ✅ Fast-fail for unavailable services
- ✅ Automatic recovery
- ✅ Per-flow configuration
- ✅ Complete metrics

See **CIRCUIT-BREAKER.md** for complete documentation.

