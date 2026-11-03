-- sdtm_project_versions: one row per (project, version name).
-- `project_code` is a soft reference to `projects.code` from the
-- `project` crate; it is validated at the application level, not by a
-- database foreign key, so the two crates' schemas stay independent.

CREATE TABLE IF NOT EXISTS sdtm_project_versions (
    id            BIGSERIAL PRIMARY KEY,
    project_code  TEXT        NOT NULL,
    version_name  TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_versions_project_name_unique UNIQUE (project_code, version_name)
);

CREATE OR REPLACE FUNCTION sdtm_project_versions_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_versions_updated_at
BEFORE UPDATE ON sdtm_project_versions
FOR EACH ROW EXECUTE FUNCTION sdtm_project_versions_set_updated_at();
