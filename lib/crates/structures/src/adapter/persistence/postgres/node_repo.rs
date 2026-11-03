//! PostgreSQL implementation of [`NodeRepository`].
//!
//! SQLx **runtime API** (`sqlx::query_as` / `sqlx::query`) is used
//! throughout: the workspace ships no `.sqlx/` offline cache and no
//! `DATABASE_URL` at build time, so the compile-time-checked macros are
//! intentionally avoided.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{DomainError, Migration, Node, NodeAttributes, NodeRepository};

/// A row of the `structure_nodes` table.
#[derive(sqlx::FromRow)]
struct NodeRow {
    node_id: String,
    description: String,
    path: String,
    directory: bool,
    parent_id: Option<String>,
    hidden: bool,
    reference_path: Option<String>,
    migration_directory: Option<String>,
    migration_filter: serde_json::Value,
}

impl TryFrom<NodeRow> for Node {
    type Error = DomainError;

    fn try_from(row: NodeRow) -> Result<Self, Self::Error> {
        let migration = match row.migration_directory {
            Some(directory) => {
                let filter = serde_json::from_value::<Vec<String>>(row.migration_filter)
                    .map_err(|e| DomainError::Repository(e.to_string()))?;
                Some(Migration { directory, filter })
            }
            None => None,
        };
        let attrs = NodeAttributes {
            description: row.description,
            path: row.path,
            directory: row.directory,
            parent: row.parent_id,
            reference: row.reference_path,
            migration,
        };
        let mut node = Node::for_repository(row.node_id, attrs);
        node.hidden = Some(row.hidden);
        Ok(node)
    }
}

/// PostgreSQL-backed `NodeRepository`.
pub struct NodeRepoPg {
    pool: PgPool,
}

impl NodeRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NodeRepository for NodeRepoPg {
    async fn insert(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError> {
        let row = sqlx::query_as::<_, NodeRow>(
            "INSERT INTO structure_nodes
                 (structure_id, node_id, description, path, directory, parent_id, hidden,
                  reference_path, migration_directory, migration_filter)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             RETURNING node_id, description, path, directory, parent_id, hidden,
                       reference_path, migration_directory, migration_filter",
        )
        .bind(structure_id)
        .bind(&node.node_id)
        .bind(&node.description)
        .bind(&node.path)
        .bind(node.directory)
        .bind(&node.parent)
        .bind(node.hidden.unwrap_or(false))
        .bind(&node.reference)
        .bind(node.migration.as_ref().map(|m| m.directory.clone()))
        .bind(
            node.migration
                .as_ref()
                .map(|m| serde_json::to_value(&m.filter))
                .transpose()
                .map_err(|e| DomainError::Repository(e.to_string()))?
                .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_error)?;
        Node::try_from(row)
    }

    async fn update(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError> {
        let row = sqlx::query_as::<_, NodeRow>(
            "UPDATE structure_nodes
                SET description = $3, path = $4, directory = $5, parent_id = $6, hidden = $7,
                    reference_path = $8, migration_directory = $9, migration_filter = $10
              WHERE structure_id = $1 AND node_id = $2
             RETURNING node_id, description, path, directory, parent_id, hidden,
                       reference_path, migration_directory, migration_filter",
        )
        .bind(structure_id)
        .bind(&node.node_id)
        .bind(&node.description)
        .bind(&node.path)
        .bind(node.directory)
        .bind(&node.parent)
        .bind(node.hidden.unwrap_or(false))
        .bind(&node.reference)
        .bind(node.migration.as_ref().map(|m| m.directory.clone()))
        .bind(
            node.migration
                .as_ref()
                .map(|m| serde_json::to_value(&m.filter))
                .transpose()
                .map_err(|e| DomainError::Repository(e.to_string()))?
                .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?
        .ok_or_else(|| DomainError::NodeNotFound(node.node_id.clone()))?;
        Node::try_from(row)
    }

    async fn delete(&self, structure_id: i64, node_id: &str) -> Result<(), DomainError> {
        let affected =
            sqlx::query("DELETE FROM structure_nodes WHERE structure_id = $1 AND node_id = $2")
                .bind(structure_id)
                .bind(node_id)
                .execute(&self.pool)
                .await
                .map_err(map_error)?
                .rows_affected();
        if affected == 0 {
            return Err(DomainError::NodeNotFound(node_id.to_string()));
        }
        Ok(())
    }

    async fn find_by_id(
        &self,
        structure_id: i64,
        node_id: &str,
    ) -> Result<Option<Node>, DomainError> {
        let row = sqlx::query_as::<_, NodeRow>(
            "SELECT node_id, description, path, directory, parent_id, hidden,
                    reference_path, migration_directory, migration_filter
             FROM structure_nodes WHERE structure_id = $1 AND node_id = $2",
        )
        .bind(structure_id)
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(Node::try_from).transpose()
    }

    async fn list(&self, structure_id: i64) -> Result<Vec<Node>, DomainError> {
        let rows = sqlx::query_as::<_, NodeRow>(
            "SELECT node_id, description, path, directory, parent_id, hidden,
                    reference_path, migration_directory, migration_filter
             FROM structure_nodes WHERE structure_id = $1 ORDER BY path ASC",
        )
        .bind(structure_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(Node::try_from).collect()
    }

    async fn find_by_path(
        &self,
        structure_id: i64,
        path: &str,
    ) -> Result<Option<Node>, DomainError> {
        let row = sqlx::query_as::<_, NodeRow>(
            "SELECT node_id, description, path, directory, parent_id, hidden,
                    reference_path, migration_directory, migration_filter
             FROM structure_nodes WHERE structure_id = $1 AND path = $2",
        )
        .bind(structure_id)
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(Node::try_from).transpose()
    }
}

/// The only place in the crate that knows about `sqlx::Error`.
///
/// The driver does not surface the natural key of a violated
/// constraint, so `DuplicatePath` carries an empty value; the
/// usecase's own check produces the useful message, and the constraint
/// is the race backstop.
fn map_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db) => match db.code().as_deref() {
            Some("23505") => DomainError::DuplicatePath(String::new()),
            _ => DomainError::Repository(db.to_string()),
        },
        other => DomainError::Repository(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_structure_nodes_table_keeps_its_constraints() {
        let sql = include_str!("../../../../migrations/0002_create_structure_nodes.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS structure_nodes"));
        assert!(
            sql.contains("CONSTRAINT structure_nodes_unique_node UNIQUE (structure_id, node_id)")
        );
        assert!(sql.contains("CONSTRAINT structure_nodes_unique_path UNIQUE (structure_id, path)"));
        assert!(sql.contains("CONSTRAINT structure_nodes_migration_filter_array CHECK (jsonb_typeof(migration_filter) = 'array')"));
        assert!(sql.contains("CONSTRAINT structure_nodes_reference_only_on_file CHECK (reference_path IS NULL OR directory = FALSE)"));
        assert!(sql.contains("CONSTRAINT structure_nodes_migration_only_on_directory CHECK (migration_directory IS NULL OR directory = TRUE)"));
        assert!(sql.contains("REFERENCES structures(id) ON DELETE CASCADE"));
        assert!(sql.contains("BEFORE UPDATE ON structure_nodes"));
    }
}
