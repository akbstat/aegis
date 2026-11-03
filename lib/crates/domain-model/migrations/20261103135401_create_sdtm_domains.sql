-- sdtm_domains: one row per (version, domain). The descriptions
-- column is a single JSONB blob carrying a Vec<SdtmDomainDescription>.

CREATE TABLE IF NOT EXISTS sdtm_domains (
    id            BIGSERIAL PRIMARY KEY,
    version_id    BIGINT      NOT NULL REFERENCES sdtm_versions(id) ON DELETE CASCADE,
    name          TEXT        NOT NULL,
    category      TEXT        NOT NULL,
    descriptions  JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_domains_version_name_unique UNIQUE (version_id, name),
    CONSTRAINT sdtm_domains_category_check CHECK (
        category IN (
            'Special Purpose',
            'Interventions',
            'Events',
            'Findings',
            'Trial Design',
            'Relationships',
            'Study Reference'
        )
    )
);

CREATE OR REPLACE FUNCTION sdtm_domains_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_domains_updated_at
BEFORE UPDATE ON sdtm_domains
FOR EACH ROW EXECUTE FUNCTION sdtm_domains_set_updated_at();
