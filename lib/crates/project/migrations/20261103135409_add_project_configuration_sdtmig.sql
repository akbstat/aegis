-- 20261103135409_add_project_configuration_sdtmig.sql
--
-- Adds the `sdtmig` key to the existing `projects.configuration`
-- JSONB column so the configuration object has a uniform shape:
--   {"language": null, "tags": [], "sdtmig": null}
--
-- Steps:
--   1. Backfill every existing row with `sdtmig: null` so the
--      object shape stays consistent for callers that read the
--      column before any application-side patch lands.
--   2. Update the column default to include `sdtmig: null` so new
--      inserts that omit the column land in a coherent state.
--
-- Mirrors the structure of `20261103135408_add_project_configuration.sql`.
UPDATE projects
SET configuration = configuration || '{"sdtmig": null}'::jsonb;
ALTER TABLE projects
ALTER COLUMN configuration SET DEFAULT '{"language": null, "tags": [], "sdtmig": null}'::jsonb;
