CREATE TABLE IF NOT EXISTS client_credentials (
    client_id          VARCHAR(64)  PRIMARY KEY,
    client_secret_hash TEXT         NOT NULL,
    name               VARCHAR(255) NOT NULL,
    active             BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    expires_at         TIMESTAMPTZ
);
