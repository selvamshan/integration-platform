// crates/control-plane/src/audit.rs

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: String,
    pub entity_name: Option<String>,
    pub action: AuditAction,
    pub status: AuditStatus,
    pub user_id: String,
    pub user_email: Option<String>,
    pub user_role: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub old_values: Option<Value>,
    pub new_values: Option<Value>,
    pub changes: Option<Value>,
    pub parameters: Option<Value>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Execute,
    Enable,
    Disable,
    Test,
    Schedule,
    Unschedule,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuditAction::Create => "CREATE",
            AuditAction::Update => "UPDATE",
            AuditAction::Delete => "DELETE",
            AuditAction::Execute => "EXECUTE",
            AuditAction::Enable => "ENABLE",
            AuditAction::Disable => "DISABLE",
            AuditAction::Test => "TEST",
            AuditAction::Schedule => "SCHEDULE",
            AuditAction::Unschedule => "UNSCHEDULE",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for AuditAction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CREATE" => Ok(AuditAction::Create),
            "UPDATE" => Ok(AuditAction::Update),
            "DELETE" => Ok(AuditAction::Delete),
            "EXECUTE" => Ok(AuditAction::Execute),
            "ENABLE" => Ok(AuditAction::Enable),
            "DISABLE" => Ok(AuditAction::Disable),
            "TEST" => Ok(AuditAction::Test),
            "SCHEDULE" => Ok(AuditAction::Schedule),
            "UNSCHEDULE" => Ok(AuditAction::Unschedule),
            _ => Err(format!("Unknown action: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditStatus {
    Success,
    Failure,
}

impl std::fmt::Display for AuditStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuditStatus::Success => "SUCCESS",
            AuditStatus::Failure => "FAILURE",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for AuditStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SUCCESS" => Ok(AuditStatus::Success),
            "FAILURE" => Ok(AuditStatus::Failure),
            _ => Err(format!("Unknown status: {s}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditLogger {
    db: PgPool,
}

impl AuditLogger {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Log an audit event with full context
    #[allow(clippy::too_many_arguments)]
    pub async fn log(
        &self,
        entity_type: &str,
        entity_id: &str,
        entity_name: Option<&str>,
        action: AuditAction,
        status: AuditStatus,
        user_id: &str,
        user_email: Option<&str>,
        user_role: Option<&str>,
        ip_address: Option<IpAddr>,
        user_agent: Option<&str>,
        request_id: Option<&str>,
        old_values: Option<Value>,
        new_values: Option<Value>,
        parameters: Option<Value>,
        error_message: Option<&str>,
        duration_ms: Option<i32>,
    ) -> Result<Uuid, sqlx::Error> {
        // Calculate changes if both old and new values provided
        let changes = if let (Some(old), Some(new)) = (&old_values, &new_values) {
            Some(calculate_changes(old, new))
        } else {
            None
        };

        let id = Uuid::new_v4();
        let action_str = action.to_string();
        let status_str = status.to_string();

        let ip_address_str = ip_address.map(|ip| ip.to_string());
        sqlx::query!(
            r#"
            INSERT INTO audit_logs (
                id, entity_type, entity_id, entity_name,
                action, status,
                user_id, user_email, user_role,
                ip_address, user_agent, request_id,
                old_values, new_values, changes, parameters,
                error_message, duration_ms
            ) VALUES (
                $1, $2, $3, $4,
                $5, $6,
                $7, $8, $9,
                $10::inet, $11, $12,
                $13, $14, $15, $16,
                $17, $18
            )
            "#,
            id,
            entity_type,
            entity_id,
            entity_name,
            action_str,
            status_str,
            user_id,
            user_email,
            user_role,
            ip_address_str as Option<String>,
            user_agent,
            request_id,
            old_values,
            new_values,
            changes,
            parameters,
            error_message,
            duration_ms,
        )
        .execute(&self.db)
        .await?;

        tracing::debug!(
            "📝 Audit: {} {} {} on {}/{}",
            user_id,
            action,
            status,
            entity_type,
            entity_id
        );

        Ok(id)
    }

    /// Log successful operation
    pub async fn log_success(
        &self,
        entity_type: &str,
        entity_id: &str,
        entity_name: Option<&str>,
        action: AuditAction,
        user_id: &str,
        user_email: Option<&str>,
        new_values: Option<Value>,
        duration_ms: Option<i32>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            entity_type,
            entity_id,
            entity_name,
            action,
            AuditStatus::Success,
            user_id,
            user_email,
            None,
            None,
            None,
            None,
            None,
            new_values,
            None,
            None,
            duration_ms,
        ).await
    }

    /// Log failed operation
    pub async fn log_failure(
        &self,
        entity_type: &str,
        entity_id: &str,
        action: AuditAction,
        user_id: &str,
        error: &str,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            entity_type,
            entity_id,
            None,
            action,
            AuditStatus::Failure,
            user_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(error),
            None,
        ).await
    }

    /// Get audit logs for specific entity
    pub async fn get_logs_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let records = sqlx::query!(
            r#"
            SELECT 
                id, entity_type, entity_id, entity_name,
                action, status,
                user_id, user_email, user_role,
                ip_address::TEXT as ip_address, user_agent, request_id,
                old_values, new_values, changes, parameters,
                error_message, duration_ms, created_at
            FROM audit_logs
            WHERE entity_type = $1 AND entity_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
            entity_type,
            entity_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        records.into_iter().map(|r| -> Result<AuditLog, sqlx::Error> {
            Ok(AuditLog {
                id: r.id,
                entity_type: r.entity_type,
                entity_id: r.entity_id,
                entity_name: r.entity_name,
                action: r.action.parse::<AuditAction>()
                    .map_err(|e| sqlx::Error::Decode(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    )))?,
                status: r.status.parse::<AuditStatus>()
                    .map_err(|e| sqlx::Error::Decode(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    )))?,
                user_id: r.user_id,
                user_email: r.user_email,
                user_role: r.user_role,
                ip_address: r.ip_address.and_then(|s| s.parse().ok()),
                user_agent: r.user_agent,
                request_id: r.request_id,
                old_values: r.old_values,
                new_values: r.new_values,
                changes: r.changes,
                parameters: r.parameters,
                error_message: r.error_message,
                duration_ms: r.duration_ms,
                created_at: r.created_at.unwrap_or_else(Utc::now),
            })
        }).collect()
    }

    /// Get audit logs for user
    pub async fn get_logs_for_user(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let records = sqlx::query!(
            r#"
            SELECT 
                id, entity_type, entity_id, entity_name,
                action, status,
                user_id, user_email, user_role,
                ip_address::TEXT as ip_address, user_agent, request_id,
                old_values, new_values, changes, parameters,
                error_message, duration_ms, created_at
            FROM audit_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        records.into_iter().map(|r| -> Result<AuditLog, sqlx::Error> {
            Ok(AuditLog {
                id: r.id,
                entity_type: r.entity_type,
                entity_id: r.entity_id,
                entity_name: r.entity_name,
                action: r.action.parse::<AuditAction>()
                    .map_err(|e| sqlx::Error::Decode(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    )))?,
                status: r.status.parse::<AuditStatus>()
                    .map_err(|e| sqlx::Error::Decode(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    )))?,
                user_id: r.user_id,
                user_email: r.user_email,
                user_role: r.user_role,
                ip_address: r.ip_address.and_then(|s| s.parse().ok()),
                user_agent: r.user_agent,
                request_id: r.request_id,
                old_values: r.old_values,
                new_values: r.new_values,
                changes: r.changes,
                parameters: r.parameters,
                error_message: r.error_message,
                duration_ms: r.duration_ms,
                created_at: r.created_at.unwrap_or_else(Utc::now),
            })
        }).collect()
    }

    /// Get recent audit logs
    pub async fn get_recent_logs(
        &self,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let records = sqlx::query!(
            r#"
            SELECT 
                id, entity_type, entity_id, entity_name,
                action, status,
                user_id, user_email, user_role,
                ip_address::TEXT as ip_address, user_agent, request_id,
                old_values, new_values, changes, parameters,
                error_message, duration_ms, created_at
            FROM audit_logs
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        records.into_iter().map(|r| -> Result<AuditLog, sqlx::Error> {
            Ok(AuditLog {
                id: r.id,
                entity_type: r.entity_type,
                entity_id: r.entity_id,
                entity_name: r.entity_name,
                action: r.action.parse::<AuditAction>()
                    .map_err(|e| sqlx::Error::Decode(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    )))?,
                status: r.status.parse::<AuditStatus>()
                    .map_err(|e| sqlx::Error::Decode(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    )))?,
                user_id: r.user_id,
                user_email: r.user_email,
                user_role: r.user_role,
                ip_address: r.ip_address.and_then(|s| s.parse().ok()),
                user_agent: r.user_agent,
                request_id: r.request_id,
                old_values: r.old_values,
                new_values: r.new_values,
                changes: r.changes,
                parameters: r.parameters,
                error_message: r.error_message,
                duration_ms: r.duration_ms,
                created_at: r.created_at.unwrap_or_else(Utc::now),
            })
        }).collect()
    }
}

