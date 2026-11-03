-- Adds the `approved` flag to crf_forms. Defaults to FALSE so every
-- pre-existing row counts as not-yet-approved. Idempotent so re-runs
-- on a partially-migrated database are safe.
ALTER TABLE crf_forms
    ADD COLUMN IF NOT EXISTS approved BOOLEAN NOT NULL DEFAULT FALSE;