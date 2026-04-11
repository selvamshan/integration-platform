# Rate Limiting with Redis

## Overview

The integration platform now includes **distributed rate limiting** with Redis persistence. Rate limits are enforced in the Data Plane and monitored in the Control Plane via NATS events.

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                     Client Request                          │
└──────────────────────────┬─────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────┐
│                    Data Plane (Port 8080)                   │
│                                                             │
│  1. Rate Limit Middleware (checks before execution)        │
│     ├─ Extract flow ID from request                        │
│     ├─ Get rate limit policy from flow                     │
│     ├─ Generate key (global/per-ip/per-user/per-flow)      │
│     └─ Check Redis                                          │
│                           │                                 │
│  2. Redis Check          ▼                                 │
│     ├─ INCR counter                                         │
│     ├─ SET expiry (window)                                  │
│     └─ Compare with limit                                   │
│                           │                                 │
│  3. Decision             ▼                                 │
│     ├─ If ALLOWED → Execute flow                            │
│     └─ If BLOCKED → Return 429 Too Many Requests            │
│                           │                                 │
│  4. Publish Event        ▼                                 │
│     └─ Send to NATS: ratelimit.event                        │
│                                                             │
└────────────────────────────────────────────────────────────┘
                           │
                           │ NATS: ratelimit.event
                           ▼
┌────────────────────────────────────────────────────────────┐
│                Control Plane (Port 8081)                    │
│                                                             │
│  1. Rate Limit Event Listener                              │
│     ├─ Receives all rate limit events                      │
│     ├─ Logs blocked requests                                │
│     └─ Stores statistics (last 1000 per flow)              │
│                                                             │
│  2. Statistics API                                          │
│     ├─ GET /rate-limits (all flows)                        │
│     └─ GET /rate-limits/:flow_id (specific flow)           │
│                                                             │
└────────────────────────────────────────────────────────────┘
                           │
                           │ Persistence
                           ▼
                  ┌─────────────────┐
                  │      Redis      │
                  │   (Port 6379)   │
                  │                 │
                  │  Keys:          │
                  │  ratelimit:*    │
                  └─────────────────┘
```

## Rate Limit Policy in Flows

### Flow Definition with Rate Limit

```json
{
  "id": "api-flow",
  "name": "API Flow with Rate Limit",
  "trigger": {
    "type": "http",
    "path": "/api/data",
    "method": "GET"
  },
  "steps": [...],
  "rate_limit": {
    "max_requests": 100,
    "window_seconds": 60,
    "key_type": "per_ip",
    "message": "Rate limit exceeded. Please try again later."
  }
}
```

### Rate Limit Policy Fields

```typescript
interface RateLimitPolicy {
  max_requests: number;      // Maximum requests allowed
  window_seconds: number;    // Time window in seconds
  key_type: KeyType;         // How to identify requesters
  message?: string;          // Custom error message
}

enum KeyType {
  "global",      // Global limit across all requests
  "per_ip",      // Per IP address
  "per_user",    // Per user/API key
  "per_flow"     // Per flow (same as global for single flow)
}
```

### Examples

#### Global Rate Limit
```json
{
  "rate_limit": {
    "max_requests": 1000,
    "window_seconds": 3600,
    "key_type": "global",
    "message": "System rate limit exceeded"
  }
}
```
**Use case:** Protect against overall system overload

#### Per-IP Rate Limit
```json
{
  "rate_limit": {
    "max_requests": 10,
    "window_seconds": 60,
    "key_type": "per_ip",
    "message": "Too many requests from your IP"
  }
}
```
**Use case:** Prevent individual IP abuse

#### Per-User Rate Limit
```json
{
  "rate_limit": {
    "max_requests": 100,
    "window_seconds": 60,
    "key_type": "per_user"
  }
}
```
**Use case:** Fair usage per authenticated user

#### Per-Flow Rate Limit
```json
{
  "rate_limit": {
    "max_requests": 500,
    "window_seconds": 60,
    "key_type": "per_flow"
  }
}
```
**Use case:** Limit specific endpoint usage

## Creating Flows with Rate Limits

### Example 1: Public API with IP-based Limit

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "public-api",
    "name": "Public API",
    "trigger": {
      "type": "http",
      "path": "/api/public",
      "method": "GET"
    },
    "steps": [
      {
        "type": "call",
        "name": "fetch_data",
        "connector": "postgres",
        "operation": "query",
        "params": {
          "sql": "SELECT * FROM public_data LIMIT 100"
        }
      }
    ],
    "rate_limit": {
      "max_requests": 10,
      "window_seconds": 60,
      "key_type": "per_ip",
      "message": "Maximum 10 requests per minute per IP"
    }
  }'
```

