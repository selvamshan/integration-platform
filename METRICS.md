# Metrics Layer - Prometheus Integration

## Overview

The Data Plane now exposes **Prometheus metrics** for comprehensive monitoring and observability. All metrics are available at the `/metrics` endpoint in Prometheus exposition format.

## Metrics Endpoint

```
GET http://localhost:8080/metrics
```

**Response Format:** Prometheus text format
**Content-Type:** `text/plain; version=0.0.4; charset=utf-8`

## Available Metrics

### 1. HTTP Request Metrics

#### `http_requests_total`
**Type:** Counter  
**Description:** Total number of HTTP requests received  
**Use Case:** Overall request volume monitoring

```prometheus
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total 12345
```

#### `http_request_duration_seconds`
**Type:** Histogram  
**Description:** HTTP request duration in seconds  
**Buckets:** 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0  
**Use Case:** Request latency analysis, SLA monitoring

```prometheus
# HELP http_request_duration_seconds HTTP request duration in seconds
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le="0.005"} 150
http_request_duration_seconds_bucket{le="0.01"} 280
http_request_duration_seconds_bucket{le="0.05"} 450
http_request_duration_seconds_bucket{le="+Inf"} 500
http_request_duration_seconds_sum 45.2
http_request_duration_seconds_count 500
```

### 2. Flow Execution Metrics

#### `flow_executions_total`
**Type:** Counter  
**Description:** Total number of flow executions attempted  
**Use Case:** Flow usage tracking

```prometheus
flow_executions_total 8942
```

#### `flow_executions_success_total`
**Type:** Counter  
**Description:** Total number of successful flow executions  
**Use Case:** Success rate calculation

```prometheus
flow_executions_success_total 8850
```

#### `flow_executions_failed_total`
**Type:** Counter  
**Description:** Total number of failed flow executions  
**Use Case:** Error rate monitoring

```prometheus
flow_executions_failed_total 92
```

**Success Rate Formula:**
```
success_rate = flow_executions_success_total / flow_executions_total * 100
```

#### `flow_execution_duration_seconds`
**Type:** Histogram  
**Description:** Flow execution duration in seconds  
**Buckets:** 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0  
**Use Case:** Flow performance monitoring

```prometheus
flow_execution_duration_seconds_bucket{le="0.1"} 120
flow_execution_duration_seconds_bucket{le="0.5"} 450
flow_execution_duration_seconds_bucket{le="1.0"} 780
flow_execution_duration_seconds_sum 523.4
flow_execution_duration_seconds_count 800
```

### 3. Rate Limiting Metrics

#### `rate_limit_checks_total`
**Type:** Counter  
**Description:** Total number of rate limit checks performed  
**Use Case:** Rate limit system usage

```prometheus
rate_limit_checks_total 15430
```

#### `rate_limit_blocked_total`
**Type:** Counter  
**Description:** Total number of requests blocked by rate limiting  
**Use Case:** Rate limit effectiveness

```prometheus
rate_limit_blocked_total 234
```

#### `rate_limit_allowed_total`
**Type:** Counter  
**Description:** Total number of requests allowed after rate limit check  
**Use Case:** Allowed traffic tracking

```prometheus
rate_limit_allowed_total 15196
```

**Block Rate Formula:**
```
block_rate = rate_limit_blocked_total / rate_limit_checks_total * 100
```

### 4. System Metrics

#### `flows_loaded`
**Type:** Gauge  
**Description:** Number of flows currently loaded in memory  
**Use Case:** System state monitoring

```prometheus
flows_loaded 45
```

#### `redis_operations_total`
**Type:** Counter  
**Description:** Total number of Redis operations  
**Use Case:** Redis usage tracking

```prometheus
redis_operations_total 30862
```

#### `redis_errors_total`
**Type:** Counter  
**Description:** Total number of Redis errors  
**Use Case:** Redis health monitoring

```prometheus
redis_errors_total 3
```

## Prometheus Configuration

