-- crf_items: one row per (form_id, code). `kind` is the source-of-truth enum for CrfItemKind.

CREATE TABLE IF NOT EXISTS crf_items (
    id            BIGSERIAL PRIMARY KEY,
    form_id       BIGINT NOT NULL REFERENCES crf_forms(id) ON DELETE CASCADE,
    code          TEXT NOT NULL,
    name          TEXT NOT NULL,
    kind          TEXT NOT NULL,
    "order"       INT NOT NULL DEFAULT 0,
    not_submitted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT crf_items_form_code_unique UNIQUE (form_id, code),
    CONSTRAINT crf_items_kind_check CHECK (kind IN ('Text','Selection','Checkbox','Datetime','Label'))
);

CREATE OR REPLACE FUNCTION crf_items_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER crf_items_updated_at
BEFORE UPDATE ON crf_items
FOR EACH ROW EXECUTE FUNCTION crf_items_set_updated_at();
