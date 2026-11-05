use crate::domain::{DomainError, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate};

use super::commands::{CreateProjectBranch, UpdateProjectBranch};
use super::error::UsecaseError;
use super::views::ProjectBranchView;

/// Orchestrates the project-branch port.
///
/// The branch aggregate has its own lifecycle, so it gets its own
/// usecase rather than growing `ProjectUsecase`.
pub struct ProjectBranchUsecase<B: ProjectBranchRepository> {
    branches: B,
}

impl<B: ProjectBranchRepository> ProjectBranchUsecase<B> {
    pub fn new(branches: B) -> Self {
        Self { branches }
    }

    /// Creates a branch of a project.
    pub async fn create_branch(
        &self,
        cmd: CreateProjectBranch,
    ) -> Result<ProjectBranchView, UsecaseError> {
        let branch = cmd.branch.trim();
        if branch.is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyBranchName));
        }
        let created = self
            .branches
            .create(ProjectBranchNew {
                project_id: cmd.project_id,
                branch: branch.to_string(),
                active: cmd.active,
            })
            .await?;
        Ok(created.into())
    }

    /// Fetches one branch by its id.
    pub async fn get_branch_by_id(&self, id: i32) -> Result<ProjectBranchView, UsecaseError> {
        let branch = self.branches.find_by_id(id).await?;
        Ok(branch.into())
    }

    /// Lists every branch of one project.
    pub async fn list_branches(
        &self,
        project_id: i32,
    ) -> Result<Vec<ProjectBranchView>, UsecaseError> {
        let branches = self.branches.list_by_project(project_id).await?;
        Ok(branches.into_iter().map(Into::into).collect())
    }

    /// Renames a branch and / or changes whether it is the project's
    /// active branch.
    pub async fn update_branch(
        &self,
        cmd: UpdateProjectBranch,
    ) -> Result<ProjectBranchView, UsecaseError> {
        let branch = match cmd.branch {
            Some(ref name) => {
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    return Err(UsecaseError::Validation(DomainError::EmptyBranchName));
                }
                Some(trimmed.to_string())
            }
            None => None,
        };
        let updated = self
            .branches
            .update(ProjectBranchUpdate {
                id: cmd.id,
                project_id: cmd.project_id,
                branch,
                active: cmd.active,
            })
            .await?;
        Ok(updated.into())
    }
}
