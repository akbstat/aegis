-- 20261103135419_create_terminology_versions.sql
--
-- Initial schema for the `terminology_versions` table. Applied
-- by `sqlx migrate run --source lib/crates/terminology/migrations`
-- before the `terminology` crate can be used against PostgreSQL.
--
-- Layout:
--   * `id`         - surrogate primary key (BIGSERIAL).
--   * `kind`       - one of `sdtm` / `adam`. CHECK constraint
--                    mirrors the Rust `TerminologyKind` enum.
--   * `name`       - workbook sheet suffix, e.g. "2026-03-27".
--                    Stored as `String`; not parsed as a date.
--                    (kind, name) is the natural key.
--   * `created_at` - DEFAULT NOW() at insert.
--   * `updated_at` - DEFAULT NOW() at insert; refresh trigger
--                    fires on every UPDATE.

CREATE TABLE terminology_versions (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT terminology_versions_kind_check
        CHECK (kind IN ('sdtm', 'adam')),
    CONSTRAINT terminology_versions_kind_name_unique
        UNIQUE (kind, name)
);

CREATE OR REPLACE FUNCTION terminology_versions_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER terminology_versions_set_updated_at
    BEFORE UPDATE ON terminology_versions
    FOR EACH ROW EXECUTE FUNCTION terminology_versions_set_updated_at();