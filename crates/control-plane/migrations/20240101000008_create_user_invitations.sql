CREATE TABLE IF NOT EXISTS user_invitations (
    id         VARCHAR(64)  PRIMARY KEY,
    email      VARCHAR(255) NOT NULL,
    role       VARCHAR(50)  NOT NULL,
    invited_by VARCHAR(64)  NOT NULL,
    invited_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ  NOT NULL,
    token      VARCHAR(64)  NOT NULL UNIQUE,
    accepted   BOOLEAN      NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_invitations_email ON user_invitations (email);
CREATE INDEX IF NOT EXISTS idx_invitations_token ON user_invitations (token);
