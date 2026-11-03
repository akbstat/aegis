-- crf_options: one row per (item_id, value). No DB-level uniqueness on item_id (items may have any number of options).

CREATE TABLE IF NOT EXISTS crf_options (
    id            BIGSERIAL PRIMARY KEY,
    item_id       BIGINT NOT NULL REFERENCES crf_items(id) ON DELETE CASCADE,
    value         TEXT NOT NULL,
    not_submitted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION crf_options_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER crf_options_updated_at
BEFORE UPDATE ON crf_options
FOR EACH ROW EXECUTE FUNCTION crf_options_set_updated_at();
