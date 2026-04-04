use anyhow::Result;
use sqlx::PgPool;


pub async fn run_migrations(db: &PgPool) -> Result<()> {
    tracing::info!("Running database migrations...");
    
    sqlx::query("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name VARCHAR(255) NOT NULL, email VARCHAR(255) NOT NULL UNIQUE, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS api_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL, version VARCHAR(50) NOT NULL, base_path VARCHAR(255) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, UNIQUE(name, version))").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS flow_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS connector_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL UNIQUE, connector_type VARCHAR(100) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS trigger_definitions (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(255) NOT NULL UNIQUE, trigger_type VARCHAR(100) NOT NULL, config JSONB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(db).await?;
     // ── Auth table ────────────────────────────────────────────────────────────
    sqlx::query("CREATE TABLE IF NOT EXISTS client_credentials (
        client_id          VARCHAR(64)  PRIMARY KEY,
        client_secret_hash TEXT         NOT NULL,
        name               VARCHAR(255) NOT NULL,
        active             BOOLEAN      NOT NULL DEFAULT TRUE,
        created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
        expires_at         TIMESTAMPTZ
    )").execute(db).await?;

    // ── Connector instances table ─────────────────────────────────────────────
    sqlx::query("CREATE TABLE IF NOT EXISTS connector_instances (
        id                  VARCHAR(64)  PRIMARY KEY,
        name                VARCHAR(255) NOT NULL,
        connector_type      VARCHAR(100) NOT NULL,
        host                VARCHAR(255) ,
        port                INTEGER      ,
        database_name       VARCHAR(255) ,
        username            VARCHAR(255) ,
        password_encrypted  TEXT         ,
        extra_attributes    JSONB        NOT NULL DEFAULT '{}',
        active              BOOLEAN      NOT NULL DEFAULT TRUE,
        created_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW()
    )").execute(db).await?;

     // ── User invitations table ────────────────────────────────────────────────
    sqlx::query("CREATE TABLE IF NOT EXISTS user_invitations (
        id              VARCHAR(64)  PRIMARY KEY,
        email           VARCHAR(255) NOT NULL,
        role            VARCHAR(50)  NOT NULL,
        invited_by      VARCHAR(64)  NOT NULL,
        invited_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
        expires_at      TIMESTAMPTZ  NOT NULL,
        token           VARCHAR(64)  NOT NULL UNIQUE,
        accepted        BOOLEAN      NOT NULL DEFAULT FALSE
    )").execute(db).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_invitations_email 
                 ON user_invitations(email)").execute(db).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_invitations_token 
                 ON user_invitations(token)").execute(db).await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS audit_logs (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),    
        -- Entity information
        entity_type VARCHAR(50) NOT NULL,
        entity_id VARCHAR(255) NOT NULL,
        entity_name VARCHAR(255),    
        -- Action details
        action VARCHAR(50) NOT NULL,
        status VARCHAR(50) NOT NULL,    
        -- User context
        user_id VARCHAR(255) NOT NULL,
        user_email VARCHAR(255),
        user_role VARCHAR(50),    
        -- Request context
        ip_address INET,
        user_agent TEXT,
        request_id VARCHAR(255),    
        -- Change details
        old_values JSONB,
        new_values JSONB,
        changes JSONB,
        parameters JSONB,    
        -- Result
        error_message TEXT,
        duration_ms INTEGER,    
        -- Timestamp
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )" ).execute(db).await?;  

    //Create indexes for common queries
    sqlx::query("CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id);")
    .execute(db).await?;
    sqlx::query("CREATE INDEX idx_audit_user ON audit_logs(user_id)")
    .execute(db).await?;
    sqlx::query("CREATE INDEX idx_audit_created ON audit_logs(created_at DESC)")
    .execute(db).await?;
    sqlx::query("CREATE INDEX idx_audit_action ON audit_logs(action)")
    .execute(db).await?;
    sqlx::query("CREATE INDEX idx_audit_status ON audit_logs(status)")
    .execute(db).await?;
    sqlx::query("CREATE INDEX idx_audit_entity_created ON audit_logs(entity_type, entity_id, created_at DESC)")
    .execute(db).await?;

    //Create composite index for common filter combinations
    sqlx::query("CREATE INDEX idx_audit_entity_action ON audit_logs(entity_type, action, created_at DESC)")
    .execute(db).await?;

    // Comment the table
    sqlx::query("COMMENT ON TABLE audit_logs IS 'Audit trail for all CRUD operations on flows, connectors, and other entities'")
    .execute(db).await?;
    sqlx::query("COMMENT ON COLUMN audit_logs.entity_type IS 'Type of entity: flow, connector_instance, connector_definition'")
    .execute(db).await?;
    sqlx::query("COMMENT ON COLUMN audit_logs.action IS 'Action performed: CREATE, UPDATE, DELETE, EXECUTE, etc.'")
    .execute(db).await?;
    sqlx::query("COMMENT ON COLUMN audit_logs.status IS 'Result status: SUCCESS or FAILURE'")
    .execute(db).await?;
    sqlx::query("COMMENT ON COLUMN audit_logs.changes IS 'Calculated diff between old_values and new_values'")
    .execute(db).await?;        

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(db).await?;
    if count.0 == 0 {
        sqlx::query("INSERT INTO users (name, email) VALUES ('Alice Johnson', 'alice@example.com'), ('Bob Smith', 'bob@example.com'), ('Charlie Brown', 'charlie@example.com'), ('Diana Prince', 'diana@example.com'), ('Eve Wilson', 'eve@example.com')").execute(db).await?;
        tracing::info!("✅ Sample data inserted");
    }
    
    tracing::info!("✅ Migrations completed");
    Ok(())
}