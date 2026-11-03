-- crf_units: items may carry multiple units; no DB-level uniqueness on item_id.

CREATE TABLE IF NOT EXISTS crf_units (
    id            BIGSERIAL PRIMARY KEY,
    item_id       BIGINT NOT NULL REFERENCES crf_items(id) ON DELETE CASCADE,
    value         TEXT NOT NULL,
    not_submitted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION crf_units_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER crf_units_updated_at
BEFORE UPDATE ON crf_units
FOR EACH ROW EXECUTE FUNCTION crf_units_set_updated_at();
