# Circuit Breaker Pattern

## Overview

The Data Plane now includes a **Circuit Breaker pattern** to prevent cascading failures and provide graceful degradation when flows are failing repeatedly.

## How It Works

```
┌─────────────────────────────────────────────────────────┐
│                  Circuit Breaker States                  │
│                                                          │
│  ┌────────┐   Failures ≥ Threshold    ┌────────┐      │
│  │ CLOSED │──────────────────────────►│  OPEN  │      │
│  └────────┘                             └────────┘      │
│      │ ▲                                    │           │
│      │ │ Success × N                        │           │
│      │ │                          Timeout   │           │
│      │ │                          Expires   │           │
│      │ │                                    ▼           │
│      │ │                              ┌──────────┐     │
│      │ └──────────────────────────────│HALF-OPEN │     │
│      │                                └──────────┘     │
│      │                                                  │
│      └─ Requests allowed                               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### States

**1. CLOSED (Normal Operation)**
- All requests allowed
- Tracks failures
- Opens when failure threshold reached

**2. OPEN (Failing)**
- All requests rejected immediately
- Returns HTTP 503 (Service Unavailable)
- After timeout, transitions to HALF-OPEN

**3. HALF-OPEN (Testing)**
- Limited requests allowed
- If successes reach threshold → CLOSED
- If any failure → OPEN

## Configuration

### Flow Definition with Circuit Breaker

```json
{
  "id": "external-api-flow",
  "name": "External API Flow",
  "trigger": {
    "type": "http",
    "path": "/api/external",
    "method": "GET"
  },
  "steps": [...],
  "circuit_breaker": {
    "failure_threshold": 5,
    "window_seconds": 60,
    "timeout_seconds": 30,
    "success_threshold": 3
  }
}
```

### Policy Fields

```typescript
interface CircuitBreakerPolicy {
  failure_threshold: number;    // Failures before opening (e.g., 5)
  window_seconds: number;       // Time window to track failures (e.g., 60)
  timeout_seconds: number;      // Time before trying half-open (e.g., 30)
  success_threshold: number;    // Successes to close from half-open (e.g., 3)
}
```

### Configuration Examples

#### Conservative (Quick to open, slow to close)
```json
{
  "circuit_breaker": {
    "failure_threshold": 3,
    "window_seconds": 30,
    "timeout_seconds": 60,
    "success_threshold": 5
  }
}
```
**Use case:** Critical external APIs, prefer fail-fast

#### Balanced (Standard protection)
```json
{
  "circuit_breaker": {
    "failure_threshold": 5,
    "window_seconds": 60,
    "timeout_seconds": 30,
    "success_threshold": 3
  }
}
```
**Use case:** Most flows, balance between protection and availability

#### Lenient (Slow to open, quick to close)
```json
{
  "circuit_breaker": {
    "failure_threshold": 10,
    "window_seconds": 120,
    "timeout_seconds": 15,
    "success_threshold": 2
  }
}
```
**Use case:** Internal services, transient issues expected

## Circuit Breaker Metrics

### Available Metrics

```prometheus
# Circuit breaker state (0=closed, 1=open, 2=half_open)
circuit_breaker_state{flow_id="my-flow"} 0

# Total times circuit opened
circuit_breaker_opens_total 15

# Total times circuit closed
circuit_breaker_closes_total 12

# Total times circuit went half-open
circuit_breaker_half_opens_total 12

# Total requests rejected due to open circuit
circuit_breaker_rejected_total 342
```

### PromQL Queries

```promql
# Current circuit breaker states
circuit_breaker_state

# Open circuits
circuit_breaker_state == 1

# Circuit open rate
rate(circuit_breaker_opens_total[5m])

# Circuit rejection rate
rate(circuit_breaker_rejected_total[5m])

# Circuit health (closes vs opens)
rate(circuit_breaker_closes_total[5m]) / rate(circuit_breaker_opens_total[5m])
```

## Complete Example

### 1. Create Flow with Circuit Breaker

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "flaky-external-api",
    "name": "Flaky External API",
    "trigger": {
      "type": "http",
      "path": "/api/flaky",
      "method": "GET"
    },
    "steps": [
      {
        "type": "call",
        "name": "external_call",
        "connector": "http",
        "operation": "get",
        "params": {
          "url": "https://httpstat.us/Random/200,503"
        }
      }
    ],
    "circuit_breaker": {
      "failure_threshold": 3,
      "window_seconds": 60,
      "timeout_seconds": 20,
      "success_threshold": 2
    }
  }'
```

### 2. Execute Flow - Trigger Failures

