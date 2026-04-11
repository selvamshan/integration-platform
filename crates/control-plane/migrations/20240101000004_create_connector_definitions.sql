CREATE TABLE IF NOT EXISTS connector_definitions (
    id             UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name           VARCHAR(255) NOT NULL UNIQUE,
    connector_type VARCHAR(100) NOT NULL,
    config         JSONB        NOT NULL,
    created_at     TIMESTAMP    DEFAULT CURRENT_TIMESTAMP
);
