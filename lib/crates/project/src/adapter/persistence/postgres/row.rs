//! Row -> domain conversion for the SQLx repository.
//!
//! `ProjectRow` is the shape returned by `sqlx::query_as`. It is NOT
//! re-exported at the crate root; only the repository uses it.

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{DomainError, Project, ProjectBranch, ProjectConfiguration, ProjectMember};

#[derive(Clone, FromRow)]
pub struct ProjectRow {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub active: bool,
    pub configuration: sqlx::types::Json<ProjectConfiguration>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRow> for Project {
    type Error = DomainError;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project::for_repository(
            row.id,
            row.code,
            row.description,
            ProjectMember::default(),
            ProjectMember::default(),
            row.configuration.0,
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}

/// One row from `project_members`.
#[derive(Clone, FromRow)]
#[allow(dead_code)]
pub struct ProjectMemberRow {
    pub project_id: i32,
    pub team_type: String,
    pub role_type: String,
    pub user_code: String,
}

/// One row from `project_branches`.
#[derive(Clone, FromRow)]
pub struct ProjectBranchRow {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectBranchRow> for ProjectBranch {
    type Error = DomainError;

    fn try_from(row: ProjectBranchRow) -> Result<Self, Self::Error> {
        Ok(ProjectBranch::for_repository(
            row.id,
            row.project_id,
            row.branch,
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}
