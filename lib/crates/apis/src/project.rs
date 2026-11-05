//! Outbound port for project lifecycle operations.
//!
//! See [`ProjectService`] for the trait surface. All supporting types
//! (`ProjectApiError`, `ProjectView`, `ProjectConfigurationView`,
//! `ProjectMemberView`, `UserSummaryView`, `TagData`, `TagView`,
//! `*Request`) are defined alongside the trait so a single
//! `use apis::project::*;` brings the whole contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error surface returned by every [`ProjectService`] method.
///
/// Adapters map backend-specific errors (e.g. `project::UsecaseError`)
/// into this type at the implementation boundary.
#[derive(Debug, Clone, Error)]
pub enum ProjectApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("branch not found: {0}")]
    BranchNotFound(i32),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("branch already exists: {0}")]
    DuplicateBranch(String),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Locale hint. Carried on the project configuration so a project
/// can pin its UI to English or Simplified Chinese. The wire code
/// is the lowercase IETF tag (`"en"` or `"zh-CN"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectLanguage {
    English,
    SimplifiedChinese,
}

impl ProjectLanguage {
    /// Stable wire code. New locales land here as new variants.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectLanguage::English => "en",
            ProjectLanguage::SimplifiedChinese => "zh-CN",
        }
    }
}

/// Wire-shaped tag data. `key` and `value` are both required and
/// non-empty; the backend enforces that contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagData {
    pub key: String,
    pub value: String,
}

/// Server-side projection of a tag. Same shape as [`TagData`]; kept
/// as a distinct type so the wire DTO can diverge later without
/// breaking the projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagView {
    pub key: String,
    pub value: String,
}

/// Wire-shaped pointer to a `SdtmVersion` (SDTM Implementation
/// Guide version). The pair `(version_id, version_name)` is what
/// the client picked from `DomainModelService::list_versions`; the
/// project crate does not verify the row still exists at write
/// time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelVersionData {
    pub version_id: i64,
    pub version_name: String,
}

/// Server-side projection of [`ModelVersionData`]. Kept as a
/// distinct type so the request DTO can diverge later without
/// breaking the projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelVersionView {
    pub version_id: i64,
    pub version_name: String,
}

/// Wire-shaped pointer to a `TerminologyVersion` (controlled
/// terminology release) a project pins. Mirrors the shape of
/// [`ModelVersionData`] but lives at a distinct type so the wire
/// field name (`sdtm_terminology`) is free to evolve independently.
/// `version_id` is the surrogate key from
/// `terminology::TerminologyVersion::id`; `version_name` is the
/// workbook suffix (e.g. `"2024-03-29"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyVersionData {
    pub version_id: i64,
    pub version_name: String,
}

/// Server-side projection of [`TerminologyVersionData`]. Kept as a
/// distinct type so the request DTO can diverge later without
/// breaking the projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyVersionView {
    pub version_id: i64,
    pub version_name: String,
}

/// Request-side configuration: optional locale plus the tag list
/// plus the optional SDTM-IG version pointer plus the optional
/// SDTM terminology pointer. Carries `TagData` because it travels
/// into the backend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationData {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagData>,
    pub sdtmig: Option<ModelVersionData>,
    pub sdtm_terminology: Option<TerminologyVersionData>,
}

/// Server-side projection of the configuration. Carries `TagView`
/// because it travels back to clients.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationView {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagView>,
    pub sdtmig: Option<ModelVersionView>,
    pub sdtm_terminology: Option<TerminologyVersionView>,
}

/// Safe projection of a project: membership lists are hydrated to
/// `Vec<UserSummaryView>`; the configuration (locale + tags) is
/// projected to `ProjectConfigurationView`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub configurations: ProjectConfigurationView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummaryView {
    pub code: String,
    pub name: String,
}

/// Wire-shaped membership data. `leaders` and `workers` are user codes
/// (not full user records); the backend hydrates them to
/// `UserSummaryView` on read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectMemberData {
    pub leaders: Vec<String>,
    pub workers: Vec<String>,
}

/// Server-side projection of a single project branch.
///
/// Branches are a standalone resource: `ProjectView` does not carry
/// them, and they are read through the branch endpoints only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBranchView {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request-side payload for creating a branch of a project.
///
/// `project_id` is resolved from the project's code by the transport
/// layer before the request reaches the backend.
#[derive(Debug, Clone)]
pub struct CreateProjectBranchRequest {
    pub project_id: i32,
    pub branch: String,
    /// Whether the new branch becomes the project's active one. When
    /// true, the project's other active branches are deactivated.
    pub active: bool,
}

/// Request-side payload for renaming a branch and / or changing
/// whether it is the project's active branch.
///
/// `None` for `branch` leaves the name alone; `None` for `active`
/// leaves the flag alone. `project_id` scopes the write, so a branch
/// of another project is reported as not found.
#[derive(Debug, Clone, Default)]
pub struct UpdateProjectBranchRequest {
    pub id: i32,
    pub project_id: i32,
    pub branch: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    /// Optional. Omit (or pass an empty `ProjectMemberData`) to create
    /// the project with no membership rows; the shell can be filled in
    /// via a later `update_project` call.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    /// Optional. `None` and `Some(empty)` both mean "default empty
    /// configuration (no language, no tags)".
    pub configurations: Option<ProjectConfigurationData>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProjectRequest {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    /// `None` = leave configuration unchanged; `Some(config)` =
    /// whole-configuration replace.
    pub configurations: Option<ProjectConfigurationData>,
}

/// Outbound port for project lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn ProjectService>` can be shared state in
/// an async server (axum, tarpc, etc.).
#[async_trait]
pub trait ProjectService: Send + Sync {
    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError>;
    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError>;
    async fn update_project(
        &self,
        req: UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;

    /// Creates a branch of a project. When `req.active` is true the new
    /// branch becomes the project's active one and the project's other
    /// active branches are deactivated.
    async fn create_project_branch(
        &self,
        req: CreateProjectBranchRequest,
    ) -> Result<ProjectBranchView, ProjectApiError>;

    /// Fetches one branch by its id.
    async fn get_project_branch_by_id(&self, id: i32)
    -> Result<ProjectBranchView, ProjectApiError>;

    /// Lists every branch of one project.
    async fn list_project_branches(
        &self,
        project_id: i32,
    ) -> Result<Vec<ProjectBranchView>, ProjectApiError>;

    /// Renames a branch and / or changes whether it is the project's
    /// active branch.
    async fn update_project_branch(
        &self,
        req: UpdateProjectBranchRequest,
    ) -> Result<ProjectBranchView, ProjectApiError>;
}
