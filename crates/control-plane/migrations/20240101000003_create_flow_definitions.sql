CREATE TABLE IF NOT EXISTS flow_definitions (
    id         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name       VARCHAR(255) NOT NULL,
    config     JSONB        NOT NULL,
    created_at TIMESTAMP    DEFAULT CURRENT_TIMESTAMP
);
