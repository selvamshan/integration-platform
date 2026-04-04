// crates/data-plane/src/scheduler.rs

use tokio_cron_scheduler::{Job, JobScheduler};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde_json::{json, Value};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use anyhow::{Result, anyhow};
use uuid::Uuid;

/// Scheduled flow registration
#[derive(Debug, Clone)]
pub struct ScheduledFlow {
    pub flow_id: String,
    pub flow_name: String,
    pub cron_expr: String,
    pub job_id: Uuid,
    pub enabled: bool,
    pub last_execution: Option<DateTime<Utc>>,
}

/// Flow scheduler service
pub struct FlowScheduler {
    scheduler: Mutex<JobScheduler>,
    timezone: Tz,
    jobs: Arc<RwLock<HashMap<String, ScheduledFlow>>>, // flow_id -> scheduled_flow
}

impl FlowScheduler {
    /// Create a new flow scheduler
    pub async fn new(timezone: &str) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        let tz = timezone.parse::<Tz>()
            .unwrap_or(chrono_tz::UTC);

        Ok(Self {
            scheduler: Mutex::new(scheduler),
            timezone: tz,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        self.scheduler.lock().await.start().await?;
        tracing::info!("✅ Flow scheduler started (timezone: {})", self.timezone);
        Ok(())
    }

    /// Stop the scheduler
    pub async fn shutdown(&self) -> Result<()> {
        self.scheduler.lock().await.shutdown().await?;
        tracing::info!("🛑 Flow scheduler stopped");
        Ok(())
    }

    /// Add or update a scheduled flow
    pub async fn schedule_flow<F>(
        &self,
        flow_id: String,
        flow_name: String,
        cron_expr: &str,
        executor: F,
    ) -> Result<Uuid>
    where
        F: Fn(String, Value) -> tokio::task::JoinHandle<Result<Value>> + Send + Sync + 'static + Clone,
    {
        // Remove existing job if present
        if let Some(existing) = self.jobs.read().await.get(&flow_id) {
            self.scheduler.lock().await.remove(&existing.job_id).await.ok();
            tracing::debug!("Removed existing schedule for flow: {}", flow_id);
        }

        // tokio_cron_scheduler requires a 6-field cron expression (sec min hour day month weekday).
        // Convert standard 5-field expressions by prepending "0 " for seconds.
        let cron_6field: String = if cron_expr.split_whitespace().count() == 5 {
            format!("0 {}", cron_expr)
        } else {
            cron_expr.to_string()
        };

        let flow_id_clone = flow_id.clone();
        let flow_name_clone = flow_name.clone();
        let jobs = self.jobs.clone();

        // Create new scheduled job
        let job = Job::new_async(cron_6field.as_str(), move |_uuid, _lock| {
            let flow_id = flow_id_clone.clone();
            let flow_name = flow_name_clone.clone();
            let executor = executor.clone();
            let jobs = jobs.clone();

            Box::pin(async move {
                let execution_time = Utc::now();
                
                tracing::info!(
                    "⏰ Executing scheduled flow: {} ({}) at {}",
                    flow_name,
                    flow_id,
                    execution_time.format("%Y-%m-%d %H:%M:%S UTC")
                );

                let trigger_context = json!({
                    "type": "schedule",
                    "scheduled_time": execution_time.to_rfc3339(),
                    "execution_time": execution_time.to_rfc3339(),
                    "flow_id": flow_id,
                    "flow_name": flow_name,
                });

                // Execute flow
                let handle = executor(flow_id.clone(), trigger_context);
                match handle.await {
                    Ok(Ok(result)) => {
                        tracing::info!("✅ Scheduled flow completed: {}", flow_name);
                        tracing::debug!("Result: {:?}", result);
                        
                        // Update last execution time
                        if let Some(job) = jobs.write().await.get_mut(&flow_id) {
                            job.last_execution = Some(execution_time);
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::error!("❌ Scheduled flow failed: {} - {}", flow_name, e);
                    }
                    Err(e) => {
                        tracing::error!("❌ Scheduled flow task failed: {} - {}", flow_name, e);
                    }
                }
            })
        })?;

        let job_id = self.scheduler.lock().await.add(job).await?;

        // Store job info
        let scheduled_flow = ScheduledFlow {
            flow_id: flow_id.clone(),
            flow_name: flow_name.clone(),
            cron_expr: cron_expr.to_string(),
            job_id,
            enabled: true,
            last_execution: None,
        };

        self.jobs.write().await.insert(flow_id.clone(), scheduled_flow);

        tracing::info!(
            "📅 Scheduled flow: {} ({}) with cron: {} (job_id: {})",
            flow_name,
            flow_id,
            cron_expr,
            job_id
        );

        Ok(job_id)
    }

    /// Remove a scheduled flow by flow_id
    pub async fn unschedule_flow(&self, flow_id: &str) -> Result<()> {
        if let Some(scheduled_flow) = self.jobs.write().await.remove(flow_id) {
            self.scheduler.lock().await.remove(&scheduled_flow.job_id).await?;
            tracing::info!("🗑️ Unscheduled flow: {} ({})", scheduled_flow.flow_name, flow_id);
            Ok(())
        } else {
            Err(anyhow!("Flow not scheduled: {}", flow_id))
        }
    }

    /// Get scheduled flow info
    pub async fn get_scheduled_flow(&self, flow_id: &str) -> Option<ScheduledFlow> {
        self.jobs.read().await.get(flow_id).cloned()
    }

    /// List all scheduled flows
    pub async fn list_scheduled_flows(&self) -> Vec<ScheduledFlow> {
        self.jobs.read().await.values().cloned().collect()
    }

    /// Check if flow is scheduled
    pub async fn is_scheduled(&self, flow_id: &str) -> bool {
        self.jobs.read().await.contains_key(flow_id)
    }
}

/// Validate cron expression
pub fn validate_cron(cron_expr: &str) -> Result<()> {
    use std::str::FromStr;
    // Try to parse as a cron expression
    cron::Schedule::from_str(cron_expr)
        .map_err(|e| anyhow!("Invalid cron expression: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cron() {
        assert!(validate_cron("0 0 * * *").is_ok());
        assert!(validate_cron("*/5 * * * *").is_ok());
        assert!(validate_cron("0 9 * * 1").is_ok());
        assert!(validate_cron("invalid").is_err());
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = FlowScheduler::new("UTC").await.unwrap();
        assert!(scheduler.start().await.is_ok());
        assert!(scheduler.shutdown().await.is_ok());
    }
}
