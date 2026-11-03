-- sdtm_project_variable_overrides: at most one override per variable.

CREATE TABLE IF NOT EXISTS sdtm_project_variable_overrides (
    id             BIGSERIAL PRIMARY KEY,
    variable_id    BIGINT      NOT NULL REFERENCES sdtm_project_variables(id) ON DELETE CASCADE,
    variable_type  TEXT        NOT NULL,
    variable_core  TEXT        NOT NULL,
    variable_role  TEXT,
    descriptions   JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_variable_overrides_variable_unique UNIQUE (variable_id),
    CONSTRAINT sdtm_project_variable_overrides_type_check CHECK (
        variable_type IN ('Numeric', 'Character')
    ),
    CONSTRAINT sdtm_project_variable_overrides_core_check CHECK (
        variable_core IN ('Req', 'Exp', 'Perm', 'Supp')
    ),
    CONSTRAINT sdtm_project_variable_overrides_role_check CHECK (
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
    ),
    CONSTRAINT sdtm_project_variable_overrides_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_variable_overrides_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_variable_overrides_updated_at
BEFORE UPDATE ON sdtm_project_variable_overrides
FOR EACH ROW EXECUTE FUNCTION sdtm_project_variable_overrides_set_updated_at();
