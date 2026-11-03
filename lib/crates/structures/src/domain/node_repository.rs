use async_trait::async_trait;

use super::error::DomainError;
use super::node::Node;

/// Outbound port for persistence of `Node` rows.
///
/// A node row is always addressed by the `structure_id` that owns it
/// plus the node's `node_id`. `parent_id` is a **soft** reference: it
/// may name a node that lives in the *template* structure, so it cannot
/// be a database foreign key.
#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn insert(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError>;
    async fn update(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError>;
    async fn delete(&self, structure_id: i64, node_id: &str) -> Result<(), DomainError>;
    /// `Ok(None)` when the structure has no row with that `node_id`.
    async fn find_by_id(
        &self,
        structure_id: i64,
        node_id: &str,
    ) -> Result<Option<Node>, DomainError>;
    /// Every row of the structure, ordered by `entry_name ASC`.
    async fn list(&self, structure_id: i64) -> Result<Vec<Node>, DomainError>;
    /// `Ok(None)` when the structure has no row with that name under
    /// that parent.
    async fn find_by_name(
        &self,
        structure_id: i64,
        parent_id: Option<&str>,
        name: &str,
    ) -> Result<Option<Node>, DomainError>;
}
