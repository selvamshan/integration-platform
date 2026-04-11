CREATE TABLE IF NOT EXISTS trigger_definitions (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name         VARCHAR(255) NOT NULL UNIQUE,
    trigger_type VARCHAR(100) NOT NULL,
    config       JSONB        NOT NULL,
    created_at   TIMESTAMP    DEFAULT CURRENT_TIMESTAMP
);
