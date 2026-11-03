-- crf_versions: one row per (project_code, name).

CREATE TABLE IF NOT EXISTS crf_versions (
    id           BIGSERIAL PRIMARY KEY,
    project_code TEXT NOT NULL,
    name         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT crf_versions_project_name_unique UNIQUE (project_code, name)
);

CREATE OR REPLACE FUNCTION crf_versions_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER crf_versions_updated_at
BEFORE UPDATE ON crf_versions
FOR EACH ROW EXECUTE FUNCTION crf_versions_set_updated_at();
