# Retry Logic

Steps can be configured with automatic retry using exponential backoff.

## Configuration

```json
{
  "id": "step1",
  "type": "call",
  "connector": "my_api",
  "operation": "post",
  "retry": {
    "max_attempts": 3,
    "backoff_ms": 1000,
    "backoff_multiplier": 2.0,
    "retryable_status_codes": [429, 500, 502, 503, 504]
  }
}
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_attempts` | 1 (no retry) | Total attempts including the first |
| `backoff_ms` | 1000 | Initial wait time in milliseconds |
| `backoff_multiplier` | 2.0 | Multiplier applied each attempt |
| `retryable_status_codes` | `[500,502,503,504]` | HTTP status codes that trigger a retry |

## Backoff Schedule

With `backoff_ms: 1000` and `backoff_multiplier: 2.0`:

| Attempt | Wait before retry |
|---------|------------------|
| 1 | — (immediate) |
| 2 | 1,000 ms |
| 3 | 2,000 ms |
| 4 | 4,000 ms |

## Interaction with Circuit Breaker

Retries count toward the circuit breaker's failure counter. If the breaker opens during retries, subsequent attempts fast-fail immediately rather than waiting for the backoff delay.

## Flow-Level Retry

You can set a default retry policy at the flow level, which applies to all steps without an explicit retry config:

```json
{
  "id": "my-flow",
  "default_retry": {
    "max_attempts": 2,
    "backoff_ms": 500
  },
  "steps": [ ... ]
}
```