**Via Execute Endpoint:**
```bash
# Direct execution
curl -X POST http://localhost:8080/flows/flaky-external-api/execute -d '{}'
```

**Via Trigger Endpoint:**
```bash
# HTTP trigger
curl http://localhost:8080/api/trigger/flaky
```

**Both endpoints respect circuit breaker state!**

Execute multiple times to trigger failures:

```bash
# Using execute endpoint
for i in {1..10}; do
  curl -X POST http://localhost:8080/flows/flaky-external-api/execute -d '{}'
  sleep 1
done

# Or using trigger endpoint
for i in {1..10}; do
  curl http://localhost:8080/api/trigger/flaky
  sleep 1
done
```

**Possible Outputs:**

```
Request 1:
{"status": "completed", ...}
HTTP: 200
---
Request 2:
{"error": "..."}
HTTP: 500
---
Request 3:
{"error": "..."}
HTTP: 500
---
Request 4:
{"error": "..."}
HTTP: 500
---
Request 5:
{"error": "Circuit breaker is open", "state": "open", "retry_after_seconds": 20}
HTTP: 503
---
```

### 3. Check Circuit Breaker Status

```bash
curl http://localhost:8080/circuit-breakers | jq '.'

# Response:
{
  "circuit_breakers": [
    {
      "flow_id": "flaky-external-api",
      "state": "open",
      "failure_count": 3,
      "success_count": 0,
      "policy": {
        "failure_threshold": 3,
        "window_seconds": 60,
        "timeout_seconds": 20,
        "success_threshold": 2
      }
    }
  ],
  "timestamp": "2024-02-14T..."
}
```

### 4. Wait for Half-Open

After `timeout_seconds`, the circuit enters HALF-OPEN:

```bash
# After 20 seconds...
curl -X POST http://localhost:8080/flows/flaky-external-api/execute -d '{}'

# If successful:
# - success_count increments
# - If success_count reaches success_threshold → CLOSED

# If fails:
# - Circuit reopens immediately
```

### 5. Check Metrics

```bash
curl http://localhost:8080/metrics | grep circuit_breaker

# Output:
# circuit_breaker_state{flow_id="flaky-external-api"} 1
# circuit_breaker_opens_total 1
# circuit_breaker_closes_total 0
# circuit_breaker_half_opens_total 0
# circuit_breaker_rejected_total 5
```

## Error Responses

### Circuit Open (503)

```json
{
  "error": "Circuit breaker is open - service temporarily unavailable",
  "flow_id": "flaky-external-api",
  "state": "open",
  "retry_after_seconds": 15
}
```

**HTTP Status:** `503 Service Unavailable`

## Monitoring & Alerting

### Grafana Dashboard Panels

**1. Circuit Breaker States**
```promql
circuit_breaker_state
```
**Visualization:** Stat panel (0=Green, 1=Red, 2=Yellow)

**2. Open Circuits Count**
```promql
count(circuit_breaker_state == 1)
```

**3. Rejection Rate**
```promql
rate(circuit_breaker_rejected_total[5m])
```

**4. Opens vs Closes**
```promql
rate(circuit_breaker_opens_total[5m])
rate(circuit_breaker_closes_total[5m])
```

### Alert Rules

```yaml
groups:
  - name: circuit_breaker
    rules:
      # Circuit opened
      - alert: CircuitBreakerOpen
        expr: circuit_breaker_state == 1
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "Circuit breaker open for {{ $labels.flow_id }}"
          description: "Flow {{ $labels.flow_id }} circuit breaker is OPEN"

      # High rejection rate
      - alert: HighCircuitBreakerRejections
        expr: rate(circuit_breaker_rejected_total[5m]) > 10
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High circuit breaker rejection rate"
          description: "{{ $value }} requests/sec rejected"

      # Circuit frequently opening
      - alert: FrequentCircuitOpens
        expr: rate(circuit_breaker_opens_total[15m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Circuit breaker opening frequently"
          description: "Circuit opening {{ $value }} times/sec"
```

## Testing Circuit Breaker

### Endpoint Coverage

Circuit breaker works with **both** execution methods:

1. **Direct Execution:** `POST /flows/:flow_id/execute`
2. **HTTP Trigger:** `GET /api/trigger/:path`

Both endpoints:
- ✅ Check circuit breaker state before execution
- ✅ Return 503 when circuit is open
- ✅ Update circuit breaker state based on results
- ✅ Track failures and successes
- ✅ Support automatic recovery

### Test Script

```bash
# Test circuit breaker with both endpoints
./test-circuit-breaker-trigger.sh
```