### Example 2: Premium API with Global Limit

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "premium-api",
    "name": "Premium API",
    "trigger": {
      "type": "http",
      "path": "/api/premium",
      "method": "POST"
    },
    "steps": [...],
    "rate_limit": {
      "max_requests": 1000,
      "window_seconds": 3600,
      "key_type": "global",
      "message": "System capacity limit reached"
    }
  }'
```

### Example 3: Flow Without Rate Limit

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "internal-flow",
    "name": "Internal Flow",
    "trigger": {
      "type": "http",
      "path": "/internal/process",
      "method": "POST"
    },
    "steps": [...]
  }'
```
**Note:** Flows without `rate_limit` field have no rate limiting

## Testing Rate Limits

### Test 1: Hit Rate Limit

```bash
# Create flow with strict limit
curl -X POST http://localhost:8081/flows -H "Content-Type: application/json" -d '{
  "id": "test-rate-limit",
  "name": "Test Rate Limit",
  "trigger": {"type": "http", "path": "/api/test", "method": "GET"},
  "steps": [{"type": "log", "name": "test", "message": "Testing"}],
  "rate_limit": {
    "max_requests": 3,
    "window_seconds": 60,
    "key_type": "per_ip"
  }
}'

# Wait for flow to sync
sleep 3

# Send requests - first 3 should succeed
for i in {1..3}; do
  echo "Request $i:"
  curl -s http://localhost:8080/flows/test-rate-limit/execute \
    -H "Content-Type: application/json" \
    -d '{}' | jq '.status'
done

# 4th request should be rate limited
echo "Request 4 (should fail):"
curl -s http://localhost:8080/flows/test-rate-limit/execute \
  -H "Content-Type: application/json" \
  -d '{}' | jq '.'
```

**Expected response for 4th request:**
```json
{
  "error": "Rate limit exceeded: 3 requests per 60 seconds",
  "flow_id": "test-rate-limit",
  "limit": 3,
  "window_seconds": 60
}
```

**HTTP Status:** `429 Too Many Requests`

### Test 2: Check Rate Limit Statistics

```bash
# Get all rate limit stats
curl http://localhost:8081/rate-limits | jq '.'

# Response:
{
  "flows": {
    "test-rate-limit": {
      "total_requests": 4,
      "allowed": 3,
      "blocked": 1,
      "block_rate": 25.0
    }
  },
  "timestamp": "2024-02-11T..."
}

# Get specific flow stats
curl http://localhost:8081/rate-limits/test-rate-limit | jq '.'

# Response:
{
  "flow_id": "test-rate-limit",
  "summary": {
    "total_requests": 4,
    "allowed": 3,
    "blocked": 1,
    "block_rate": 25.0
  },
  "by_key": [
    {
      "key": "ratelimit:ip:172.18.0.1:test-rate-limit",
      "allowed": 3,
      "blocked": 1
    }
  ],
  "recent_events": [...]
}
```

### Test 3: Different Key Types

**Per-IP (different IPs get separate limits):**
```bash
# From IP 1 - 3 requests allowed
for i in {1..3}; do curl http://localhost:8080/flows/test/execute -d '{}'; done

# From IP 2 - 3 more requests allowed (separate counter)
# (Simulated - in Docker, all from same IP)
```

**Global (shared limit across all):**
```json
{
  "rate_limit": {
    "max_requests": 10,
    "window_seconds": 60,
    "key_type": "global"
  }
}
```
All requests count toward same limit

## Redis Keys

### Key Format

```
ratelimit:{key_type}:{identifier}:{flow_id}
```

### Examples

```redis
# Global
ratelimit:global:my-flow

# Per-IP
ratelimit:ip:192.168.1.100:my-flow

# Per-User
ratelimit:user:user123:my-flow

# Per-Flow
ratelimit:flow:my-flow
```

### Redis Operations

**Check key:**
```bash
docker exec integration-redis redis-cli GET "ratelimit:global:test-flow"
# Output: "3" (current count)
```

**Check TTL:**
```bash
docker exec integration-redis redis-cli TTL "ratelimit:global:test-flow"
# Output: "45" (seconds remaining)
```

**Reset limit:**
```bash
docker exec integration-redis redis-cli DEL "ratelimit:global:test-flow"
# Output: (integer) 1
```

**View all rate limit keys:**
```bash
docker exec integration-redis redis-cli KEYS "ratelimit:*"
```

## Monitoring

### Control Plane Logs

**Allowed requests:**
```
✅ Rate limit check: flow=public-api, count=5/10
✅ Rate limit check: flow=public-api, count=6/10
```

**Blocked requests:**
```
🚫 Rate limit exceeded: flow=public-api, key=ratelimit:ip:1.2.3.4:public-api, count=11/10
🚫 Rate limit exceeded: flow=public-api, key=ratelimit:ip:5.6.7.8:public-api, count=11/10
```

### Data Plane Logs

