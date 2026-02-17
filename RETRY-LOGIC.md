# Retry Logic with Exponential Backoff

## Overview

The Data Plane includes **automatic retry logic** with exponential backoff for handling transient failures. Retries are configured per-flow and work with both execute and trigger endpoints.

## Configuration

### Flow Definition with Retry Policy

```json
{
  "id": "resilient-api",
  "name": "Resilient API",
  "trigger": {
    "type": "http",
    "path": "/api/resilient",
    "method": "GET"
  },
  "steps": [...],
  "retry": {
    "max_attempts": 3,
    "initial_delay_ms": 100,
    "max_delay_ms": 5000,
    "backoff_multiplier": 2.0,
    "jitter": true
  }
}
```

### Policy Fields

```typescript
interface RetryPolicy {
  max_attempts: number;         // Total attempts (including initial)
  initial_delay_ms: number;     // First retry delay (ms)
  max_delay_ms: number;         // Maximum delay cap (ms)
  backoff_multiplier: number;   // Exponential multiplier (default: 2.0)
  jitter: boolean;              // Add randomness (default: false)
}
```

### Configuration Examples

#### Conservative (Quick retries, few attempts)
```json
{
  "retry": {
    "max_attempts": 2,
    "initial_delay_ms": 50,
    "max_delay_ms": 500,
    "backoff_multiplier": 2.0,
    "jitter": false
  }
}
```
**Delays:** 50ms  
**Total time:** ~50ms  
**Use case:** Fast-failing APIs

#### Balanced (Standard retry)
```json
{
  "retry": {
    "max_attempts": 3,
    "initial_delay_ms": 100,
    "max_delay_ms": 5000,
    "backoff_multiplier": 2.0,
    "jitter": true
  }
}
```
**Delays:** 50-100ms, 100-200ms  
**Total time:** ~150-300ms  
**Use case:** Most flows

#### Aggressive (Many retries, long waits)
```json
{
  "retry": {
    "max_attempts": 5,
    "initial_delay_ms": 1000,
    "max_delay_ms": 30000,
    "backoff_multiplier": 2.0,
    "jitter": true
  }
}
```
**Delays:** 500-1000ms, 1000-2000ms, 2000-4000ms, 4000-8000ms  
**Total time:** ~7.5-15 seconds  
**Use case:** Critical operations, eventual consistency

## Exponential Backoff Calculation

```
delay = initial_delay * (backoff_multiplier ^ attempt)
capped_delay = min(delay, max_delay)

With jitter:
actual_delay = capped_delay * random(0.5 to 1.0)
```

**Example with policy:**
```json
{
  "initial_delay_ms": 100,
  "max_delay_ms": 5000,
  "backoff_multiplier": 2.0,
  "jitter": false
}
```

| Attempt | Calculation | Delay | Total Wait |
|---------|-------------|-------|------------|
| 1 (initial) | - | 0ms | 0ms |
| 2 | 100 × 2^0 | 100ms | 100ms |
| 3 | 100 × 2^1 | 200ms | 300ms |
| 4 | 100 × 2^2 | 400ms | 700ms |
| 5 | 100 × 2^3 | 800ms | 1500ms |
| 6 | 100 × 2^4 | 1600ms | 3100ms |

## Retry Metrics

### Available Metrics

```prometheus
# Total retry attempts across all flows
retry_attempts_total

# Successful retries (eventually succeeded)
retry_success_total

# Exhausted retries (all attempts failed)
retry_exhausted_total
```

### PromQL Queries

```promql
# Retry attempt rate
rate(retry_attempts_total[5m])

# Retry success rate
rate(retry_success_total[5m]) / rate(retry_exhausted_total[5m] + retry_success_total[5m])

# Percentage of executions requiring retry
rate(retry_attempts_total[5m]) / rate(flow_executions_total[5m]) * 100
```

## Complete Example

