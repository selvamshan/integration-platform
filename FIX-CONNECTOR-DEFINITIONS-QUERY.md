# Fix Connector Definitions Query

The issue is a mismatch between the table schema and the query.

---

## Current Table Structure

From `initialize_builtin_registry`:

```sql
CREATE TABLE connector_definitions (
    name VARCHAR(255) PRIMARY KEY,
    connector_type VARCHAR(64),
    config JSONB  -- Contains entire ConnectorDefinition as JSON
);
```

The `config` column contains the complete connector definition including:
- id
- name
- connector_type
- description
- icon
- operations
- config_schema
- enabled

---

## Fixed Handler

Replace the handler with this:

```rust
/// GET /connector-definitions
/// Returns available connector types from connector_definitions table
async fn list_connector_definitions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT name, connector_type, config
        FROM connector_definitions
        ORDER BY name
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let definitions: Vec<Value> = rows
        .iter()
        .filter_map(|row| {
            // Parse the JSON config which contains the full connector definition
            row.config.as_ref().and_then(|config| {
                serde_json::from_value(config.clone()).ok()
            })
        })
        .collect();

    Ok(Json(json!({
        "definitions": definitions,
        "count": definitions.len()
    })))
}
```

---

## Alternative: Use In-Memory Registry

Even simpler - use the in-memory registry that's already populated:

```rust
/// GET /connector-definitions
/// Returns available connector types from in-memory registry
async fn list_connector_definitions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let connectors = state.connectors.read().await;
    
    let definitions: Vec<Value> = connectors
        .iter()
        .map(|c| json!({
            "id": c.id,
            "name": c.name,
            "connector_type": c.connector_type,
            "description": c.description,
            "icon": c.icon,
            "operations": c.operations,
            "config_schema": c.config_schema,
            "enabled": c.enabled
        }))
        .collect();

    Ok(Json(json!({
        "definitions": definitions,
        "count": definitions.len()
    })))
}
```

This is better because:
- ✅ No database query needed
- ✅ Data is already loaded at startup
- ✅ Matches the exact structure
- ✅ Fast response

---

## Trigger Definitions Too

Add the same for triggers:

```rust
/// GET /trigger-definitions
/// Returns available trigger types
async fn list_trigger_definitions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let triggers = state.triggers.read().await;
    
    let definitions: Vec<Value> = triggers
        .iter()
        .map(|t| json!({
            "id": t.id,
            "name": t.name,
            "trigger_type": t.trigger_type,
            "description": t.description,
            "icon": t.icon,
            "config_schema": t.config_schema,
            "enabled": t.enabled
        }))
        .collect();

    Ok(Json(json!({
        "definitions": definitions,
        "count": definitions.len()
    })))
}
```

Add route:
```rust
.route("/trigger-definitions", get(list_trigger_definitions))
```

---

## Complete Fixed Implementation

See the file `fix-connector-definitions-handler.rs` for the complete code.

