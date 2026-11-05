-- Drop the `active` flag from `project_branches`.
--
-- A project branch is now a plain named row: the "at most one active
-- branch per project" switch is gone, so neither the flag nor the
-- partial unique index that backed it is needed.
--
-- The index is dropped explicitly first (the column drop would take
-- it down implicitly anyway) so the intent is readable.

DROP INDEX IF EXISTS project_branches_one_active_per_project;

ALTER TABLE project_branches DROP COLUMN IF EXISTS active;
