# Integration Runtime

The Integration Runtime (`crates/integration-runtime`) is a library crate that implements all connectors and the flow execution engine. It is used by the Data Plane.

## Connector Trait

Every connector implements the `Connector` trait:

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    async fn execute(
        &self,
        operation: &str,
        params: &serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, ConnectorError>;
}
```

## Supported Connectors

| Connector | Module | Operations |
|-----------|--------|-----------|
| HTTP | `connectors/http` | `get`, `post`, `put`, `delete`, `patch`, `oauth_token` |
| PostgreSQL | `connectors/postgres` | `query`, `execute` |
| MySQL | `connectors/mysql` | `query`, `execute` |
| MSSQL | `connectors/mssql` | `query`, `execute` |
| Oracle | `connectors/oracle` | `query`, `execute` |
| AWS S3 | `connectors/aws/s3` | `get_object`, `put_object`, `list_objects`, `delete_object` |

## Graph Executor

Flows are represented as a DAG (Directed Acyclic Graph). The Graph Executor:

1. Topologically sorts steps
2. Runs independent steps in parallel using `tokio::spawn`
3. Passes the output of each step as input to dependent steps via the `ExecutionContext`
4. Collects and merges results

```
trigger
   │
   ├──► step_a (independent)
   │
   └──► step_b (independent)
            │
            ▼
         step_c (depends on step_b output)
```

## Loop Executor

The Loop Executor wraps a sub-flow and executes it once per item in a collection:

```json
{
  "type": "loop",
  "name": "process_rows",
  "items": "{{fetch_data.rows}}",
  "steps": [
    { "type": "call", "connector": "postgres", "operation": "execute",
      "params": { "sql": "INSERT INTO ...", "params": ["{{item.id}}"] } }
  ]
}
```

## Execution Context

The `ExecutionContext` holds:

- Trigger input (HTTP body, headers, query params)
- Accumulated step outputs (referenced via `{{step_name.field}}`)
- Flow metadata (flow ID, project ID, run ID)

Template variables (`{{...}}`) are resolved from the context at execution time using a lightweight expression resolver.
