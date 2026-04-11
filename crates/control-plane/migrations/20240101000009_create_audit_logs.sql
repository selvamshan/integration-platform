CREATE TABLE IF NOT EXISTS audit_logs (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Entity information
    entity_type  VARCHAR(50)  NOT NULL,
    entity_id    VARCHAR(255) NOT NULL,
    entity_name  VARCHAR(255),
    -- Action details
    action       VARCHAR(50)  NOT NULL,
    status       VARCHAR(50)  NOT NULL,
    -- User context
    user_id      VARCHAR(255) NOT NULL,
    user_email   VARCHAR(255),
    user_role    VARCHAR(50),
    -- Request context
    ip_address   INET,
    user_agent   TEXT,
    request_id   VARCHAR(255),
    -- Change details
    old_values   JSONB,
    new_values   JSONB,
    changes      JSONB,
    parameters   JSONB,
    -- Result
    error_message TEXT,
    duration_ms  INTEGER,
    -- Timestamp
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_entity         ON audit_logs (entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_user           ON audit_logs (user_id);
CREATE INDEX IF NOT EXISTS idx_audit_created        ON audit_logs (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_action         ON audit_logs (action);
CREATE INDEX IF NOT EXISTS idx_audit_status         ON audit_logs (status);
CREATE INDEX IF NOT EXISTS idx_audit_entity_created ON audit_logs (entity_type, entity_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_entity_action  ON audit_logs (entity_type, action, created_at DESC);

COMMENT ON TABLE audit_logs IS 'Audit trail for all CRUD operations on flows, connectors, and other entities';
COMMENT ON COLUMN audit_logs.entity_type IS 'Type of entity: flow, connector_instance, connector_definition';
COMMENT ON COLUMN audit_logs.action      IS 'Action performed: CREATE, UPDATE, DELETE, EXECUTE, etc.';
COMMENT ON COLUMN audit_logs.status      IS 'Result status: SUCCESS or FAILURE';
COMMENT ON COLUMN audit_logs.changes     IS 'Calculated diff between old_values and new_values';
