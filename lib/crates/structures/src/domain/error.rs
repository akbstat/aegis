use thiserror::Error;

/// Every error the domain layer can raise.
///
/// Validation variants come from the constructors; the rest come from
/// the adapters and the usecase at run time.
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    // ---- validation (constructor-time) ----
    #[error("empty name")]
    EmptyName,
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("invalid structure level: {0}")]
    InvalidStructureLevel(String),
    #[error("missing project code")]
    MissingProjectCode,
    #[error("missing template")]
    MissingTemplate,
    #[error("unexpected owner on a template structure")]
    UnexpectedOwner,
    #[error("hidden is not allowed on a template structure")]
    HiddenNotAllowed,
    #[error("nothing to hide: the node overrides no template node")]
    NothingToHide,
    #[error("a reference is only allowed on a file node")]
    ReferenceOnDirectory,
    #[error("a migration is only allowed on a directory node")]
    MigrationOnFile,
    #[error("parent node not found: {0}")]
    ParentNotFound(String),
    #[error("parent node is not a directory: {0}")]
    ParentNotADirectory(String),
    #[error("the node hierarchy contains a cycle")]
    Cycle,

    // ---- existence / duplicate (runtime) ----
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("structure not found: {0}")]
    StructureNotFound(i64),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("template structure not found: {0}")]
    TemplateNotFound(i64),
    #[error("structure {0} is not a template structure")]
    NotATemplate(i64),
    #[error("name already used under the same parent: {0}")]
    DuplicateName(String),
    #[error("project {0} already has a structure")]
    DuplicateProjectStructure(String),
    #[error("template structure {0} is still in use")]
    TemplateInUse(i64),

    #[error("not found")]
    NotFound,
    #[error("repository error: {0}")]
    Repository(String),
}
