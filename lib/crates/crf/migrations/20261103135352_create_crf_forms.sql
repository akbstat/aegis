-- crf_forms: one row per (version_id, code). CASCADE removes forms when the version is deleted.

CREATE TABLE IF NOT EXISTS crf_forms (
    id            BIGSERIAL PRIMARY KEY,
    version_id    BIGINT NOT NULL REFERENCES crf_versions(id) ON DELETE CASCADE,
    code          TEXT NOT NULL,
    name          TEXT NOT NULL,
    "order"       INT NOT NULL DEFAULT 0,
    not_submitted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT crf_forms_version_code_unique UNIQUE (version_id, code)
);

CREATE OR REPLACE FUNCTION crf_forms_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER crf_forms_updated_at
BEFORE UPDATE ON crf_forms
FOR EACH ROW EXECUTE FUNCTION crf_forms_set_updated_at();
