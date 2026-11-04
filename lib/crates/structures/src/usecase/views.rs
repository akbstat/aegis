use crate::domain::{MergedNode, Migration, StructureLevel};

/// A structure and its effective tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureView {
    pub id: i64,
    pub name: String,
    /// The absolute base directory this structure's tree is rooted at.
    pub rootpath: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

/// A structure's identity, without its tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureSummaryView {
    pub id: i64,
    pub name: String,
    /// The absolute base directory this structure's tree is rooted at.
    pub rootpath: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// A structure's identity and level, without the tree.
///
/// The auth gate needs the level on every write, and building the whole
/// effective tree to read one enum would be wasteful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureMetaView {
    pub id: i64,
    pub name: String,
    /// The absolute base directory this structure's tree is rooted at.
    pub rootpath: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// The effective tree of a structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTreeView {
    pub structure_id: i64,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

/// One node of the effective tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeView {
    pub node_id: String,
    pub description: String,
    /// The node's own name — a folder or a file name.
    pub entry_name: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    /// The node's effective children.
    pub children: Vec<NodeView>,
}

/// One of the structure's own records.
///
/// The two facts a client cannot get from the merged tree: whether the
/// record is hidden, and whether it overrides a template node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnNodeView {
    pub node_id: String,
    pub description: String,
    /// The node's own name — a folder or a file name.
    pub entry_name: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    pub hidden: bool,
    pub overrides_template: bool,
}

/// What to copy into a directory once it has been created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationView {
    pub directory: String,
    pub filter: Vec<String>,
}

impl From<Migration> for MigrationView {
    fn from(m: Migration) -> Self {
        Self {
            directory: m.directory,
            filter: m.filter,
        }
    }
}

impl From<MergedNode> for NodeView {
    fn from(m: MergedNode) -> Self {
        Self {
            node_id: m.node.node_id,
            description: m.node.description,
            entry_name: m.node.entry_name,
            directory: m.node.directory,
            parent: m.node.parent,
            reference: m.node.reference,
            migration: m.node.migration.map(Into::into),
            children: m.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl NodeView {
    /// A node rendered on its own, with no children.
    ///
    /// Used for a node the effective tree does not contain — a hidden
    /// node, or one whose parent is no longer reachable.
    pub fn from_node(node: &crate::domain::Node) -> Self {
        Self {
            node_id: node.node_id.clone(),
            description: node.description.clone(),
            entry_name: node.entry_name.clone(),
            directory: node.directory,
            parent: node.parent.clone(),
            reference: node.reference.clone(),
            migration: node.migration.clone().map(Into::into),
            children: Vec::new(),
        }
    }
}