### 1. Create Flow with Retry

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "flaky-service",
    "name": "Flaky External Service",
    "trigger": {
      "type": "http",
      "path": "/api/flaky-service",
      "method": "GET"
    },
    "steps": [
      {
        "type": "call",
        "name": "external_api",
        "connector": "http",
        "operation": "get",
        "params": {
          "url": "https://httpstat.us/Random/200,503,503"
        }
      }
    ],
    "retry": {
      "max_attempts": 3,
      "initial_delay_ms": 100,
      "max_delay_ms": 1000,
      "backoff_multiplier": 2.0,
      "jitter": true
    }
  }'
```

### 2. Execute and Observe Retries

**Via Execute Endpoint:**
```bash
curl -X POST http://localhost:8080/flows/flaky-service/execute -d '{}'
```

**Via Trigger Endpoint:**
```bash
curl http://localhost:8080/api/trigger/flaky-service
```

**Logs (with retries):**
```
📨 Executing flow: flaky-service
🔄 Flow flaky-service failed on attempt 1/3, retrying in 87ms: HTTP 503
🔄 Flow flaky-service failed on attempt 2/3, retrying in 156ms: HTTP 503
✅ Flow flaky-service succeeded on attempt 3/3
✅ Flow flaky-service completed in 0.456s
```

**Logs (exhausted):**
```
📨 Executing flow: flaky-service
🔄 Flow flaky-service failed on attempt 1/3, retrying in 92ms: HTTP 503
🔄 Flow flaky-service failed on attempt 2/3, retrying in 184ms: HTTP 503
❌ Flow flaky-service failed after 3 attempts: HTTP 503
❌ Flow flaky-service failed after 0.389s: HTTP 503
```

### 3. Check Retry Metrics

```bash
curl http://localhost:8080/metrics | grep retry

# Output:
# retry_attempts_total 5
# retry_success_total 2
# retry_exhausted_total 1
```

## Retry with Other Patterns

### Retry + Circuit Breaker

```json
{
  "retry": {
    "max_attempts": 3,
    "initial_delay_ms": 100,
    "max_delay_ms": 1000,
    "backoff_multiplier": 2.0
  },
  "circuit_breaker": {
    "failure_threshold": 5,
    "timeout_seconds": 30,
    "success_threshold": 2
  }
}
```

**Behavior:**
1. Request fails → Retry up to 3 times
2. All retries fail → Counts as 1 failure for circuit breaker
3. After 5 such failures → Circuit opens
4. Circuit open → No retries attempted (immediate 503)

### Retry + Rate Limiting

```json
{
  "retry": {
    "max_attempts": 3,
    "initial_delay_ms": 200,
    "max_delay_ms": 2000,
    "backoff_multiplier": 2.0
  },
  "rate_limit": {
    "max_requests": 10,
    "window_seconds": 60,
    "key_type": "per_ip"
  }
}
```

**Behavior:**
1. Rate limit check → If exceeded, reject (no retry)
2. If allowed → Execute with retry
3. Retry attempts don't count against rate limit separately

## Jitter Explained

**Without Jitter:**
```
Retry 1: 100ms
Retry 2: 200ms
Retry 3: 400ms
```
All clients retry at exact same time → Thundering herd

**With Jitter:**
```
Retry 1: 50-100ms (random)
Retry 2: 100-200ms (random)
Retry 3: 200-400ms (random)
```
Clients spread out → Reduced load spikes

## Best Practices

### 1. Choose Appropriate Max Attempts
- **Idempotent operations:** 3-5 attempts
- **Non-idempotent:** 1-2 attempts
- **Critical operations:** 5+ attempts

### 2. Set Reasonable Delays
- **Initial delay:** 50-500ms
- **Max delay:** 1-30 seconds
- Consider user experience

### 3. Use Jitter for Shared Resources
- Always use jitter for:
  - Database connections
  - External APIs
  - Shared rate-limited services

### 4. Combine with Circuit Breaker
```json
{
  "retry": {
    "max_attempts": 3,
    "initial_delay_ms": 100
  },
  "circuit_breaker": {
    "failure_threshold": 5,
    "timeout_seconds": 30
  }
}
```

### 5. Monitor Retry Metrics
- High retry rate → Upstream issues
- Low success rate → Need circuit breaker
- Exhausted retries → Increase max_delay

## Endpoint Coverage

Retry logic works with **both** endpoints:

1. **Direct Execution:** `POST /flows/:flow_id/execute`
2. **HTTP Trigger:** `GET /api/trigger/:path`

Both endpoints:
- ✅ Respect retry policy
- ✅ Log retry attempts
- ✅ Track retry metrics
- ✅ Use exponential backoff
- ✅ Support jitter

## Testing

### Test Script

```bash
./test-retry.sh
```

### Manual Test

```bash
# Create flow with retry
curl -X POST http://localhost:8081/flows -d '{
  "id": "retry-test",
  "name": "Retry Test",
  "trigger": {"type": "http", "path": "/api/retry-test", "method": "GET"},
  "steps": [
    {
      "type": "call",
      "name": "flaky_call",
      "connector": "http",
      "operation": "get",
      "params": {"url": "https://httpstat.us/Random/200,503,503"}
    }
  ],
  "retry": {
    "max_attempts": 4,
    "initial_delay_ms": 100,
    "max_delay_ms": 2000,
    "backoff_multiplier": 2.0,
    "jitter": true
  }
}'

