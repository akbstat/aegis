-- structures: one row per directory layout.
--
-- `project_code` is a soft reference to `projects.code` from the
-- `project` crate; it is validated at the application level, not by a
-- database foreign key, so the two crates' schemas stay independent.
--
-- `template_id` references another row in this table and IS a foreign
-- key: a template cannot be deleted while a project structure points
-- at it.

CREATE TABLE IF NOT EXISTS structures (
    id           BIGSERIAL PRIMARY KEY,
    name         TEXT        NOT NULL,
    level        TEXT        NOT NULL,
    project_code TEXT,
    template_id  BIGINT      REFERENCES structures(id) ON DELETE RESTRICT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT structures_level_check CHECK (level IN ('template', 'project')),
    CONSTRAINT structures_level_shape CHECK (
        (level = 'template' AND project_code IS NULL AND template_id IS NULL) OR
        (level = 'project'  AND project_code IS NOT NULL AND template_id IS NOT NULL)
    )
);

-- Exactly one project-level structure per project.
CREATE UNIQUE INDEX IF NOT EXISTS structures_one_per_project
    ON structures (project_code) WHERE level = 'project';

CREATE OR REPLACE FUNCTION structures_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER structures_updated_at
BEFORE UPDATE ON structures
FOR EACH ROW EXECUTE FUNCTION structures_set_updated_at();
