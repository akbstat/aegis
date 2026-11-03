-- sdtm_project_domains: one row per (version, domain code).
-- `key_variables` is a JSONB array of variable codes kept in the order
-- the caller supplied.

CREATE TABLE IF NOT EXISTS sdtm_project_domains (
    id             BIGSERIAL PRIMARY KEY,
    version_id     BIGINT      NOT NULL REFERENCES sdtm_project_versions(id) ON DELETE CASCADE,
    domain_code    TEXT        NOT NULL,
    key_variables  JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (version_id, domain_code),
    CONSTRAINT sdtm_project_domains_key_variables_is_array CHECK (jsonb_typeof(key_variables) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_domains_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_domains_updated_at
BEFORE UPDATE ON sdtm_project_domains
FOR EACH ROW EXECUTE FUNCTION sdtm_project_domains_set_updated_at();
