use common::{Message, Result, Error, FlowStep};
use serde_json::json;
use tracing::{info, warn};

use crate::templates::{evaluate_condition, resolve_template_str};

/// Trait for anything that can execute a sequence of flow steps.
/// Implemented by `FlowExecutor` so `LoopExecutor` can call back into it.
#[async_trait::async_trait]
pub trait StepExecutor: Send + Sync {
    async fn run_steps(&self, steps: &[FlowStep], message: Message) -> Result<Message>;
}

/// Loop step types
#[derive(Debug, Clone)]
pub enum LoopType {
    /// While loop - continues while condition is true
    While { condition: String },
    /// For-each loop - iterates over array
    ForEach { items_path: String },
    /// Count loop - fixed number of iterations
    Count { count: usize },
}

/// Handles all loop modes, delegating step execution back to the caller via `StepExecutor`.
pub struct LoopExecutor {
    pub max_iterations: usize,
}

impl LoopExecutor {
    pub fn new() -> Self {
        Self { max_iterations: 1000 }
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Execute a loop, calling `executor.run_steps` for each iteration.
    pub async fn execute(
        &self,
        name: &str,
        loop_type: LoopType,
        steps: &[FlowStep],
        mut message: Message,
        executor: &dyn StepExecutor,
    ) -> Result<Message> {
        info!("🔄 [{}] Starting loop (mode: {})", name, loop_type_name(&loop_type));
        let mut iteration: usize = 0;

        match loop_type {
            LoopType::While { ref condition } => {
                while iteration < self.max_iterations {
                    if !evaluate_condition(condition, &message.payload) {
                        break;
                    }

                    if let serde_json::Value::Object(ref mut map) = message.payload {
                        map.insert("iteration".to_string(), json!(iteration + 1));
                        map.insert("index".to_string(), json!(iteration));
                    }

                    message = executor.run_steps(steps, message).await?;
                    iteration += 1;
                }

                if iteration >= self.max_iterations {
                    warn!("   ⚠️  Loop '{}' hit max_iterations ({})", name, self.max_iterations);
                }
            }

            LoopType::ForEach { items_path } => {
                let items_value = resolve_template_str(&items_path, &message.payload);
                let array = items_value
                    .as_array()
                    .ok_or_else(|| Error::Flow(format!(
                        "Loop '{}': iterate_over '{}' did not resolve to an array",
                        name, items_path
                    )))?
                    .clone();

                info!("   ForEach over {} items", array.len());

                for (index, item) in array.iter().enumerate() {
                    if iteration >= self.max_iterations {
                        warn!("   ⚠️  Loop '{}' hit max_iterations ({})", name, self.max_iterations);
                        break;
                    }

                    if let serde_json::Value::Object(ref mut map) = message.payload {
                        map.insert("item".to_string(), item.clone());
                        map.insert("index".to_string(), json!(index));
                        map.insert("iteration".to_string(), json!(iteration + 1));
                    }

                    message = executor.run_steps(steps, message).await?;
                    iteration += 1;
                }
            }

            LoopType::Count { count } => {
                let n = count.min(self.max_iterations);
                info!("   Count loop: {} iterations", n);

                for i in 0..n {
                    if let serde_json::Value::Object(ref mut map) = message.payload {
                        map.insert("index".to_string(), json!(i));
                        map.insert("iteration".to_string(), json!(i + 1));
                    }

                    message = executor.run_steps(steps, message).await?;
                    iteration += 1;
                }
            }
        }

        if let serde_json::Value::Object(ref mut map) = message.payload {
            map.insert("loop_iterations".to_string(), json!(iteration));
        }

        info!("   ✅ Loop '{}' completed: {} iterations", name, iteration);
        Ok(message)
    }
}

impl Default for LoopExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn loop_type_name(lt: &LoopType) -> &'static str {
    match lt {
        LoopType::While { .. } => "while",
        LoopType::ForEach { .. } => "foreach",
        LoopType::Count { .. } => "count",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Message;
    use serde_json::json;

    struct TestExecutor;

    #[async_trait::async_trait]
    impl StepExecutor for TestExecutor {
        async fn run_steps(&self, _steps: &[FlowStep], message: Message) -> Result<Message> {
            Ok(message)
        }
    }

    #[tokio::test]
    async fn test_count_loop() {
        let executor = LoopExecutor::new();
        let message = Message::new(json!({}));

        let result = executor.execute(
            "test",
            LoopType::Count { count: 5 },
            &[],
            message,
            &TestExecutor,
        ).await.unwrap();

        assert_eq!(result.payload["loop_iterations"], 5);
    }

    #[tokio::test]
    async fn test_foreach_loop() {
        let executor = LoopExecutor::new();
        let message = Message::new(json!({ "items": [1, 2, 3] }));

        let result = executor.execute(
            "test",
            LoopType::ForEach { items_path: "{{items}}".to_string() },
            &[],
            message,
            &TestExecutor,
        ).await.unwrap();

        assert_eq!(result.payload["loop_iterations"], 3);
    }
}
