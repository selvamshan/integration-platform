# Audit Log Implementation - Flow & Connector CRUD

Complete audit trail for all operations on flows and connectors.

---

## Overview

Track all changes to flows and connectors with:
- **Who** - User who made the change
- **What** - Action performed (CREATE, UPDATE, DELETE, EXECUTE)
- **When** - Timestamp
- **Where** - IP address, user agent
- **Details** - Before/after values, parameters

---

## Database Schema

### Audit Log Table

```sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Entity information
    entity_type VARCHAR(50) NOT NULL,  -- 'flow', 'connector_instance', 'connector_definition'
    entity_id VARCHAR(255) NOT NULL,
    entity_name VARCHAR(255),
    
    -- Action details
    action VARCHAR(50) NOT NULL,  -- 'CREATE', 'UPDATE', 'DELETE', 'EXECUTE', 'ENABLE', 'DISABLE'
    status VARCHAR(50) NOT NULL,  -- 'SUCCESS', 'FAILURE'
    
    -- User context
    user_id VARCHAR(255) NOT NULL,
    user_email VARCHAR(255),
    user_role VARCHAR(50),
    
    -- Request context
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(255),
    
    -- Change details
    old_values JSONB,  -- State before change
    new_values JSONB,  -- State after change
    changes JSONB,     -- Specific fields changed
    parameters JSONB,  -- Request parameters
    
    -- Result
    error_message TEXT,
    duration_ms INTEGER,
    
    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Indexes
    INDEX idx_audit_entity (entity_type, entity_id),
    INDEX idx_audit_user (user_id),
    INDEX idx_audit_created (created_at DESC),
    INDEX idx_audit_action (action),
    INDEX idx_audit_entity_created (entity_type, entity_id, created_at DESC)
);

-- Partition by month for performance
CREATE TABLE audit_logs_y2024m03 PARTITION OF audit_logs
    FOR VALUES FROM ('2024-03-01') TO ('2024-04-01');
```

### Migration

```sql
-- migrations/000X_create_audit_logs.sql

-- Create audit logs table
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type VARCHAR(50) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,
    entity_name VARCHAR(255),
    action VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    user_email VARCHAR(255),
    user_role VARCHAR(50),
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(255),
    old_values JSONB,
    new_values JSONB,
    changes JSONB,
    parameters JSONB,
    error_message TEXT,
    duration_ms INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_user ON audit_logs(user_id);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_action ON audit_logs(action);

-- Create retention policy (optional - keep 1 year)
CREATE INDEX idx_audit_retention ON audit_logs(created_at)
    WHERE created_at < NOW() - INTERVAL '1 year';
```

---

## Rust Implementation

### Audit Logger Struct

Create `crates/control-plane/src/audit.rs`:

