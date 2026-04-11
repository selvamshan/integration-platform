CREATE TABLE IF NOT EXISTS connector_instances (
    id                 VARCHAR(64)  PRIMARY KEY,
    name               VARCHAR(255) NOT NULL,
    connector_type     VARCHAR(100) NOT NULL,
    host               VARCHAR(255),
    port               INTEGER,
    database_name      VARCHAR(255),
    username           VARCHAR(255),
    password_encrypted TEXT,
    extra_attributes   JSONB        NOT NULL DEFAULT '{}',
    active             BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