This script verifies:
- ✅ Circuit breaker works with execute endpoint
- ✅ Circuit breaker works with trigger endpoint
- ✅ Both endpoints respect open circuit
- ✅ State transitions work correctly
- ✅ Metrics tracked for both paths

### Scenario 1: Trigger Circuit Open

```bash
# Create flow that always fails
curl -X POST http://localhost:8081/flows -d '{
  "id": "always-fails",
  "name": "Always Fails",
  "trigger": {"type": "http", "path": "/api/fail", "method": "GET"},
  "steps": [
    {
      "type": "call",
      "name": "bad_call",
      "connector": "http",
      "operation": "get",
      "params": {"url": "https://httpstat.us/500"}
    }
  ],
  "circuit_breaker": {
    "failure_threshold": 3,
    "window_seconds": 60,
    "timeout_seconds": 10,
    "success_threshold": 2
  }
}'

sleep 3

# Execute 5 times - first 3 fail, rest rejected
for i in {1..5}; do
  echo "Request $i"
  curl -s http://localhost:8080/flows/always-fails/execute -d '{}' | jq '.error'
done

# Check status
curl http://localhost:8080/circuit-breakers | jq '.circuit_breakers[] | select(.flow_id=="always-fails")'
```

### Scenario 2: Circuit Recovery

```bash
# Create flow with intermittent failures
# Wait for circuit to open, then for timeout
# Successful requests in half-open should close circuit

# Monitor state transitions in logs:
docker-compose logs -f data-plane | grep "Circuit breaker"

# Expected logs:
# 🔴 Circuit breaker OPEN for flow: always-fails (failures: 3)
# 🔄 Circuit breaker HALF-OPEN for flow: always-fails
# ✅ Circuit breaker CLOSED for flow: always-fails
```

## Benefits

### 1. Prevent Cascading Failures
- Stop calling failing services
- Reduce load on struggling dependencies
- Prevent system-wide outages

### 2. Fast Fail
- Immediate rejection (no timeout wait)
- Better user experience
- Lower resource usage

### 3. Automatic Recovery
- Self-healing when service recovers
- No manual intervention needed
- Gradual traffic restoration

### 4. Observable
- Real-time state visibility
- Detailed metrics
- Alert integration

### 5. Configurable
- Per-flow policies
- Tunable thresholds
- Adaptive protection

## Best Practices

### 1. Tune Thresholds
- Start conservative
- Adjust based on metrics
- Consider SLAs

### 2. Set Appropriate Timeouts
- Too short: Unnecessary opens
- Too long: Prolonged failures
- Match service recovery time

### 3. Monitor Metrics
- Watch open/close patterns
- Alert on frequent opens
- Track rejection rates

### 4. Combine with Rate Limiting
```json
{
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

### 5. Use for External Dependencies
- Third-party APIs
- Database calls
- Microservices
- Remote systems

## API Reference

### GET /circuit-breakers

Returns current state of all circuit breakers.

```bash
curl http://localhost:8080/circuit-breakers
```

**Response:**
```json
{
  "circuit_breakers": [
    {
      "flow_id": "external-api",
      "state": "closed",
      "failure_count": 0,
      "success_count": 0,
      "policy": {
        "failure_threshold": 5,
        "window_seconds": 60,
        "timeout_seconds": 30,
        "success_threshold": 3
      }
    }
  ],
  "timestamp": "2024-02-14T10:30:00Z"
}
```

## Summary

### What Was Implemented

1. ✅ **Circuit Breaker Middleware** - Checks state before execution
2. ✅ **State Machine** - CLOSED → OPEN → HALF-OPEN → CLOSED
3. ✅ **Per-Flow Policies** - Configurable thresholds
4. ✅ **Automatic Recovery** - Self-healing with half-open testing
5. ✅ **Metrics** - 5 new Prometheus metrics
6. ✅ **Status API** - `/circuit-breakers` endpoint
7. ✅ **HTTP 503 Responses** - Standard error handling

### Circuit Breaker Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `circuit_breaker_state` | Gauge | Current state (0/1/2) |
| `circuit_breaker_opens_total` | Counter | Times opened |
| `circuit_breaker_closes_total` | Counter | Times closed |
| `circuit_breaker_half_opens_total` | Counter | Times half-opened |
| `circuit_breaker_rejected_total` | Counter | Rejected requests |

### Complete Protection Stack

```
Client Request
    ↓
Circuit Breaker → Check state → Allow/Reject
    ↓
Rate Limiter → Check limits → Allow/Block
    ↓
Flow Execution → Success/Failure
    ↓
Update Circuit Breaker State
```

**Your flows are now protected with circuit breakers!** 🔌
