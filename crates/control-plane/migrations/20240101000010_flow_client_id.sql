ALTER TABLE flow_definitions
    ADD COLUMN IF NOT EXISTS client_id VARCHAR(255) REFERENCES client_credentials(client_id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_flow_definitions_client_id ON flow_definitions (client_id);
