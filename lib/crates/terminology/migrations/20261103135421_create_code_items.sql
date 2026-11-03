-- 20261103135421_create_code_items.sql
--
-- Permissible values inside each codelist, one row per
-- (codelist, code) pair.
--
-- Layout mirrors code_lists:
--   * FK to code_lists(id) ON DELETE CASCADE.
--   * UNIQUE (codelist_id, code).
--   * `tsv` GENERATED over the same four text columns as the
--     search port (unweighted — every column contributes equally
--     to `ts_rank`), plus a GIN index.
--   * `code_items_set_updated_at` trigger refreshes
--     `updated_at` on UPDATE.
--
-- `version_id` is a denormalised copy of the parent codelist's
-- `version_id`. It is the leading column of the composite index
-- `code_items_version_id_code_idx` that backs the natural-key
-- lookup `list_by_version_and_code`. The composite index also
-- serves queries on `version_id` alone, so no separate
-- `(version_id)` index is needed. The column is populated by the
-- adapter on insert and is not a foreign key by itself (the parent
-- `code_lists` row is the source of truth).

CREATE TABLE code_items (
    id BIGSERIAL PRIMARY KEY,
    codelist_id BIGINT NOT NULL REFERENCES code_lists(id) ON DELETE CASCADE,
    version_id BIGINT NOT NULL,
    code TEXT NOT NULL,
    submission_value TEXT NOT NULL DEFAULT '',
    synonym TEXT NOT NULL DEFAULT '',
    definition TEXT NOT NULL DEFAULT '',
    nci_preferred_term TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', coalesce(submission_value, '') || ' ' || coalesce(synonym, '') || ' ' || coalesce(definition, '') || ' ' || coalesce(nci_preferred_term, ''))) STORED,
    CONSTRAINT code_items_codelist_code_unique UNIQUE (codelist_id, code)
);

CREATE INDEX code_items_codelist_id_idx ON code_items (codelist_id);
CREATE INDEX code_items_version_id_code_idx ON code_items (version_id, code);
CREATE INDEX code_items_tsv_idx ON code_items USING GIN (tsv);

CREATE OR REPLACE FUNCTION code_items_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER code_items_set_updated_at
    BEFORE UPDATE ON code_items
    FOR EACH ROW EXECUTE FUNCTION code_items_set_updated_at();