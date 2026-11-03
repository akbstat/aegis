use crate::domain::{Migration, StructureLevel};

/// Create a structure.
///
/// A **template** structure carries no `project_code` and no
/// `template_id`; a **project** structure must carry both.
#[derive(Debug, Clone)]
pub struct CreateStructure {
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// Rename a structure.
///
/// Only the name is mutable. The level, the owning project and the
/// template pointer are immutable after creation.
#[derive(Debug, Clone)]
pub struct RenameStructure {
    pub id: i64,
    pub name: String,
}

/// Place a node at a path.
///
/// The usecase resolves the path against the effective structure and
/// decides whether this creates a new node or overrides an inherited
/// one. `parent` is always explicit: `None` means "at the root".
#[derive(Debug, Clone)]
pub struct AddNode {
    pub structure_id: i64,
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<Migration>,
    pub hidden: Option<bool>,
}

/// Replace a node's mutable attributes, addressed by `node_id`.
///
/// A full replacement: `parent: None` moves the node to the root and
/// `hidden: Some(false)` un-hides it.
#[derive(Debug, Clone)]
pub struct UpdateNode {
    pub structure_id: i64,
    pub node_id: String,
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<Migration>,
    pub hidden: Option<bool>,
}

/// Remove a node.
///
/// The node and everything under it leave the structure: an override is
/// hidden, a record with no template counterpart is deleted.
#[derive(Debug, Clone)]
pub struct DeleteNode {
    pub structure_id: i64,
    pub node_id: String,
}
