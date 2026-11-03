-- sdtm_variables: one row per (domain, variable). descriptions
-- is a single JSONB blob carrying a Vec<SdtmVariableDescription>.

CREATE TABLE IF NOT EXISTS sdtm_variables (
    id                   BIGSERIAL PRIMARY KEY,
    domain_id            BIGINT      NOT NULL REFERENCES sdtm_domains(id) ON DELETE CASCADE,
    name                 TEXT        NOT NULL,
    variable_controlled  TEXT,
    variable_type        TEXT        NOT NULL,
    variable_core        TEXT        NOT NULL,
    variable_role        TEXT,
    variable_sequence    BIGINT      NOT NULL,
    descriptions         JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_variables_domain_name_unique UNIQUE (domain_id, name),
    CONSTRAINT sdtm_variables_type_check CHECK (
        variable_type IN ('Numeric', 'Character')
    ),
    CONSTRAINT sdtm_variables_core_check CHECK (
        variable_core IN ('Req', 'Exp', 'Perm', 'Supp')
    ),
    CONSTRAINT sdtm_variables_role_check CHECK (
        variable_role IS NULL OR variable_role IN (
            'Identifier',
            'Topic',
            'Timing',
            'Record Qualifier',
            'Synonym Qualifier',
            'Variable Qualifier',
            'Grouping Qualifier',
            'Rule'
        )
    )
);

CREATE OR REPLACE FUNCTION sdtm_variables_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_variables_updated_at
BEFORE UPDATE ON sdtm_variables
FOR EACH ROW EXECUTE FUNCTION sdtm_variables_set_updated_at();
