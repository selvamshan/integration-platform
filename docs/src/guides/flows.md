# Building Flows

A flow is a JSON document that defines a trigger and a sequence of steps to execute.

## Minimal Flow Example

```json
{
  "id": "hello-world",
  "name": "Hello World",
  "trigger": {
    "type": "http",
    "path": "/hello",
    "method": "GET"
  },
  "steps": [
    {
      "id": "step1",
      "type": "call",
      "name": "fetch_users",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT id, name FROM users LIMIT 10"
      }
    }
  ]
}
```

## Create a Flow

```bash
curl -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d @flow.json
```

## Trigger the Flow

```bash
curl http://localhost:8080/api/trigger/hello
```

## Trigger Types

### HTTP Trigger

```json
{
  "trigger": {
    "type": "http",
    "path": "/my-path",
    "method": "POST"
  }
}
```

Access request data in steps:

| Variable | Value |
|----------|-------|
| `{{trigger.body.field}}` | JSON body field |
| `{{trigger.headers.x-user-id}}` | Request header |
| `{{trigger.query.page}}` | Query parameter |

### Scheduler Trigger

```json
{
  "trigger": {
    "type": "schedule",
    "cron": "0 * * * *"
  }
}
```

Runs the flow every hour.

## Passing Data Between Steps

Step outputs are available to all subsequent steps:

```json
{
  "steps": [
    {
      "id": "step1",
      "name": "get_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": { "sql": "SELECT name FROM users WHERE id = $1", "params": ["42"] }
    },
    {
      "id": "step2",
      "name": "send_email",
      "depends_on": ["step1"],
      "connector": "sendgrid_api",
      "operation": "post",
      "params": {
        "path": "/v3/mail/send",
        "body": { "to": "admin@example.com", "subject": "Hello {{get_user.rows[0].name}}" }
      }
    }
  ]
}
```

## Loop Over a Collection

```json
{
  "id": "loop_step",
  "type": "loop",
  "name": "process_each",
  "items": "{{fetch_data.rows}}",
  "steps": [
    {
      "connector": "postgres_prod",
      "operation": "execute",
      "params": {
        "sql": "UPDATE items SET processed = true WHERE id = $1",
        "params": ["{{item.id}}"]
      }
    }
  ]
}
```

## Retry Configuration

```json
{
  "id": "step1",
  "retry": {
    "max_attempts": 3,
    "backoff_ms": 500,
    "backoff_multiplier": 2.0
  }
}
```

## Parallel Execution

Steps without `depends_on` run in parallel automatically:

```json
{
  "steps": [
    { "id": "a", "name": "fetch_orders",   "connector": "postgres_prod", ... },
    { "id": "b", "name": "fetch_products", "connector": "postgres_prod", ... },
    { "id": "c", "name": "merge_results",  "depends_on": ["a", "b"],    ... }
  ]
}
```

Steps `a` and `b` run concurrently; `c` waits for both.