```rust
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct AuditLogger {
    db: PgPool,
}

impl AuditLogger {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Log an audit event
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
                $10, $11, $12,
                $13, $14, $15, $16,
                $17, $18
            )
            "#,
            id,
            entity_type,
            entity_id,
            entity_name,
            serde_json::to_string(&action).unwrap(),
            serde_json::to_string(&status).unwrap(),
            user_id,
            user_email,
            user_role,
            ip_address,
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
            "📝 Audit log created: {} {} on {}/{}",
            action,
            status,
            entity_type,
            entity_id
        );

        Ok(id)
    }

    /// Convenience method for successful operations
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

    /// Convenience method for failed operations
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

    /// Get audit logs for an entity
    pub async fn get_logs_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let records = sqlx::query_as!(
            AuditLogRecord,
            r#"
            SELECT *
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

        Ok(records.into_iter().map(|r| r.into()).collect())
    }

    /// Get audit logs for a user
    pub async fn get_logs_for_user(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let records = sqlx::query_as!(
            AuditLogRecord,
            r#"
            SELECT *
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

        Ok(records.into_iter().map(|r| r.into()).collect())
    }

    /// Get recent audit logs
    pub async fn get_recent_logs(
        &self,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let records = sqlx::query_as!(
            AuditLogRecord,
            r#"
            SELECT *
            FROM audit_logs
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(records.into_iter().map(|r| r.into()).collect())
    }
}

/// Calculate what changed between old and new values
fn calculate_changes(old: &Value, new: &Value) -> Value {
    let mut changes = serde_json::Map::new();

    if let (Some(old_obj), Some(new_obj)) = (old.as_object(), new.as_object()) {
        for (key, new_val) in new_obj {
            if let Some(old_val) = old_obj.get(key) {
                if old_val != new_val {
                    changes.insert(key.clone(), json!({
                        "old": old_val,
                        "new": new_val
                    }));
                }
            } else {
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

// Database record struct
#[derive(sqlx::FromRow)]
struct AuditLogRecord {
    id: Uuid,
    entity_type: String,
    entity_id: String,
    entity_name: Option<String>,
    action: String,
    status: String,
    user_id: String,
    user_email: Option<String>,
    user_role: Option<String>,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
    request_id: Option<String>,
    old_values: Option<Value>,
    new_values: Option<Value>,
    changes: Option<Value>,
    parameters: Option<Value>,
    error_message: Option<String>,
    duration_ms: Option<i32>,
    created_at: DateTime<Utc>,
}

impl From<AuditLogRecord> for AuditLog {
    fn from(r: AuditLogRecord) -> Self {
        Self {
            id: r.id,
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            entity_name: r.entity_name,
            action: serde_json::from_str(&r.action).unwrap(),
            status: serde_json::from_str(&r.status).unwrap(),
            user_id: r.user_id,
            user_email: r.user_email,
            user_role: r.user_role,
            ip_address: r.ip_address,
            user_agent: r.user_agent,
            request_id: r.request_id,
            old_values: r.old_values,
            new_values: r.new_values,
            changes: r.changes,
            parameters: r.parameters,
            error_message: r.error_message,
            duration_ms: r.duration_ms,
            created_at: r.created_at,
        }
    }
}
```

---

## Integration into Handlers

### Flow CRUD with Audit Logging

