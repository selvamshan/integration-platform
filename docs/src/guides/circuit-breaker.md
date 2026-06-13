# Circuit Breaker

The circuit breaker protects downstream connectors from cascading failures.

## States

```
CLOSED ──(failure threshold exceeded)──► OPEN
  ▲                                        │
  │                                  (timeout elapses)
  │                                        ▼
  └────(probe succeeds)──────────── HALF-OPEN
```

| State | Behavior |
|-------|----------|
| **Closed** | Requests pass through normally |
| **Open** | Requests fail immediately (fast-fail) without calling the connector |
| **Half-Open** | One probe request is allowed; if it succeeds, the breaker closes |

## Configuration

Per-connector circuit breaker settings:

```json
{
  "id": "flaky_api",
  "connector_type": "http",
  "circuit_breaker": {
    "failure_threshold": 5,
    "success_threshold": 2,
    "timeout_secs": 30
  }
}
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `failure_threshold` | 5 | Consecutive failures before opening |
| `success_threshold` | 2 | Consecutive successes to close from half-open |
| `timeout_secs` | 30 | Seconds to wait in open state before probing |

## Error Response

When the circuit is open:

```json
{
  "error": "Circuit breaker open for connector: flaky_api",
  "retry_after_secs": 25
}
```

## Monitoring

The Prometheus metrics endpoint exposes circuit breaker state changes:

```
circuit_breaker_opens_total{connector="flaky_api"} 3
```
