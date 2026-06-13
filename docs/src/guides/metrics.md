# Prometheus Metrics

The Data Plane exposes Prometheus metrics at `GET /metrics`.

## Available Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `flow_executions_total` | Counter | `flow_id`, `status` | Total flow executions |
| `flow_execution_duration_seconds` | Histogram | `flow_id` | End-to-end execution latency |
| `connector_calls_total` | Counter | `connector_type`, `operation`, `status` | Connector call count |
| `connector_call_duration_seconds` | Histogram | `connector_type`, `operation` | Connector call latency |
| `rate_limit_rejections_total` | Counter | `scope` | Requests rejected by rate limiter |
| `circuit_breaker_opens_total` | Counter | `connector` | Circuit breaker open events |
| `active_flows` | Gauge | — | Flows currently loaded in cache |

## Prometheus Scrape Config

```yaml
# prometheus.yml
scrape_configs:
  - job_name: integration-platform
    static_configs:
      - targets: ['data-plane:8080']
```

## Example Queries

```promql
# Flow error rate over last 5 minutes
rate(flow_executions_total{status="error"}[5m])
  / rate(flow_executions_total[5m])

# 99th percentile flow execution latency
histogram_quantile(0.99, rate(flow_execution_duration_seconds_bucket[5m]))

# Connector calls by type
sum by (connector_type) (rate(connector_calls_total[5m]))
```

## Grafana Dashboard

Import the bundled dashboard JSON from `building_aid/grafana-dashboard.json` (if present) or create panels using the queries above.
