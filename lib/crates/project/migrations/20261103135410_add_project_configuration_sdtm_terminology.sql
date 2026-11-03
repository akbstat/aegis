-- 20261103135410_add_project_configuration_sdtm_terminology.sql
--
-- Adds the `sdtm_terminology` key to the existing
-- `projects.configuration` JSONB column so the configuration
-- object has a uniform shape:
--   {"language": null, "tags": [], "sdtmig": null, "sdtm_terminology": null}
--
-- Steps:
--   1. Backfill every existing row with `sdtm_terminology: null` so
--      the object shape stays consistent for callers that read the
--      column before any application-side patch lands.
--   2. Update the column default to include
--      `sdtm_terminology: null` so new inserts that omit the column
--      land in a coherent state.
--
-- Mirrors the structure of `20261103135409_add_project_configuration_sdtmig.sql`.
UPDATE projects
SET configuration = configuration || '{"sdtm_terminology": null}'::jsonb;
ALTER TABLE projects
ALTER COLUMN configuration SET DEFAULT '{"language": null, "tags": [], "sdtmig": null, "sdtm_terminology": null}'::jsonb;