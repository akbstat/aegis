use super::error::DomainError;
use super::node::Node;
use super::structure_level::StructureLevel;

/// A directory layout.
///
/// A **template** structure is global and shared. A **project** structure
/// belongs to exactly one project and extends exactly one template; its
/// `nodes` are the *extension records* — the difference from the
/// template, not the whole tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    /// The owning project. Set at project level, absent at template level.
    pub project_code: Option<String>,
    /// The template this structure extends. Set at project level, absent
    /// at template level. Immutable once the structure is created.
    pub template_id: Option<i64>,
    /// The structure's own rows. At project level these are the
    /// extension records.
    pub nodes: Vec<Node>,
}

impl Structure {
    /// The validating constructor for a structure that has not been
    /// persisted yet.
    ///
    /// A template structure carries no owner and no template; a project
    /// structure must carry both. The `id` is `0` until the repository
    /// assigns one.
    pub fn new(
        name: String,
        level: StructureLevel,
        project_code: Option<String>,
        template_id: Option<i64>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        match level {
            StructureLevel::Template => {
                if project_code.is_some() || template_id.is_some() {
                    return Err(DomainError::UnexpectedOwner);
                }
            }
            StructureLevel::Project => {
                if project_code.as_deref().map(str::trim).unwrap_or("").is_empty() {
                    return Err(DomainError::MissingProjectCode);
                }
                if template_id.is_none() {
                    return Err(DomainError::MissingTemplate);
                }
            }
        }
        Ok(Self {
            id: 0,
            name,
            level,
            project_code,
            template_id,
            nodes: Vec::new(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        name: String,
        level: StructureLevel,
        project_code: Option<String>,
        template_id: Option<i64>,
        nodes: Vec<Node>,
    ) -> Self {
        Self {
            id,
            name,
            level,
            project_code,
            template_id,
            nodes,
        }
    }
}

/// Input DTO for `StructureRepository::create`.
#[derive(Debug, Clone)]
pub struct StructureNew {
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// The filter for `StructureRepository::list`.
///
/// Both fields are optional: a filter with neither lists every
/// structure.
#[derive(Debug, Clone, Default)]
pub struct StructureFilter {
    pub level: Option<StructureLevel>,
    pub project_code: Option<String>,
}
