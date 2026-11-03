-- sdtm_versions: one row per published CDISC SDTM release sheet.
-- Identified by `name` (e.g. `2024-09-27`); `id` is the surrogate key.

CREATE TABLE IF NOT EXISTS sdtm_versions (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT      NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_versions_name_unique UNIQUE (name)
);

-- updated_at trigger
CREATE OR REPLACE FUNCTION sdtm_versions_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_versions_updated_at
BEFORE UPDATE ON sdtm_versions
FOR EACH ROW EXECUTE FUNCTION sdtm_versions_set_updated_at();
