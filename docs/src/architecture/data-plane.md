# Data Plane

The Data Plane (`crates/data-plane`, port 8080) executes flows in response to HTTP triggers.

## Responsibilities

- **HTTP Trigger Handling** — Expose `GET /api/trigger/:flow_path` endpoints
- **Flow Caching** — Keep an in-memory copy of all flows synced from the Control Plane
- **Flow Execution** — Delegate to Integration Runtime for step-by-step execution
- **Rate Limiting** — Enforce per-IP, per-user, and per-flow limits via Redis
- **Circuit Breaker** — Protect downstream connectors from cascading failures
- **Prometheus Metrics** — Expose `/metrics` for scraping

## Flow Sync via NATS

When a flow is created or updated in the Control Plane, a `flow.sync` NATS message is published. The Data Plane subscribes and refreshes its cache:

```
Control Plane → NATS (flow.sync) → Data Plane cache refresh
```

On startup, the Data Plane fetches all existing flows from the Control Plane REST API to warm its cache.

## HTTP Trigger Endpoint

```
GET  /api/trigger/:path
POST /api/trigger/:path
```

The Data Plane matches `:path` to a flow's trigger configuration and executes it. The response is the output of the last step in the flow.

## Rate Limiting

Rate limits are checked before flow execution using a Redis sliding-window counter. Limits are configurable per:

- IP address
- Authenticated user
- Flow ID

When a limit is exceeded, the endpoint returns `429 Too Many Requests`.

## Circuit Breaker

Each connector call is wrapped in a circuit breaker. After a configurable number of consecutive failures, the breaker opens and immediately returns an error (fast-fail) until the connector recovers.

States: **Closed** → **Open** → **Half-Open** → **Closed**

## Metrics

Available at `GET /metrics` (Prometheus format):

| Metric | Type | Description |
|--------|------|-------------|
| `flow_executions_total` | Counter | Total flow executions |
| `flow_execution_duration_seconds` | Histogram | Execution latency |
| `connector_calls_total` | Counter | Total connector calls by type |
| `rate_limit_rejections_total` | Counter | Requests rejected by rate limiter |
| `circuit_breaker_opens_total` | Counter | Circuit breaker open events |
