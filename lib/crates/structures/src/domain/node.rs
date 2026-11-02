use super::error::DomainError;
use super::migration::Migration;

/// The mutable attributes of a node, as supplied by a caller.
///
/// Grouped into a struct so the constructors stay readable — a node has
/// eight fields, and `Node::new(id, attrs)` reads better than eight
/// positional arguments.
#[derive(Debug, Clone, Default)]
pub struct NodeAttributes {
    /// Free-text label. May be empty.
    pub description: String,
    /// The node's path relative to the structure root.
    pub path: String,
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
    /// error: path shape first, then the two attribute pairings.
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_path(&self.path)?;
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
    /// The node's path relative to the structure root.
    pub path: String,
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
    /// The validating constructor. Rejects a malformed path and the two
    /// attribute pairings a node may not have.
    pub fn new(node_id: String, attrs: NodeAttributes) -> Result<Self, DomainError> {
        attrs.validate()?;
        Ok(Self {
            node_id,
            description: attrs.description,
            path: attrs.path,
            hidden: None,
            directory: attrs.directory,
            parent: attrs.parent,
            reference: attrs.reference,
            migration: attrs.migration,
        })
    }

    /// Skip validation. Reserved for the adapter layer — the data has
    /// already been validated on the way in.
    // The adapter layer lands in a later task; until then nothing in
    // the crate calls this, and the dead-code lint would fire.
    #[allow(dead_code)]
    pub(crate) fn for_repository(node_id: String, attrs: NodeAttributes) -> Self {
        Self {
            node_id,
            description: attrs.description,
            path: attrs.path,
            hidden: None,
            directory: attrs.directory,
            parent: attrs.parent,
            reference: attrs.reference,
            migration: attrs.migration,
        }
    }
}

/// A relative path is well formed when it is non-blank, uses `/` as its
/// only separator, and has no empty, `.` or `..` segment.
fn validate_path(path: &str) -> Result<(), DomainError> {
    let invalid = |path: &str| Err(DomainError::InvalidPath(path.to_string()));

    if path.trim().is_empty() {
        return invalid(path);
    }
    if path.starts_with('/') || path.ends_with('/') {
        return invalid(path);
    }
    if path.contains('\\') {
        return invalid(path);
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return invalid(path);
        }
    }
    Ok(())
}