**Rate limit passed:**
```
✅ Rate limit check passed for flow public-api (key: ratelimit:ip:1.2.3.4:public-api)
📨 Executing flow: public-api
```

**Rate limit exceeded:**
```
🚫 Rate limit exceeded for flow public-api (key: ratelimit:ip:1.2.3.4:public-api)
```

### Statistics API

**Monitor in real-time:**
```bash
# Watch rate limit stats every 2 seconds
watch -n 2 'curl -s http://localhost:8081/rate-limits | jq ".flows"'
```

## Configuration

### Environment Variables

**Data Plane:**
```bash
REDIS_URL=redis://redis:6379
```

**Control Plane:**
```bash
REDIS_URL=redis://redis:6379
```

### Docker Compose

```yaml
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
```

## Advanced Use Cases

### Dynamic Rate Limits

Update flow rate limits without restart:

```bash
# Update flow with new rate limit
curl -X PUT http://localhost:8081/flows/my-flow \
  -H "Content-Type: application/json" \
  -d '{
    "id": "my-flow",
    "name": "My Flow",
    "trigger": {...},
    "steps": [...],
    "rate_limit": {
      "max_requests": 200,  # Increased from 100
      "window_seconds": 60,
      "key_type": "per_ip"
    }
  }'

# New limit applies immediately!
```

### Burst Protection

Short window for burst protection:

```json
{
  "rate_limit": {
    "max_requests": 5,
    "window_seconds": 1,
    "key_type": "per_ip",
    "message": "Too many requests per second"
  }
}
```

### Tiered Rate Limits

Different flows for different tiers:

```bash
# Free tier
curl -X POST http://localhost:8081/flows -d '{
  "id": "free-api",
  "rate_limit": {"max_requests": 10, "window_seconds": 60, "key_type": "per_ip"}
}'

# Pro tier
curl -X POST http://localhost:8081/flows -d '{
  "id": "pro-api",
  "rate_limit": {"max_requests": 100, "window_seconds": 60, "key_type": "per_user"}
}'

# Enterprise tier (no limit)
curl -X POST http://localhost:8081/flows -d '{
  "id": "enterprise-api"
}'
```

## Benefits

### 1. Distributed
- ✅ Redis-based - works across multiple Data Plane instances
- ✅ Consistent limits across all nodes
- ✅ No single point of failure

### 2. Flexible
- ✅ Per-flow configuration
- ✅ Multiple key types (global, per-ip, per-user, per-flow)
- ✅ Custom error messages

### 3. Observable
- ✅ Real-time events via NATS
- ✅ Statistics API in Control Plane
- ✅ Detailed logging

### 4. Performance
- ✅ Middleware-level (fast rejection)
- ✅ Redis INCR operation (atomic & fast)
- ✅ Non-blocking event publishing

### 5. User-Friendly
- ✅ Clear error messages
- ✅ HTTP 429 status code
- ✅ Includes limit information in response

## Troubleshooting

### Rate Limit Not Working

**Check flow has rate_limit configured:**
```bash
curl http://localhost:8081/flows/my-flow | jq '.rate_limit'
```

**Check Data Plane received flow:**
```bash
curl http://localhost:8080/flows | jq '.flows[] | select(.id=="my-flow") | .rate_limit'
```

**Check Redis connection:**
```bash
docker exec integration-redis redis-cli PING
# Output: PONG
```

### Rate Limit Too Aggressive

**Check current count:**
```bash
docker exec integration-redis redis-cli GET "ratelimit:ip:YOUR_IP:flow-id"
```

**Reset if needed:**
```bash
docker exec integration-redis redis-cli DEL "ratelimit:ip:YOUR_IP:flow-id"
```

### No Statistics Showing

**Check NATS events:**
```bash
docker-compose logs control-plane | grep "ratelimit"
```

**Expected:**
```
✅ Subscribed to ratelimit.event
✅ Rate limit check: flow=...
```

## Summary

### What We Built

1. ✅ **Rate Limit Middleware** in Data Plane
2. ✅ **Redis Persistence** for distributed limiting
3. ✅ **NATS Events** for monitoring
4. ✅ **Statistics API** in Control Plane
5. ✅ **Flexible Policies** per flow

### Key Features

- **Per-flow rate limits** configured in flow definition
- **Multiple key types:** global, per-ip, per-user, per-flow
- **Redis-based** for distributed consistency
- **Event-driven** monitoring via NATS
- **Statistics API** for observability
- **Custom error messages**
- **429 status code** for rate limit errors

### Architecture Highlights

```
Client → Data Plane Middleware → Redis Check → Allow/Block
                                      ↓
                                   NATS Event
                                      ↓
                            Control Plane Monitoring
                                      ↓
                               Statistics API
```

**Rate limiting is now fully integrated into your integration platform!** 🎯
