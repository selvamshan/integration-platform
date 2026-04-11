# How to Use loop_executor.rs - Step-by-Step Integration Guide

Complete guide to integrate loop control into your integration platform.

---

## Step 1: Add loop_executor.rs to Your Project

### Copy the File

```bash
cp implementations/loop_executor.rs your-project/crates/integration-runtime/src/
```

### Update mod.rs

In `crates/integration-runtime/src/lib.rs` or `mod.rs`, add:

```rust
pub mod loop_executor;
pub use loop_executor::{LoopExecutor, LoopType};
```

---

## Step 2: Update FlowStep Enum

In your flow execution code (e.g., `crates/integration-runtime/src/executor.rs`), add the Loop variant:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FlowStep {
    Call {
        name: String,
        connector: String,
        operation: String,
        params: Value,
    },
    Transform {
        name: String,
        spec: Value,
    },
    Log {
        name: String,
        message: String,
    },
    SetVariable {
        name: String,
        variables: Value,
    },
    // NEW: Add Loop step
    Loop {
        name: String,
        #[serde(flatten)]
        loop_config: LoopConfig,
        steps: Vec<FlowStep>,
        max_iterations: Option<usize>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "loop_mode")]
pub enum LoopConfig {
    #[serde(rename = "while")]
    While {
        condition: ConditionExpr,
    },
    #[serde(rename = "foreach")]
    ForEach {
        iterate_over: String,
    },
    #[serde(rename = "count")]
    Count {
        count: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionExpr {
    #[serde(rename = "type")]
    condition_type: String,
    expression: String,
}
```

---

## Step 3: Integrate into Flow Executor

### Example Integration

```rust
use crate::loop_executor::{LoopExecutor, LoopType};
use std::collections::HashMap;
use serde_json::Value;

pub struct FlowExecutor {
    loop_executor: LoopExecutor,
    // ... other fields
}

impl FlowExecutor {
    pub fn new() -> Self {
        Self {
            loop_executor: LoopExecutor::new().with_max_iterations(1000),
        }
    }

    pub async fn execute_step(
        &self,
        step: &FlowStep,
        context: &mut HashMap<String, Value>,
    ) -> Result<Value> {
        match step {
            FlowStep::Call { name, connector, operation, params } => {
                self.execute_call(name, connector, operation, params, context).await
            }
            FlowStep::Transform { name, spec } => {
                self.execute_transform(name, spec, context).await
            }
            FlowStep::Log { name, message } => {
                self.execute_log(name, message, context).await
            }
            FlowStep::SetVariable { name, variables } => {
                self.execute_set_variable(name, variables, context).await
            }
            // NEW: Handle Loop step
            FlowStep::Loop { name, loop_config, steps, max_iterations } => {
                self.execute_loop(name, loop_config, steps, max_iterations, context).await
            }
        }
    }

    async fn execute_loop(
        &self,
        name: &str,
        config: &LoopConfig,
        steps: &[FlowStep],
        max_iterations: &Option<usize>,
        context: &mut HashMap<String, Value>,
    ) -> Result<Value> {
        tracing::info!("Executing loop: {}", name);

        // Convert LoopConfig to LoopType
        let loop_type = match config {
            LoopConfig::While { condition } => {
                LoopType::While {
                    condition: condition.expression.clone(),
                }
            }
            LoopConfig::ForEach { iterate_over } => {
                LoopType::ForEach {
                    items_path: iterate_over.clone(),
                }
            }
            LoopConfig::Count { count } => {
                LoopType::Count {
                    count: *count,
                }
            }
        };

        // Convert FlowSteps to JSON for loop executor
        let step_values: Vec<Value> = steps.iter()
            .map(|s| serde_json::to_value(s).unwrap())
            .collect();

        // Create executor with custom max_iterations if specified
        let executor = if let Some(max) = max_iterations {
            LoopExecutor::new().with_max_iterations(*max)
        } else {
            self.loop_executor.clone()
        };

        // Execute the loop
        // Note: We need to execute actual steps, not just pass JSON
        // Let's create a custom execution method
        let result = self.execute_loop_with_steps(
            loop_type,
            steps,
            context,
            max_iterations.unwrap_or(1000)
        ).await?;

        Ok(result)
    }

    async fn execute_loop_with_steps(
        &self,
        loop_type: LoopType,
        steps: &[FlowStep],
        context: &mut HashMap<String, Value>,
        max_iterations: usize,
    ) -> Result<Value> {
        let mut results = Vec::new();
        let mut iteration = 0;

        match loop_type {
            LoopType::While { condition } => {
                // While loop
                while iteration < max_iterations {
                    // Evaluate condition
                    if !self.evaluate_condition(&condition, context)? {
                        break;
                    }

                    // Set loop variables
                    context.insert("iteration".to_string(), json!(iteration + 1));
                    context.insert("index".to_string(), json!(iteration));

                    // Execute steps
                    let mut last_result = Value::Null;
                    for step in steps {
                        last_result = self.execute_step(step, context).await?;
                    }
                    results.push(last_result);

                    iteration += 1;
                }
            }

            LoopType::ForEach { items_path } => {
                // For-each loop
                let items = self.resolve_path(&items_path, context)?;
                let array = items.as_array()
                    .ok_or_else(|| anyhow!("Not an array: {}", items_path))?;

                for (index, item) in array.iter().enumerate() {
                    if iteration >= max_iterations {
                        break;
                    }

                    // Set loop variables
                    context.insert("item".to_string(), item.clone());
                    context.insert("index".to_string(), json!(index));
                    context.insert("iteration".to_string(), json!(iteration + 1));

                    // Execute steps
                    let mut last_result = Value::Null;
                    for step in steps {
                        last_result = self.execute_step(step, context).await?;
                    }
                    results.push(last_result);

                    iteration += 1;
                }
            }

            LoopType::Count { count } => {
                // Count loop
                let iterations = count.min(max_iterations);
                for i in 0..iterations {
                    context.insert("index".to_string(), json!(i));
                    context.insert("iteration".to_string(), json!(i + 1));

                    // Execute steps
                    let mut last_result = Value::Null;
                    for step in steps {
                        last_result = self.execute_step(step, context).await?;
                    }
                    results.push(last_result);

                    iteration += 1;
                }
            }
        }

        // Set final loop iteration count
        context.insert("loop_iterations".to_string(), json!(iteration));

        Ok(json!({
            "iterations": iteration,
            "results": results
        }))
    }

    fn evaluate_condition(
        &self,
        condition: &str,
        context: &HashMap<String, Value>,
    ) -> Result<bool> {
        // Resolve variables first
        let resolved = self.resolve_variables(condition, context);

        // Evaluate expression
        match resolved.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            expr if expr.contains("!=") => {
                let parts: Vec<&str> = expr.split("!=").collect();
                if parts.len() == 2 {
                    Ok(parts[0].trim() != parts[1].trim())
                } else {
                    Err(anyhow!("Invalid condition"))
                }
            }
            expr if expr.contains("==") => {
                let parts: Vec<&str> = expr.split("==").collect();
                if parts.len() == 2 {
                    Ok(parts[0].trim() == parts[1].trim())
                } else {
                    Err(anyhow!("Invalid condition"))
                }
            }
            _ => Err(anyhow!("Unsupported condition: {}", resolved))
        }
    }

    fn resolve_path(
        &self,
        path: &str,
        context: &HashMap<String, Value>,
    ) -> Result<Value> {
        let clean = path.trim_matches(|c| c == '{' || c == '}').trim();
        let parts: Vec<&str> = clean.split('.').collect();
        
        let mut current = context.get(parts[0])
            .ok_or_else(|| anyhow!("Variable not found: {}", parts[0]))?
            .clone();

        for part in &parts[1..] {
            current = current.get(part)
                .ok_or_else(|| anyhow!("Path not found"))?
                .clone();
        }

        Ok(current)
    }

    fn resolve_variables(
        &self,
        template: &str,
        context: &HashMap<String, Value>,
    ) -> String {
        let mut result = template.to_string();

        while let Some(start) = result.find("{{") {
            if let Some(end) = result[start..].find("}}") {
                let var_name = &result[start + 2..start + end].trim();
                
                if let Ok(value) = self.resolve_path(var_name, context) {
                    let value_str = match value {
                        Value::String(s) => s,
                        Value::Null => "null".to_string(),
                        v => v.to_string(),
                    };
                    result.replace_range(start..start + end + 2, &value_str);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }
}
```

---

## Step 4: Example Usage

### Flow Definition

```json
{
  "id": "test-pagination",
  "name": "Test Pagination Loop",
  "steps": [
    {
      "type": "set_variable",
      "name": "init",
      "variables": {
        "cursor": null,
        "total": 0
      }
    },
    {
      "type": "loop",
      "name": "fetch_pages",
      "loop_mode": "while",
      "condition": {
        "type": "expression",
        "expression": "cursor != 'EOF'"
      },
      "max_iterations": 10,
      "steps": [
        {
          "type": "call",
          "name": "get_page",
          "connector": "api",
          "operation": "get",
          "params": {
            "cursor": "{{cursor}}"
          }
        },
        {
          "type": "loop",
          "name": "process_items",
          "loop_mode": "foreach",
          "iterate_over": "{{get_page.items}}",
          "steps": [
            {
              "type": "log",
              "name": "log_item",
              "message": "Processing item {{item.id}}"
            }
          ]
        },
        {
          "type": "set_variable",
          "name": "update_cursor",
          "variables": {
            "cursor": "{{get_page.next_cursor || 'EOF'}}",
            "total": "{{total + get_page.items.length}}"
          }
        }
      ]
    },
    {
      "type": "log",
      "name": "done",
      "message": "Processed {{total}} items in {{loop_iterations}} iterations"
    }
  ]
}
```

### Execute the Flow

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Load flow
    let flow_json = std::fs::read_to_string("flow.json")?;
    let flow: Flow = serde_json::from_str(&flow_json)?;
    
    // Create executor
    let executor = FlowExecutor::new();
    
    // Create context
    let mut context = HashMap::new();
    context.insert("trigger".to_string(), json!({}));
    
    // Execute flow
    for step in &flow.steps {
        executor.execute_step(step, &mut context).await?;
    }
    
    println!("Flow completed!");
    println!("Total iterations: {}", context.get("loop_iterations"));
    
    Ok(())
}
```

---

## Step 5: Testing

### Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_while_loop() {
        let executor = FlowExecutor::new();
        let mut context = HashMap::new();
        context.insert("cursor".to_string(), json!(null));
        context.insert("count".to_string(), json!(0));

        let steps = vec![
            FlowStep::SetVariable {
                name: "increment".to_string(),
                variables: json!({"count": "{{count + 1}}"}),
            },
            FlowStep::SetVariable {
                name: "check".to_string(),
                variables: json!({"cursor": "{{count >= 3 ? 'EOF' : null}}"}),
            },
        ];

        let result = executor.execute_loop_with_steps(
            LoopType::While {
                condition: "cursor != 'EOF'".to_string(),
            },
            &steps,
            &mut context,
            10,
        ).await.unwrap();

        assert_eq!(result["iterations"], 3);
    }
}
```

---

## Step 6: Add Dependencies

In `Cargo.toml`:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
```

---

## Common Patterns

### Pattern 1: Pagination with Cursor

```json
{
  "type": "loop",
  "loop_mode": "while",
  "condition": {"expression": "cursor != 'EOF'"},
  "steps": [
    {"type": "call", "connector": "api", "params": {"cursor": "{{cursor}}"}},
    {"type": "set_variable", "variables": {"cursor": "{{api.next_cursor || 'EOF'}}"}}
  ]
}
```

### Pattern 2: Batch Processing

```json
{
  "type": "loop",
  "loop_mode": "foreach",
  "iterate_over": "{{orders}}",
  "steps": [
    {"type": "call", "connector": "payment", "params": {"order_id": "{{item.id}}"}}
  ]
}
```

### Pattern 3: Retry with Backoff

```json
{
  "type": "loop",
  "loop_mode": "count",
  "count": 5,
  "steps": [
    {"type": "call", "connector": "unreliable-api"},
    {"type": "delay", "duration": "{{2 ** index}} seconds"}
  ]
}
```

---

## Troubleshooting

### Issue: Loop never ends

**Solution:** Check max_iterations and condition logic

```rust
// Set lower max for testing
.with_max_iterations(10)
```

### Issue: Variables not resolving

**Solution:** Check variable path and context

```rust
// Debug context
tracing::debug!("Context: {:?}", context);
```

### Issue: Nested loops not working

**Solution:** Ensure inner loop variables don't conflict

```json
{
  "type": "loop",
  "name": "outer",
  "steps": [
    {
      "type": "loop",
      "name": "inner",
      "steps": [
        // Use {{item}} for inner, parent context still accessible
      ]
    }
  ]
}
```

---

## Summary

✅ **Step 1:** Copy loop_executor.rs  
✅ **Step 2:** Update FlowStep enum  
✅ **Step 3:** Integrate into executor  
✅ **Step 4:** Create flow definitions  
✅ **Step 5:** Test thoroughly  
✅ **Step 6:** Add dependencies  

**Your loop control is ready to use!** 🔄✅
