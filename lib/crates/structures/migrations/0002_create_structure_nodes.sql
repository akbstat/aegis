-- structure_nodes: one row per node of a structure.
--
-- At project level these rows are the *extension records* — the
-- difference from the template, not the whole tree.
--
-- `parent_id` is a SOFT reference: it may name a node that lives in the
-- template structure, so it cannot be a database foreign key.

CREATE TABLE IF NOT EXISTS structure_nodes (
    id                  BIGSERIAL PRIMARY KEY,
    structure_id        BIGINT      NOT NULL REFERENCES structures(id) ON DELETE CASCADE,
    node_id             TEXT        NOT NULL,
    description         TEXT        NOT NULL DEFAULT '',
    path                TEXT        NOT NULL,
    directory           BOOLEAN     NOT NULL DEFAULT TRUE,
    parent_id           TEXT,
    hidden              BOOLEAN     NOT NULL DEFAULT FALSE,
    reference_path      TEXT,
    migration_directory TEXT,
    migration_filter    JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT structure_nodes_unique_node UNIQUE (structure_id, node_id),
    CONSTRAINT structure_nodes_unique_path UNIQUE (structure_id, path),
    CONSTRAINT structure_nodes_migration_filter_array CHECK (jsonb_typeof(migration_filter) = 'array'),
    CONSTRAINT structure_nodes_reference_only_on_file CHECK (reference_path IS NULL OR directory = FALSE),
    CONSTRAINT structure_nodes_migration_only_on_directory CHECK (migration_directory IS NULL OR directory = TRUE)
);

CREATE INDEX IF NOT EXISTS structure_nodes_by_parent ON structure_nodes (structure_id, parent_id);

CREATE OR REPLACE FUNCTION structure_nodes_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER structure_nodes_updated_at
BEFORE UPDATE ON structure_nodes
FOR EACH ROW EXECUTE FUNCTION structure_nodes_set_updated_at();
