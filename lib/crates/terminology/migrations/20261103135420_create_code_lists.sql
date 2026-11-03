-- 20261103135420_create_code_lists.sql
--
-- CDISC codelists, one row per (version, code) pair. Items live
-- in a separate `code_items` table (migration 0003).
--
-- Layout:
--   * `id`               - surrogate primary key.
--   * `version_id`       - FK to terminology_versions(id)
--                          ON DELETE CASCADE.
--   * `code`             - NCI C-code of the codelist. UNIQUE
--                          per version.
--   * `extensible`       - whether sponsors may add new
--                          permissible values.
--   * `name`, `submission_value`, `synonym`, `definition`,
--     `nci_preferred_term` - five text columns surfaced through
--                            full-text search. The search port
--                            treats all five uniformly — there is
--                            no per-column weight.
--   * `created_at`, `updated_at` - DEFAULT NOW(); trigger
--                            refreshes updated_at on UPDATE.
--   * `tsv`              - GENERATED tsvector over the five
--                            text columns (unweighted). GIN index
--                            backs the search port.

CREATE TABLE code_lists (
    id BIGSERIAL PRIMARY KEY,
    version_id BIGINT NOT NULL REFERENCES terminology_versions(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    extensible BOOLEAN NOT NULL,
    name TEXT NOT NULL,
    submission_value TEXT NOT NULL DEFAULT '',
    synonym TEXT NOT NULL DEFAULT '',
    definition TEXT NOT NULL DEFAULT '',
    nci_preferred_term TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', coalesce(name, '') || ' ' || coalesce(submission_value, '') || ' ' || coalesce(synonym, '') || ' ' || coalesce(definition, '') || ' ' || coalesce(nci_preferred_term, ''))) STORED,
    CONSTRAINT code_lists_version_code_unique UNIQUE (version_id, code)
);

CREATE INDEX code_lists_version_id_idx ON code_lists (version_id);
CREATE INDEX code_lists_tsv_idx ON code_lists USING GIN (tsv);

CREATE OR REPLACE FUNCTION code_lists_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER code_lists_set_updated_at
    BEFORE UPDATE ON code_lists
    FOR EACH ROW EXECUTE FUNCTION code_lists_set_updated_at();