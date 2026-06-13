# Rate Limiting

The Data Plane enforces rate limits using a Redis sliding-window counter.

## Default Limits

| Scope | Default Limit | Window |
|-------|--------------|--------|
| Per IP | 100 req | 60 s |
| Per User | 1,000 req | 60 s |
| Per Flow | 500 req | 60 s |

When a limit is exceeded the endpoint returns:

```
HTTP 429 Too Many Requests
Retry-After: 30
```

## Configuration

Override defaults via environment variables:

```env
RATE_LIMIT_PER_IP=100
RATE_LIMIT_PER_USER=1000
RATE_LIMIT_PER_FLOW=500
RATE_LIMIT_WINDOW_SECS=60
```

Or configure per-flow limits in the flow definition:

```json
{
  "id": "my-flow",
  "rate_limit": {
    "per_ip": 10,
    "per_flow": 100,
    "window_secs": 60
  }
}
```

## Redis Dependency

Rate limiting requires a running Redis instance. Configure via `REDIS_URL`:

```env
REDIS_URL=redis://localhost:6379
```

If Redis is unavailable, the Data Plane logs a warning and allows requests through (fail-open behavior).

## Testing Rate Limits

```bash
# Hit the rate limit quickly
for i in $(seq 1 110); do
  curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/api/trigger/my-flow
done
```

You should see `200` responses followed by `429`.