/// Calculate what changed between old and new values
fn calculate_changes(old: &Value, new: &Value) -> Value {
    let mut changes = serde_json::Map::new();

    if let (Some(old_obj), Some(new_obj)) = (old.as_object(), new.as_object()) {
        // Check for changed fields
        for (key, new_val) in new_obj {
            if let Some(old_val) = old_obj.get(key) {
                if old_val != new_val {
                    changes.insert(key.clone(), json!({
                        "old": old_val,
                        "new": new_val
                    }));
                }
            } else {
                // New field added
                changes.insert(key.clone(), json!({
                    "old": null,
                    "new": new_val
                }));
            }
        }

        // Check for removed fields
        for key in old_obj.keys() {
            if !new_obj.contains_key(key) {
                changes.insert(key.clone(), json!({
                    "old": old_obj.get(key),
                    "new": null
                }));
            }
        }
    }

    Value::Object(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_changes() {
        let old = json!({"name": "Old Name", "status": "active", "removed": "field"});
        let new = json!({"name": "New Name", "status": "active", "added": "field"});

        let changes = calculate_changes(&old, &new);
        
        // Name changed
        assert!(changes["name"]["old"] == "Old Name");
        assert!(changes["name"]["new"] == "New Name");
        
        // Status unchanged (not in changes)
        assert!(changes.get("status").is_none());
        
        // Field removed
        assert!(changes.get("removed").is_some());
        
        // Field added
        assert!(changes.get("added").is_some());
    }
}
