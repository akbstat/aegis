use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A named working branch of a project.
///
/// A project may own any number of branches; they are plain named
/// rows, and the name is unique within the owning project.
#[derive(Clone, PartialEq, Eq)]
pub struct ProjectBranch {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectBranch {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    ///
    /// Rejects a name that is empty or whitespace-only. The name is
    /// stored as given; trimming happens in the usecase layer, which is
    /// the only place that knows the value comes from a caller.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        project_id: i32,
        branch: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if branch.trim().is_empty() {
            return Err(DomainError::EmptyBranchName);
        }
        Ok(Self {
            id,
            project_id,
            branch,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i32,
        project_id: i32,
        branch: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            project_id,
            branch,
            created_at,
            updated_at,
        }
    }
}

impl std::fmt::Debug for ProjectBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectBranch")
            .field("id", &self.id)
            .field("project_id", &self.project_id)
            .field("branch", &self.branch)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Input DTO for `ProjectBranchRepository::create`.
#[derive(Debug, Clone)]
pub struct ProjectBranchNew {
    pub project_id: i32,
    pub branch: String,
}

/// Input DTO for `ProjectBranchRepository::update`. Every field is
/// optional so the usecase can pass only what changed.
///
/// `project_id` scopes the write: the repository only touches a branch
/// that belongs to that project, so a branch of another project is
/// reported as not found rather than updated.
#[derive(Debug, Clone, Default)]
pub struct ProjectBranchUpdate {
    pub id: i32,
    pub project_id: i32,
    pub branch: Option<String>,
}

/// Outbound port for persistence of project branches.
#[async_trait]
pub trait ProjectBranchRepository: Send + Sync {
    /// Creates a branch.
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError>;

    /// Fetches one branch by its id. Returns
    /// [`DomainError::BranchNotFound`] when no such branch exists.
    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError>;

    /// Lists every branch of one project, ordered by id.
    async fn list_by_project(&self, project_id: i32) -> Result<Vec<ProjectBranch>, DomainError>;

    /// Updates a branch.
    ///
    /// - `branch: Some(name)` renames it. A name that collides with
    ///   another branch of the same project surfaces as
    ///   [`DomainError::DuplicateBranch`].
    /// - `branch: None` leaves the name alone.
    ///
    /// The update is scoped to `input.project_id`: a branch belonging to
    /// another project is reported as
    /// [`DomainError::BranchNotFound`].
    async fn update(&self, input: ProjectBranchUpdate) -> Result<ProjectBranch, DomainError>;
}
