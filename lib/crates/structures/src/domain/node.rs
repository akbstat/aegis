use super::error::DomainError;
use super::migration::Migration;

/// The mutable attributes of a node, as supplied by a caller.
///
/// Grouped into a struct so the constructors stay readable — a node has
/// seven fields, and `Node::new(id, attrs)` reads better than seven
/// positional arguments.
#[derive(Debug, Clone, Default)]
pub struct NodeAttributes {
    /// Free-text label. May be empty.
    pub description: String,
    /// The node's own name — the folder or file name it carries on
    /// disk. It is **not** a path: the nesting comes from `parent`, and
    /// two nodes under the same parent may not share a name.
    pub entry_name: String,
    /// Whether this node is a directory.
    pub directory: bool,
    /// The `node_id` of the containing node, if any.
    pub parent: Option<String>,
    /// Absolute file path. Only meaningful when `directory` is false.
    pub reference: Option<String>,
    /// What to copy into this directory. Only meaningful when `directory`
    /// is true.
    pub migration: Option<Migration>,
}

impl NodeAttributes {
    /// Enforce the node's invariants.
    ///
    /// The order is fixed so the same bad input always produces the same
    /// error: the name's shape first, then the two attribute pairings.
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_name(&self.entry_name)?;
        if self.reference.is_some() && self.directory {
            return Err(DomainError::ReferenceOnDirectory);
        }
        if self.migration.is_some() && !self.directory {
            return Err(DomainError::MigrationOnFile);
        }
        Ok(())
    }
}

/// One entry in a structure — a folder or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Unique inside one structure. **Shared identity space**: a project
    /// structure's extension record for an inherited node carries the
    /// template node's `node_id`.
    pub node_id: String,
    /// Free-text label. May be empty.
    pub description: String,
    /// The node's own name — the folder or file name. Unique among the
    /// node's siblings, not across the whole structure.
    pub entry_name: String,
    /// When true the node is filtered out of the effective structure.
    /// Only meaningful at project level.
    pub hidden: Option<bool>,
    /// Whether this node is a directory.
    pub directory: bool,
    /// The `node_id` of the containing node, if any.
    pub parent: Option<String>,
    /// Absolute file path. Only meaningful when `directory` is false.
    pub reference: Option<String>,
    /// What to copy into this directory. Only meaningful when `directory`
    /// is true.
    pub migration: Option<Migration>,
}

impl Node {
    /// The validating constructor. Rejects a malformed name and the two
    /// attribute pairings a node may not have.
    pub fn new(node_id: String, attrs: NodeAttributes) -> Result<Self, DomainError> {
        attrs.validate()?;
        Ok(Self {
            node_id,
            description: attrs.description,
            entry_name: attrs.entry_name,
            hidden: None,
            directory: attrs.directory,
            parent: attrs.parent,
            reference: attrs.reference,
            migration: attrs.migration,
        })
    }

    /// Skip validation. Reserved for the adapter layer — the data has
    /// already been validated on the way in.
    pub(crate) fn for_repository(node_id: String, attrs: NodeAttributes) -> Self {
        Self {
            node_id,
            description: attrs.description,
            entry_name: attrs.entry_name,
            hidden: None,
            directory: attrs.directory,
            parent: attrs.parent,
            reference: attrs.reference,
            migration: attrs.migration,
        }
    }
}

/// A node's name is well formed when it is non-blank, is a single path
/// segment — it names one folder or one file — and is neither `.` nor
/// `..`.
fn validate_name(name: &str) -> Result<(), DomainError> {
    let invalid = |name: &str| Err(DomainError::InvalidName(name.to_string()));

    if name.trim().is_empty() {
        return invalid(name);
    }
    if name.contains('/') || name.contains('\\') {
        return invalid(name);
    }
    if name == "." || name == ".." {
        return invalid(name);
    }
    Ok(())
}
