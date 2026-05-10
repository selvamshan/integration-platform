CREATE TABLE IF NOT EXISTS projects (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL,
    description TEXT,
    created_at  TIMESTAMP    DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE flow_definitions
    ADD COLUMN IF NOT EXISTS project_id UUID REFERENCES projects(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_flow_definitions_project_id ON flow_definitions (project_id);
