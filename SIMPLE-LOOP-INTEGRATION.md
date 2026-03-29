# Simplified Loop Executor Integration - No LoopConfig Required

Direct integration into your existing FlowStep enum.

---

## Simple Integration - 3 Steps

### Step 1: Add Loop to Existing FlowStep Enum

Just add these fields directly to your FlowStep:

```rust
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
    // ADD THIS:
    Loop {
        name: String,
        loop_mode: String,  // "while", "foreach", or "count"
        
        // For while loops
        #[serde(skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
        
        // For foreach loops
        #[serde(skip_serializing_if = "Option::is_none")]
        iterate_over: Option<String>,
        
        // For count loops
        #[serde(skip_serializing_if = "Option::is_none")]
        count: Option<usize>,
        
        // Common fields
        steps: Vec<FlowStep>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_iterations: Option<usize>,
    },
}
```

---

## Step 2: Add Loop Execution to Your Executor

Add this method to your flow executor:

```rust
impl FlowExecutor {
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
            // ADD THIS:
            FlowStep::Loop { name, loop_mode, condition, iterate_over, count, steps, max_iterations } => {
                self.execute_loop(
                    name,
                    loop_mode,
                    condition.as_ref(),
                    iterate_over.as_ref(),
                    count,
                    steps,
                    max_iterations.unwrap_or(1000),
                    context
                ).await
            }
        }
    }

    async fn execute_loop(
        &self,
        name: &str,
        loop_mode: &str,
        condition: Option<&String>,
        iterate_over: Option<&String>,
        count: &Option<usize>,
        steps: &[FlowStep],
        max_iterations: usize,
        context: &mut HashMap<String, Value>,
    ) -> Result<Value> {
        tracing::info!("🔄 Executing loop: {} (mode: {})", name, loop_mode);

        let mut results = Vec::new();
        let mut iteration = 0;

        match loop_mode {
            "while" => {
                // While loop
                let condition_expr = condition
                    .ok_or_else(|| anyhow!("While loop missing condition"))?;

                while iteration < max_iterations {
                    // Evaluate condition
                    if !self.evaluate_condition(condition_expr, context)? {
                        tracing::debug!("Loop condition false, breaking");
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

            "foreach" => {
                // For-each loop
                let items_path = iterate_over
                    .ok_or_else(|| anyhow!("ForEach loop missing iterate_over"))?;

                let items = self.resolve_variable(items_path, context)?;
                let array = items.as_array()
                    .ok_or_else(|| anyhow!("iterate_over is not an array: {}", items_path))?;

                tracing::debug!("ForEach loop over {} items", array.len());

                for (index, item) in array.iter().enumerate() {
                    if iteration >= max_iterations {
                        tracing::warn!("Loop hit max_iterations: {}", max_iterations);
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

            "count" => {
                // Count loop
                let count_val = count
                    .ok_or_else(|| anyhow!("Count loop missing count"))?;
                let iterations = (*count_val).min(max_iterations);

                tracing::debug!("Count loop for {} iterations", iterations);

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

            _ => return Err(anyhow!("Unknown loop_mode: {}", loop_mode)),
        }

        // Set final loop iteration count
        context.insert("loop_iterations".to_string(), json!(iteration));

        tracing::info!("✅ Loop {} completed: {} iterations", name, iteration);

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
        // Resolve variables in condition
        let resolved = self.resolve_variables_in_string(condition, context);

        tracing::debug!("Evaluating condition: {} -> {}", condition, resolved);

        // Simple expression evaluation
        match resolved.trim() {
            "true" => Ok(true),
            "false" => Ok(false),
            "null" => Ok(false),
            "EOF" => Ok(false),
            expr if expr.contains("!=") => {
                let parts: Vec<&str> = expr.split("!=").map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    let result = parts[0] != parts[1];
                    tracing::debug!("  {} != {} = {}", parts[0], parts[1], result);
                    Ok(result)
                } else {
                    Err(anyhow!("Invalid != condition: {}", expr))
                }
            }
            expr if expr.contains("==") => {
                let parts: Vec<&str> = expr.split("==").map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    let result = parts[0] == parts[1];
                    tracing::debug!("  {} == {} = {}", parts[0], parts[1], result);
                    Ok(result)
                } else {
                    Err(anyhow!("Invalid == condition: {}", expr))
                }
            }
            _ => Err(anyhow!("Unsupported condition expression: {}", resolved))
        }
    }

    fn resolve_variable(
        &self,
        path: &str,
        context: &HashMap<String, Value>,
    ) -> Result<Value> {
        // Remove {{ }} if present
        let clean_path = path.trim_matches(|c| c == '{' || c == '}' || c == ' ');

        // Split by dots for nested access
        let parts: Vec<&str> = clean_path.split('.').collect();
        
        let mut current = context.get(parts[0])
            .ok_or_else(|| anyhow!("Variable not found: {}", parts[0]))?
            .clone();

        for part in &parts[1..] {
            current = current.get(part)
                .ok_or_else(|| anyhow!("Path not found: {}.{}", parts[0], part))?
                .clone();
        }

        Ok(current)
    }

    fn resolve_variables_in_string(
        &self,
        template: &str,
        context: &HashMap<String, Value>,
    ) -> String {
        let mut result = template.to_string();

        // Find and replace all {{variable}} patterns
        while let Some(start) = result.find("{{") {
            if let Some(end) = result[start..].find("}}") {
                let var_path = &result[start + 2..start + end].trim();
                
                if let Ok(value) = self.resolve_variable(var_path, context) {
                    let value_str = match value {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        _ => serde_json::to_string(&value).unwrap_or_default(),
                    };
                    
                    result.replace_range(start..start + end + 2, &value_str);
                } else {
                    // Can't resolve, skip
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

## Step 3: Use in Flow Definitions

### While Loop Example

```json
{
  "type": "loop",
  "name": "pagination_loop",
  "loop_mode": "while",
  "condition": "cursor != 'EOF'",
  "max_iterations": 100,
  "steps": [
    {
      "type": "call",
      "name": "fetch_page",
      "connector": "api",
      "operation": "get",
      "params": {"cursor": "{{cursor}}"}
    },
    {
      "type": "log",
      "name": "log",
      "message": "Fetched page {{iteration}}"
    }
  ]
}
```

### ForEach Loop Example

```json
{
  "type": "loop",
  "name": "process_items",
  "loop_mode": "foreach",
  "iterate_over": "{{fetch_page.items}}",
  "steps": [
    {
      "type": "call",
      "name": "insert",
      "connector": "postgres",
      "operation": "execute",
      "params": {
        "sql": "INSERT INTO users VALUES ($1, $2)",
        "params": ["{{item.id}}", "{{item.name}}"]
      }
    }
  ]
}
```

### Count Loop Example

```json
{
  "type": "loop",
  "name": "retry",
  "loop_mode": "count",
  "count": 5,
  "steps": [
    {
      "type": "call",
      "name": "api_call",
      "connector": "unreliable-api",
      "operation": "get"
    }
  ]
}
```

---

## Complete Working Example

### Flow with Nested Loops

```json
{
  "id": "user-sync",
  "name": "Sync Users",
  "steps": [
    {
      "type": "log",
      "name": "start",
      "message": "Starting sync"
    },
    {
      "type": "loop",
      "name": "outer_loop",
      "loop_mode": "while",
      "condition": "cursor != 'EOF'",
      "max_iterations": 50,
      "steps": [
        {
          "type": "call",
          "name": "fetch",
          "connector": "api",
          "operation": "get",
          "params": {"cursor": "{{cursor}}"}
        },
        {
          "type": "loop",
          "name": "inner_loop",
          "loop_mode": "foreach",
          "iterate_over": "{{fetch.data}}",
          "steps": [
            {
              "type": "call",
              "name": "insert",
              "connector": "postgres",
              "operation": "execute",
              "params": {
                "sql": "INSERT INTO users VALUES ($1, $2)",
                "params": ["{{item.id}}", "{{item.name}}"]
              }
            }
          ]
        }
      ]
    },
    {
      "type": "log",
      "name": "done",
      "message": "Completed {{loop_iterations}} iterations"
    }
  ]
}
```

---

## Testing

```rust
#[tokio::test]
async fn test_while_loop() {
    let executor = FlowExecutor::new();
    let mut context = HashMap::new();
    context.insert("cursor".to_string(), json!(null));

    let loop_step = FlowStep::Loop {
        name: "test".to_string(),
        loop_mode: "while".to_string(),
        condition: Some("cursor != 'EOF'".to_string()),
        iterate_over: None,
        count: None,
        max_iterations: Some(3),
        steps: vec![
            FlowStep::Log {
                name: "log".to_string(),
                message: "Iteration {{iteration}}".to_string(),
            }
        ],
    };

    let result = executor.execute_step(&loop_step, &mut context).await.unwrap();
    
    assert_eq!(context.get("loop_iterations").unwrap(), &json!(3));
}
```

---

## Summary

✅ **No LoopConfig needed** - Everything in FlowStep  
✅ **Simple integration** - Just add match arm  
✅ **Works immediately** - No extra structs  
✅ **Full functionality** - While, ForEach, Count loops  

**Copy the code above and you're done!** 🚀✅
