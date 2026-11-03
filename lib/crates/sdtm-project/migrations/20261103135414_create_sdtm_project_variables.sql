-- sdtm_project_variables: one row per (domain, variable code).

CREATE TABLE IF NOT EXISTS sdtm_project_variables (
    id                BIGSERIAL PRIMARY KEY,
    domain_id         BIGINT      NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE,
    variable_code     TEXT        NOT NULL,
    variable_sequence BIGINT      NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (domain_id, variable_code)
);

CREATE OR REPLACE FUNCTION sdtm_project_variables_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_variables_updated_at
BEFORE UPDATE ON sdtm_project_variables
FOR EACH ROW EXECUTE FUNCTION sdtm_project_variables_set_updated_at();
