use crate::domain::{ProjectConfiguration, ProjectMember};

#[derive(Debug, Clone)]
pub struct CreateProject {
    pub code: String,
    pub description: String,
    /// Optional. `None` and `Some(empty)` are equivalent on create.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// Optional. `None` defaults to an empty configuration (no
    /// language, no tags); `Some(config)` writes the whole
    /// configuration on create.
    pub configuration: Option<ProjectConfiguration>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProject {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// `None` = leave configuration unchanged; `Some(config)` =
    /// whole-configuration replace.
    pub configuration: Option<ProjectConfiguration>,
}

/// Command to create a branch of a project.
#[derive(Debug, Clone)]
pub struct CreateProjectBranch {
    pub project_id: i32,
    pub branch: String,
}

/// Command to rename a branch. `branch: None` leaves the name alone.
#[derive(Debug, Clone)]
pub struct UpdateProjectBranch {
    pub id: i32,
    /// Scopes the write to one project. A branch of another project is
    /// reported as not found.
    pub project_id: i32,
    pub branch: Option<String>,
}
