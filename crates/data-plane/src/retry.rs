use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;

/// Calculate delay for retry attempt using exponential backoff
pub fn calculate_backoff_delay(
    attempt: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    multiplier: f64,
) -> Duration {
    let delay_ms = (initial_delay_ms as f64 * multiplier.powi(attempt as i32)) as u64;
    let capped_delay = delay_ms.min(max_delay_ms);
    Duration::from_millis(capped_delay)
}

/// Execute a function with retry logic
pub async fn with_retry<F, Fut, T, E>(
    max_attempts: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    backoff_multiplier: f64,
    operation: F,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;
    
    for attempt in 0..max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    tracing::info!("✅ Retry succeeded on attempt {}/{}", attempt + 1, max_attempts);
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                
                if attempt < max_attempts - 1 {
                    let delay = calculate_backoff_delay(
                        attempt,
                        initial_delay_ms,
                        max_delay_ms,
                        backoff_multiplier,
                    );
                    
                    tracing::warn!(
                        "⚠️  Attempt {}/{} failed, retrying in {:?}...",
                        attempt + 1,
                        max_attempts,
                        delay
                    );
                    
                    sleep(delay).await;
                } else {
                    tracing::error!("❌ All {} retry attempts exhausted", max_attempts);
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        // Test exponential backoff
        assert_eq!(
            calculate_backoff_delay(0, 100, 10000, 2.0),
            Duration::from_millis(100)
        );
        assert_eq!(
            calculate_backoff_delay(1, 100, 10000, 2.0),
            Duration::from_millis(200)
        );
        assert_eq!(
            calculate_backoff_delay(2, 100, 10000, 2.0),
            Duration::from_millis(400)
        );
        
        // Test max delay cap
        assert_eq!(
            calculate_backoff_delay(10, 100, 1000, 2.0),
            Duration::from_millis(1000)
        );
    }
}
