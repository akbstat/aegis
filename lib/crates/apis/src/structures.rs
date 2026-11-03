//! Outbound port for the directory-structure service.
//!
//! Mirrors the surface of `structures::usecase::StructureUsecase` so
//! adapters in any backend (in-memory, PostgreSQL, …) can adapt their
//! own types to the shared contract defined here. All supporting DTOs
//! (request shapes, view projections, enum re-declarations, and
//! [`StructureApiError`]) live alongside the trait so a single
//! `use apis::structures::*;` brings the whole contract into scope.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error surface returned by every [`StructureService`] method.
///
/// Adapters map backend-specific errors (e.g. `structures::UsecaseError`)
/// into this type at the implementation boundary.
#[derive(Debug, Clone, Error)]
pub enum StructureApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("structure not found: {0}")]
    StructureNotFound(i64),

    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("template structure not found: {0}")]
    TemplateNotFound(i64),

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("name already used under the same parent: {0}")]
    DuplicateName(String),

    #[error("project already has a structure: {0}")]
    DuplicateProjectStructure(String),

    #[error("template structure is still in use: {0}")]
    TemplateInUse(i64),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Which kind of structure this is.
///
/// Mirrors `structures::domain::StructureLevel`. The two enums are kept
/// in sync layer by layer — adapter implementations convert losslessly
/// via the matching variant constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureLevel {
    Template,
    Project,
}

/// What to copy into a directory once it has been created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationView {
    pub directory: String,
    pub filter: Vec<String>,
}

/// One node of the effective tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeView {
    pub node_id: String,
    pub description: String,
    pub entry_name: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    /// The node's effective children.
    pub children: Vec<NodeView>,
}

/// A structure and its effective tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

/// A structure's identity, without its tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureSummaryView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// A structure's identity and level, without the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureMetaView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// The effective tree of a structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureTreeView {
    pub structure_id: i64,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

/// One of a structure's own records — the extension records, with the
/// two facts a client cannot get from the merged tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnNodeView {
    pub node_id: String,
    pub description: String,
    pub entry_name: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    pub hidden: bool,
    pub overrides_template: bool,
}

/// A page-less list of structures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureList {
    pub structures: Vec<StructureSummaryView>,
}

/// A page-less list of a structure's own records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnNodeList {
    pub nodes: Vec<OwnNodeView>,
}

// ---- requests ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStructureRequest {
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameStructureRequest {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListStructuresRequest {
    pub level: Option<StructureLevel>,
    pub project_code: Option<String>,
}

/// Place a node under a parent.
///
/// `parent` is always explicit: `None` means "at the root".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddNodeRequest {
    pub structure_id: i64,
    pub entry_name: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    pub hidden: Option<bool>,
}

/// Replace a node's mutable attributes.
///
/// A full replacement: `parent: None` moves the node to the root and
/// `hidden: Some(false)` un-hides it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeRequest {
    pub structure_id: i64,
    pub node_id: String,
    pub entry_name: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteNodeRequest {
    pub structure_id: i64,
    pub node_id: String,
}

/// The public-facing contract for directory structures.
#[async_trait]
pub trait StructureService: Send + Sync {
    async fn create_structure(
        &self,
        req: CreateStructureRequest,
    ) -> Result<StructureView, StructureApiError>;

    async fn get_structure(&self, id: i64) -> Result<StructureView, StructureApiError>;

    /// The structure's identity and level, without building the effective
    /// tree. The auth gate needs the level on every write, and building
    /// the whole tree to read one enum would be wasteful.
    async fn get_structure_meta(&self, id: i64) -> Result<StructureMetaView, StructureApiError>;

    async fn list_structures(
        &self,
        req: ListStructuresRequest,
    ) -> Result<StructureList, StructureApiError>;

    async fn rename_structure(
        &self,
        req: RenameStructureRequest,
    ) -> Result<StructureView, StructureApiError>;

    async fn delete_structure(&self, id: i64) -> Result<(), StructureApiError>;

    async fn add_node(&self, req: AddNodeRequest) -> Result<NodeView, StructureApiError>;

    async fn update_node(&self, req: UpdateNodeRequest) -> Result<NodeView, StructureApiError>;

    async fn delete_node(&self, req: DeleteNodeRequest) -> Result<(), StructureApiError>;

    /// This structure's own records — the extension records, with their
    /// `hidden` flag and whether each overrides a template node. This is
    /// where a client learns the `node_id` of a hidden node, so it can
    /// undo a hide.
    async fn list_own_nodes(&self, structure_id: i64) -> Result<OwnNodeList, StructureApiError>;

    async fn build_structure(&self, id: i64) -> Result<StructureTreeView, StructureApiError>;
}
