CREATE TABLE IF NOT EXISTS api_definitions (
    id         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name       VARCHAR(255) NOT NULL,
    version    VARCHAR(50)  NOT NULL,
    base_path  VARCHAR(255) NOT NULL,
    config     JSONB        NOT NULL,
    created_at TIMESTAMP    DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (name, version)
);
