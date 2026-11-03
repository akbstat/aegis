-- sdtm_project_domain_overrides: at most one override per domain.
-- `UNIQUE (domain_id)` is what enforces "each domain can only hold one
-- override".

CREATE TABLE IF NOT EXISTS sdtm_project_domain_overrides (
    id            BIGSERIAL PRIMARY KEY,
    domain_id     BIGINT      NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE,
    category      TEXT        NOT NULL,
    descriptions  JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_domain_overrides_domain_unique UNIQUE (domain_id),
    CONSTRAINT sdtm_project_domain_overrides_category_check CHECK (
        category IN (
            'Special Purpose',
            'Interventions',
            'Events',
            'Findings',
            'Trial Design',
            'Relationships',
            'Study Reference'
        )
    ),
    CONSTRAINT sdtm_project_domain_overrides_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_domain_overrides_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_domain_overrides_updated_at
BEFORE UPDATE ON sdtm_project_domain_overrides
FOR EACH ROW EXECUTE FUNCTION sdtm_project_domain_overrides_set_updated_at();