# Execute multiple times
for i in {1..10}; do
  echo "Request $i:"
  curl http://localhost:8080/api/trigger/retry-test
  echo ""
done

# Check metrics
curl http://localhost:8080/metrics | grep retry
```

## Monitoring & Alerting

### Grafana Dashboard

**Retry Rate:**
```promql
rate(retry_attempts_total[5m])
```

**Retry Success Rate:**
```promql
rate(retry_success_total[5m]) / 
(rate(retry_success_total[5m]) + rate(retry_exhausted_total[5m])) * 100
```

**Flows Requiring Retries:**
```promql
rate(retry_attempts_total[5m]) / rate(flow_executions_total[5m]) * 100
```

### Alert Rules

```yaml
groups:
  - name: retry_alerts
    rules:
      # High retry rate
      - alert: HighRetryRate
        expr: rate(retry_attempts_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High retry rate detected"
          description: "{{ $value }} retries/sec"

      # Low retry success rate
      - alert: LowRetrySuccessRate
        expr: |
          rate(retry_success_total[5m]) / 
          (rate(retry_success_total[5m]) + rate(retry_exhausted_total[5m])) < 0.5
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "Retry success rate below 50%"
          description: "Consider circuit breaker or investigation"
```

## Summary

### What Was Implemented

1. ✅ **Exponential Backoff** - Increasing delays between retries
2. ✅ **Jitter Support** - Randomization to prevent thundering herd
3. ✅ **Configurable Policy** - Per-flow retry configuration
4. ✅ **Max Delay Cap** - Prevent excessive wait times
5. ✅ **Metrics** - Track attempts, successes, exhaustions
6. ✅ **Both Endpoints** - Execute and trigger support
7. ✅ **Logging** - Detailed retry attempt logs

### Retry Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `retry_attempts_total` | Counter | Total retry attempts |
| `retry_success_total` | Counter | Retries that succeeded |
| `retry_exhausted_total` | Counter | Retries that exhausted |

### Complete Protection Stack

```
Client Request
    ↓
Circuit Breaker → Open? → 503
    ↓ Closed
Rate Limiter → Exceeded? → 429
    ↓ Allowed
Flow Execution (with Retry)
    ├─ Attempt 1 → Fail
    ├─ Wait (backoff + jitter)
    ├─ Attempt 2 → Fail
    ├─ Wait (backoff × 2 + jitter)
    └─ Attempt 3 → Success ✅
```

**Your flows now have automatic retry with intelligent backoff!** 🔄