```rust
use crate::audit::{AuditLogger, AuditAction};

/// CREATE Flow
async fn create_flow(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let start_time = std::time::Instant::now();
    let flow_id = Uuid::new_v4().to_string();

    // Create flow
    let flow = Flow {
        id: flow_id.clone(),
        name: payload["name"].as_str().unwrap().to_string(),
        // ... other fields
    };

    match create_flow_in_db(&flow).await {
        Ok(_) => {
            // Log successful creation
            state.audit_logger.log_success(
                "flow",
                &flow_id,
                Some(&flow.name),
                AuditAction::Create,
                &user.id,
                user.email.as_deref(),
                Some(serde_json::to_value(&flow).unwrap()),
                Some(start_time.elapsed().as_millis() as i32),
            ).await.ok();

            Ok(Json(json!(flow)))
        }
        Err(e) => {
            // Log failure
            state.audit_logger.log_failure(
                "flow",
                &flow_id,
                AuditAction::Create,
                &user.id,
                &e.to_string(),
            ).await.ok();

            Err(AppError::Internal(e.to_string()))
        }
    }
}

/// UPDATE Flow
async fn update_flow(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let start_time = std::time::Instant::now();

    // Get old values
    let old_flow = get_flow_from_db(&id).await?;
    let old_values = serde_json::to_value(&old_flow).unwrap();

    // Update flow
    let updated_flow = update_flow_in_db(&id, payload).await?;
    let new_values = serde_json::to_value(&updated_flow).unwrap();

    // Log update with before/after values
    state.audit_logger.log(
        "flow",
        &id,
        Some(&updated_flow.name),
        AuditAction::Update,
        AuditStatus::Success,
        &user.id,
        user.email.as_deref(),
        user.role.as_deref(),
        None,
        None,
        None,
        Some(old_values),
        Some(new_values),
        None,
        None,
        Some(start_time.elapsed().as_millis() as i32),
    ).await.ok();

    Ok(Json(json!(updated_flow)))
}

/// DELETE Flow
async fn delete_flow(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let start_time = std::time::Instant::now();

    // Get flow before deletion
    let flow = get_flow_from_db(&id).await?;
    let old_values = serde_json::to_value(&flow).unwrap();

    // Delete
    delete_flow_from_db(&id).await?;

    // Log deletion
    state.audit_logger.log(
        "flow",
        &id,
        Some(&flow.name),
        AuditAction::Delete,
        AuditStatus::Success,
        &user.id,
        user.email.as_deref(),
        user.role.as_deref(),
        None,
        None,
        None,
        Some(old_values),
        None,
        None,
        None,
        Some(start_time.elapsed().as_millis() as i32),
    ).await.ok();

    Ok(Json(json!({"message": "Flow deleted"})))
}

/// EXECUTE Flow
async fn execute_flow(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let start_time = std::time::Instant::now();

    match execute_flow_logic(&id, payload.clone()).await {
        Ok(result) => {
            state.audit_logger.log(
                "flow",
                &id,
                None,
                AuditAction::Execute,
                AuditStatus::Success,
                &user.id,
                user.email.as_deref(),
                None,
                None,
                None,
                None,
                None,
                Some(result.clone()),
                Some(payload),
                None,
                Some(start_time.elapsed().as_millis() as i32),
            ).await.ok();

            Ok(Json(result))
        }
        Err(e) => {
            state.audit_logger.log(
                "flow",
                &id,
                None,
                AuditAction::Execute,
                AuditStatus::Failure,
                &user.id,
                user.email.as_deref(),
                None,
                None,
                None,
                None,
                None,
                None,
                Some(payload),
                Some(&e.to_string()),
                Some(start_time.elapsed().as_millis() as i32),
            ).await.ok();

            Err(AppError::Internal(e.to_string()))
        }
    }
}
```

---

## Audit Log API Endpoints

```rust
/// GET /audit-logs
async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<Value>, AppError> {
    // Only admins can view all audit logs
    if user.role != "admin" {
        return Err(AppError::Forbidden);
    }

    let logs = if let Some(entity_id) = params.entity_id {
        state.audit_logger.get_logs_for_entity(
            &params.entity_type.unwrap_or("flow".to_string()),
            &entity_id,
            params.limit.unwrap_or(100),
        ).await?
    } else if let Some(user_id) = params.user_id {
        state.audit_logger.get_logs_for_user(
            &user_id,
            params.limit.unwrap_or(100),
        ).await?
    } else {
        state.audit_logger.get_recent_logs(
            params.limit.unwrap_or(100),
        ).await?
    };

    Ok(Json(json!({
        "logs": logs,
        "count": logs.len()
    })))
}

/// GET /flows/:id/audit-logs
async fn get_flow_audit_logs(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let logs = state.audit_logger.get_logs_for_entity(
        "flow",
        &id,
        100,
    ).await?;

    Ok(Json(json!({
        "flow_id": id,
        "logs": logs,
        "count": logs.len()
    })))
}

/// GET /connector-instances/:id/audit-logs
async fn get_connector_audit_logs(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let logs = state.audit_logger.get_logs_for_entity(
        "connector_instance",
        &id,
        100,
    ).await?;

    Ok(Json(json!({
        "connector_id": id,
        "logs": logs,
        "count": logs.len()
    })))
}

#[derive(Deserialize)]
struct AuditQueryParams {
    entity_type: Option<String>,
    entity_id: Option<String>,
    user_id: Option<String>,
    limit: Option<i64>,
}
```

---

## Summary

✅ **Complete audit trail** for all CRUD operations  
✅ **Before/after tracking** with change detection  
✅ **User context** including IP and user agent  
✅ **Performance tracking** with duration  
✅ **Queryable** by entity, user, or time  
✅ **Retention policies** with partitioning  

**All changes are now tracked and auditable!** 📝✅
