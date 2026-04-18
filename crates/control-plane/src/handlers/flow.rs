use std::sync::Arc;
use axum::{   
    extract::{State, Path, Json}   
};
use async_nats::Client as NatsClient;


use common::{  
    ApiDefinition, 
    FlowDefinition, 
    Endpoint,  
    ConfigUpdate,   
    Trigger,  
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use sqlx::Row;
use crate::error::AppError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct TestFlowRequest {
    flow: Value,
    test_input: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct TestFlowResponse {
    success: bool,
    result: Option<Value>,
    error: Option<String>,
    execution: ExecutionDetails,
}

#[derive(Debug, Serialize)]
pub struct ExecutionDetails {
    duration_ms: u64,
    steps_executed: usize,
    step_results: Vec<StepResult>,
    output: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct StepResult {
    name: String,
    step_type: String,
    success: bool,
    output: Option<Value>,
    error: Option<String>,
    duration_ms: u64,
}

/// POST /flows/test
/// Test a flow definition without saving it
pub async fn test_flow(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TestFlowRequest>,
) -> Result<Json<TestFlowResponse>, AppError> {
    let start_time = std::time::Instant::now();
    
    tracing::info!("🧪 Testing flow");
    
    // Validate flow structure — must have a name and either nodes (graph) or steps (legacy)
    if payload.flow["name"].is_null() {
        return Err(AppError::BadRequest("Invalid flow structure: missing name".to_string()));
    }
    let has_nodes = payload.flow["nodes"].as_array().map_or(false, |a| !a.is_empty());
    let has_steps = payload.flow["steps"].as_array().map_or(false, |a| !a.is_empty());
    if !has_nodes && !has_steps {
        return Err(AppError::BadRequest("Invalid flow structure: must have nodes or steps".to_string()));
    }

    // Collect the steps to test — from nodes for graph flows, otherwise from steps
    let steps: Vec<&Value> = if has_nodes {
        payload.flow["nodes"]
            .as_array()
            .map(|nodes| nodes.iter().map(|n| &n["step"]).collect())
            .unwrap_or_default()
    } else {
        payload.flow["steps"]
            .as_array()
            .map(|s| s.iter().collect())
            .unwrap_or_default()
    };

    // Prepare test input
    let test_input = payload.test_input.unwrap_or_else(|| json!({}));

    // Execute steps
    let mut step_results = Vec::new();
    let mut context = serde_json::Map::new();
    context.insert("trigger".to_string(), test_input);

    let mut last_output = Value::Null;
    let mut execution_success = true;

    for (index, step) in steps.iter().enumerate() {
        let step_start = std::time::Instant::now();
        let default_name = format!("step_{}", index);
        let step_name = step["name"].as_str().unwrap_or(&default_name);
        let step_type = step["type"].as_str().unwrap_or("unknown");

        // Execute step
        let step_result = match execute_test_step(step, &context, &state).await {
            Ok(output) => {
                context.insert(step_name.to_string(), output.clone());
                last_output = output.clone();

                StepResult {
                    name: step_name.to_string(),
                    step_type: step_type.to_string(),
                    success: true,
                    output: Some(output),
                    error: None,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                }
            }
            Err(e) => {
                execution_success = false;
                StepResult {
                    name: step_name.to_string(),
                    step_type: step_type.to_string(),
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                }
            }
        };

        step_results.push(step_result);

        if !execution_success {
            break;
        }
    }
    
    let total_duration = start_time.elapsed().as_millis() as u64;
    
    Ok(Json(TestFlowResponse {
        success: execution_success,
        result: if execution_success { Some(last_output.clone()) } else { None },
        error: if !execution_success {
            step_results.iter().find(|r| !r.success).and_then(|r| r.error.clone())
        } else {
            None
        },
        execution: ExecutionDetails {
            duration_ms: total_duration,
            steps_executed: step_results.len(),
            step_results,
            output: Some(last_output),
        },
    }))
}

pub async fn execute_test_step(
    step: &Value,
    _context: &serde_json::Map<String, Value>,
    state: &AppState,
) -> Result<Value, AppError> {
    let step_type = step["type"].as_str().unwrap_or("unknown");
    
    match step_type {
        "log" => {
            let message = step["message"].as_str().unwrap_or("Test log");
            tracing::info!("📝 Log: {}", message);
            Ok(json!({ "logged": message }))
        }
        "transform" => {
            let spec = &step["spec"];
            let transform_type = spec["type"].as_str().unwrap_or("select");
            Ok(json!({ 
                "transform_type": transform_type, 
                "result": "transformed_data" 
            }))
        }
        "call" => {
            let connector = step["connector"].as_str()
                .ok_or_else(|| AppError::BadRequest("Missing connector".to_string()))?;
            
            // Verify connector exists
            let instances = state.connector_instances.read().await;
            if instances.iter().find(|c| c.id == connector).is_none() {
                return Err(AppError::BadRequest(
                    format!("Connector not found: {}", connector)
                ));
            }
            
            Ok(json!({
                "connector": connector,
                "operation": step["operation"],
                "test_mode": true,
                "result": "mock_connector_response"
            }))
        }
        "set_variable" => {
            Ok(json!({ "variables_set": step["variables"] }))
        }
        _ => Ok(json!({ "step_type": step_type, "result": "ok" }))
    }
}

// Add route:
// .route("/flows/test", post(test_flow))


// Flow endpoints
pub async fn list_flows(State(state): State<Arc<AppState>>) -> Json<Value> {
    let flows = state.flows.read().await;
    Json(json!({"flows": *flows, "count": flows.len()}))
}

pub async fn create_flow(State(state): State<Arc<AppState>>, Json(flow): Json<FlowDefinition>) -> Result<Json<Value>, AppError> {
    tracing::info!("📡 Creating flow: {}", flow.name);
    
    sqlx::query("INSERT INTO flow_definitions (name, config) VALUES ($1, $2)")
        .bind(&flow.name)
        .bind(serde_json::to_value(&flow).unwrap())
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    flows.push(flow.clone());
    drop(flows);
    
    // Auto-create or update API definition for HTTP triggers
    if let Trigger::Http { path, method } = &flow.trigger {
        auto_update_api_definition(&state, &flow, path, method).await?;
    }
    
    let event = ConfigUpdate::FlowCreated { flow: flow.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow created and API auto-updated: {}", flow.id);
    Ok(Json(json!(flow)))
}

pub async fn update_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(flow): Json<FlowDefinition>) -> Result<Json<Value>, AppError> {
    tracing::info!("🔄 Updating flow: {}", id);
    
    if flow.id != id {
        return Err(AppError::Internal("Flow ID mismatch".to_string()));
    }
    
    sqlx::query("UPDATE flow_definitions SET name = $1, config = $2 WHERE config->>'id' = $3")
        .bind(&flow.name)
        .bind(serde_json::to_value(&flow).unwrap())
        .bind(&id)
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    if let Some(existing) = flows.iter_mut().find(|f| f.id == id) {
        *existing = flow.clone();
    }
    drop(flows);
    
    // Auto-update API definition
    if let Trigger::Http { path, method } = &flow.trigger {
        auto_update_api_definition(&state, &flow, path, method).await?;
    }
    
    let event = ConfigUpdate::FlowUpdated { flow: flow.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow updated and API auto-updated: {}", id);
    Ok(Json(json!(flow)))
}

pub async fn get_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let flows = state.flows.read().await;
    let flow = flows.iter().find(|f| f.id == id).ok_or_else(|| AppError::NotFound("Flow not found".to_string()))?;
    Ok(Json(json!(flow)))
}

pub async fn delete_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    tracing::info!("🗑️  Deleting flow: {}", id);
    
    // Get flow before deleting to update API
    let flow = {
        let flows = state.flows.read().await;
        flows.iter().find(|f| f.id == id).cloned()
    };
    
    sqlx::query("DELETE FROM flow_definitions WHERE config->>'id' = $1")
        .bind(&id)
        .execute(&state.db)
        .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    
    let mut flows = state.flows.write().await;
    flows.retain(|f| f.id != id);
    drop(flows);
    
    // Remove from API definition
    if let Some(flow) = flow {
        if let Trigger::Http { path, .. } = &flow.trigger {
            remove_from_api_definition(&state, &id, path).await?;
        }
    }
    
    let event = ConfigUpdate::FlowDeleted { flow_id: id.clone() };
    publish_event(&state.nats, &event).await?;
    
    tracing::info!("✅ Flow deleted and API updated: {}", id);
    Ok(Json(json!({"deleted": true, "flow_id": id})))
}


// Auto-update API definition when flow changes
async fn auto_update_api_definition(state: &AppState, flow: &FlowDefinition, path: &str, method: &str) -> Result<(), AppError> {
    tracing::info!("🔄 Auto-updating API definition for flow: {}", flow.id);

    let api_name = flow.name.clone();
    let api_version = "1.0";

    let mut apis = state.apis.write().await;

    // Query DB directly as source of truth to avoid stale in-memory state after restart
    let existing_row = sqlx::query(
        "SELECT id, config FROM api_definitions WHERE name = $1 AND version = $2"
    )
        .bind(&api_name)
        .bind(api_version)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    if let Some(row) = existing_row {
        let db_id: uuid::Uuid = row.try_get("id")
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
        let config: serde_json::Value = row.try_get("config")
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

        let mut api: ApiDefinition = serde_json::from_value(config)
            .map_err(|e| AppError::Internal(format!("Deserialize error: {}", e)))?;

        if let Some(endpoint) = api.endpoints.iter_mut().find(|e| e.path == path && e.method == method) {
            endpoint.flow_id = flow.id.clone();
        } else {
            api.endpoints.push(Endpoint {
                path: path.to_string(),
                method: method.to_string(),
                flow_id: flow.id.clone(),
            });
        }

        sqlx::query("UPDATE api_definitions SET config = $1 WHERE id = $2")
            .bind(serde_json::to_value(&api).unwrap())
            .bind(db_id)
            .execute(&state.db)
            .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

        // Sync in-memory cache
        if let Some(mem_api) = apis.iter_mut().find(|a| a.name == api_name && a.version == api_version) {
            *mem_api = api;
        } else {
            apis.push(api);
        }

        tracing::info!("✅ Updated existing API definition");
    } else {
        let api_id = uuid::Uuid::new_v4().to_string();
        let new_api = ApiDefinition {
            id: api_id.clone(),
            name: api_name.to_string(),
            version: api_version.to_string(),
            base_path: "/api".to_string(),
            endpoints: vec![Endpoint {
                path: path.to_string(),
                method: method.to_string(),
                flow_id: flow.id.clone(),
            }],
        };

        sqlx::query("INSERT INTO api_definitions (id, name, version, base_path, config) VALUES ($1, $2, $3, $4, $5)")
            .bind(uuid::Uuid::parse_str(&new_api.id).unwrap())
            .bind(&new_api.name)
            .bind(&new_api.version)
            .bind(&new_api.base_path)
            .bind(serde_json::to_value(&new_api).unwrap())
            .execute(&state.db)
            .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

        apis.push(new_api);

        tracing::info!("✅ Created new API definition");
    }

    Ok(())
}

async fn remove_from_api_definition(state: &AppState, flow_id: &str, _path: &str) -> Result<(), AppError> {
    let mut apis = state.apis.write().await;
    
    for api in apis.iter_mut() {
        api.endpoints.retain(|e| e.flow_id != flow_id);
        
        // Update in DB
        sqlx::query("UPDATE api_definitions SET config = $1 WHERE id = $2")
            .bind(serde_json::to_value(&*api).unwrap())
            .bind(uuid::Uuid::parse_str(&api.id).unwrap())
            .execute(&state.db)
            .await.map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;
    }
    
    Ok(())
}

pub async fn publish_event(nats: &NatsClient, event: &ConfigUpdate) -> Result<(), AppError> {
    let subject = event.subject();
    let payload = serde_json::to_vec(event).map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;
    nats.publish(subject, payload.into()).await.map_err(|e| AppError::Internal(format!("NATS publish error: {}", e)))?;
    tracing::debug!("📤 Published event to {}", subject);
    Ok(())
}
