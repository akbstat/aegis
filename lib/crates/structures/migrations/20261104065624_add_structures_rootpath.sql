-- structures.rootpath: the absolute base directory the structure's
-- tree is rooted at.
--
-- The column arrives NOT NULL with a default so the ALTER succeeds
-- against a table that already holds rows; the default is dropped
-- immediately afterwards, so every new row must supply a root path.
-- Rows created before this change carry an empty root path until an
-- operator sets one through the update endpoint.
--
-- The blank check lives in the domain (`Structure::new`), not here --
-- the same split the `name` column uses.

ALTER TABLE structures
    ADD COLUMN rootpath TEXT NOT NULL DEFAULT '';

ALTER TABLE structures
    ALTER COLUMN rootpath DROP DEFAULT;