### prometheus.yml

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'integration-platform-data-plane'
    static_configs:
      - targets: ['data-plane:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
```

### Docker Compose with Prometheus

```yaml
services:
  prometheus:
    image: prom/prometheus:latest
    container_name: integration-prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    networks:
      - integration-network

  data-plane:
    # ... existing config
    # Metrics available at http://data-plane:8080/metrics

volumes:
  prometheus_data:
```

## Grafana Dashboards

### Sample Dashboard Panels

#### 1. Request Rate
```promql
rate(http_requests_total[5m])
```

#### 2. Request Duration P95
```promql
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

#### 3. Flow Success Rate
```promql
rate(flow_executions_success_total[5m]) 
/ 
rate(flow_executions_total[5m]) * 100
```

#### 4. Flow Error Rate
```promql
rate(flow_executions_failed_total[5m]) 
/ 
rate(flow_executions_total[5m]) * 100
```

#### 5. Average Flow Duration
```promql
rate(flow_execution_duration_seconds_sum[5m]) 
/ 
rate(flow_execution_duration_seconds_count[5m])
```

#### 6. Rate Limit Block Rate
```promql
rate(rate_limit_blocked_total[5m]) 
/ 
rate(rate_limit_checks_total[5m]) * 100
```

#### 7. Redis Error Rate
```promql
rate(redis_errors_total[5m])
```

#### 8. Flows Loaded (Current)
```promql
flows_loaded
```

## Alerting Rules

### alerts.yml

```yaml
groups:
  - name: integration_platform
    interval: 30s
    rules:
      # High error rate
      - alert: HighFlowErrorRate
        expr: |
          rate(flow_executions_failed_total[5m]) 
          / 
          rate(flow_executions_total[5m]) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High flow error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }}"

      # Slow flow execution
      - alert: SlowFlowExecution
        expr: |
          histogram_quantile(0.95, 
            rate(flow_execution_duration_seconds_bucket[5m])
          ) > 5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Slow flow execution (P95 > 5s)"
          description: "P95 latency is {{ $value }}s"

      # High rate limit block rate
      - alert: HighRateLimitBlockRate
        expr: |
          rate(rate_limit_blocked_total[5m]) 
          / 
          rate(rate_limit_checks_total[5m]) > 0.20
        for: 5m
        labels:
          severity: info
        annotations:
          summary: "High rate limiting activity"
          description: "{{ $value | humanizePercentage }} of requests blocked"

      # Redis errors
      - alert: RedisErrors
        expr: rate(redis_errors_total[5m]) > 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Redis errors detected"
          description: "{{ $value }} errors per second"

      # No flows loaded
      - alert: NoFlowsLoaded
        expr: flows_loaded == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "No flows loaded in Data Plane"
          description: "Data Plane has 0 flows loaded"
```

## Testing Metrics

### 1. Access Metrics Endpoint

```bash
curl http://localhost:8080/metrics

# Sample output:
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
# http_requests_total 150
#
# HELP flow_executions_total Total number of flow executions
# TYPE flow_executions_total counter
# flow_executions_total 45
#
# HELP flow_execution_duration_seconds Flow execution duration in seconds
# TYPE flow_execution_duration_seconds histogram
# flow_execution_duration_seconds_bucket{le="0.1"} 10
# flow_execution_duration_seconds_bucket{le="0.5"} 35
# ...
```

### 2. Generate Load and Check Metrics

```bash
# Execute flow multiple times
for i in {1..100}; do
  curl -s -X POST http://localhost:8080/flows/my-flow/execute -d '{}' > /dev/null
done

# Check metrics
curl http://localhost:8080/metrics | grep flow_executions
```

### 3. Specific Metric Queries

```bash
# Count total requests
curl -s http://localhost:8080/metrics | grep "^http_requests_total"

# Get flow success rate data
curl -s http://localhost:8080/metrics | grep "^flow_executions"

# Check rate limiting metrics
curl -s http://localhost:8080/metrics | grep "^rate_limit"
```

## Monitoring Best Practices

### 1. Key Metrics to Monitor

**RED Method:**
- **R**ate: `rate(http_requests_total[5m])`
- **E**rrors: `rate(flow_executions_failed_total[5m])`
- **D**uration: `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))`

**USE Method:**
- **U**tilization: Flow execution rate
- **S**aturation: Rate limit blocked rate
- **E**rrors: Redis errors, flow failures

### 2. Recommended Alerts

1. **Error Rate > 5%** for 5 minutes → Warning
2. **P95 Latency > 5s** for 10 minutes → Warning
3. **Redis Errors** → Critical
4. **No Flows Loaded** → Critical
5. **Rate Limit Block Rate > 20%** → Info

### 3. Dashboard Layouts

**Overview Dashboard:**
- Request rate (QPS)
- Success rate percentage
- P50/P95/P99 latency
- Active flows count

**Flow Performance Dashboard:**
- Flow execution rate
- Success vs failure ratio
- Execution duration histogram
- Individual flow performance

**Rate Limiting Dashboard:**
- Rate limit checks
- Block rate percentage
- Blocked requests over time
- Top blocked keys

**System Health Dashboard:**
- Redis operations & errors
- Flows loaded
- Memory usage (if available)
- Request throughput

## Integration Examples

### Prometheus + Grafana Stack

```yaml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - ./alerts.yml:/etc/prometheus/alerts.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
    networks:
      - integration-network

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources
    depends_on:
      - prometheus
    networks:
      - integration-network

  alertmanager:
    image: prom/alertmanager:latest
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
    networks:
      - integration-network

volumes:
  prometheus_data:
  grafana_data:
```

## Custom Metrics (Future Enhancement)

To add custom per-flow metrics:

```rust
// In data-plane code
lazy_static! {
    static ref FLOW_EXECUTION_BY_ID: IntCounterVec = IntCounterVec::new(
        Opts::new("flow_executions_by_id", "Flow executions by flow ID"),
        &["flow_id"]
    ).unwrap();
}

// In execute_flow function
FLOW_EXECUTION_BY_ID.with_label_values(&[&flow_id]).inc();
```

## Summary

### Metrics Categories

| Category | Metrics Count | Purpose |
|----------|---------------|---------|
| HTTP | 2 | Request monitoring |
| Flow Execution | 4 | Flow performance |
| Rate Limiting | 3 | Rate limit effectiveness |
| System | 3 | System health |
| **Total** | **12** | **Complete observability** |

### Key Features

- ✅ **Prometheus-compatible** - Standard exposition format
- ✅ **Real-time metrics** - Updated with every request
- ✅ **Histogram support** - Percentile calculations (P50, P95, P99)
- ✅ **Low overhead** - Efficient counter/histogram updates
- ✅ **Production-ready** - Battle-tested Prometheus client

### Quick Access

```bash
# Health check (includes metrics status)
curl http://localhost:8080/health

# Metrics endpoint
curl http://localhost:8080/metrics

# Prometheus UI (if running)
open http://localhost:9090

# Grafana (if running)
open http://localhost:3000
```

**Your Data Plane now has comprehensive observability!** 📊
