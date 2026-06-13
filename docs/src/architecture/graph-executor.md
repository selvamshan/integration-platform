# Graph Executor

The Graph Executor runs flows as a DAG, executing independent steps in parallel.

## Flow Definition Structure

```json
{
  "id": "my-flow",
  "name": "My Flow",
  "trigger": {
    "type": "http",
    "path": "/my-endpoint",
    "method": "POST"
  },
  "steps": [
    {
      "id": "step_a",
      "type": "call",
      "name": "fetch_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE id = $1",
        "params": ["{{trigger.body.user_id}}"]
      }
    },
    {
      "id": "step_b",
      "type": "call",
      "name": "fetch_orders",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM orders WHERE user_id = $1",
        "params": ["{{trigger.body.user_id}}"]
      }
    },
    {
      "id": "step_c",
      "type": "call",
      "name": "notify_slack",
      "depends_on": ["step_a", "step_b"],
      "connector": "slack_webhook",
      "operation": "post",
      "params": {
        "path": "/services/...",
        "body": {
          "text": "User {{fetch_user.name}} has {{fetch_orders.count}} orders"
        }
      }
    }
  ]
}
```

In this example, `step_a` and `step_b` run in parallel. `step_c` waits for both.

## Step Types

| Type | Description |
|------|-------------|
| `call` | Execute a connector operation |
| `loop` | Iterate over a collection and run sub-steps |
| `condition` | Branch based on a boolean expression |
| `transform` | Apply a JSONata / JMESPath transformation |
| `log` | Write a log message (no side effects) |

## Variable Resolution

Step outputs are referenced in subsequent steps using `{{step_name.field}}` syntax:

```json
"params": {
  "sql": "SELECT * FROM orders WHERE user_id = $1",
  "params": ["{{fetch_user.rows[0].id}}"]
}
```

The executor resolves variables from the `ExecutionContext` at runtime before passing params to the connector.

## Error Handling

By default, a step failure stops the flow and returns an error. You can configure per-step retry:

```json
{
  "id": "step_a",
  "retry": {
    "max_attempts": 3,
    "backoff_ms": 1000,
    "backoff_multiplier": 2.0
  }
}
```

This retries up to 3 times with exponential backoff (1 s, 2 s, 4 s).
