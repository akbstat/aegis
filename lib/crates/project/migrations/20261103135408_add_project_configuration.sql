-- 20261103135408_add_project_configuration.sql
--
-- Migrate the existing `projects.tags` JSONB column into a new
-- `projects.configuration` JSONB column that carries both the
-- project's locale (`language`) and its tag list (`tags`). The
-- `tags` column is dropped in the same migration; the array-shape
-- CHECK is replaced with an object-shape CHECK on `configuration`.
--
-- Default shape on the new column:
--   {"language": null, "tags": []}
-- so inserts that omit the column land in a coherent state without
-- any application-side patching.
ALTER TABLE projects
ADD COLUMN configuration JSONB NOT NULL DEFAULT '{"language": null, "tags": []}'::jsonb;
UPDATE projects
SET configuration = jsonb_build_object(
        'language',
        NULL,
        'tags',
        COALESCE(tags, '[]'::jsonb)
    );
ALTER TABLE projects DROP COLUMN tags;
ALTER TABLE projects
ADD CONSTRAINT projects_configuration_is_object CHECK (jsonb_typeof(configuration) = 'object');