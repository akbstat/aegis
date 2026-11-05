use std::sync::Arc;

use async_trait::async_trait;

use apis::project::{
    ProjectApiError, ProjectBranchView, ProjectConfigurationView, ProjectMemberView,
    ProjectService, ProjectView, UserSummaryView,
};

use crate::domain::{DomainError, ProjectLookup};

use super::ProjectLookupImpl;

#[derive(Clone)]
struct FakeProject {
    leader_codes: Vec<String>,
}

#[async_trait]
impl ProjectService for FakeProject {
    async fn create_project(
        &self,
        _req: apis::project::CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_by_id(&self, _id: i32) -> Result<ProjectView, ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError> {
        Ok(view(code, &self.leader_codes))
    }
    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError> {
        unimplemented!()
    }
    async fn update_project(
        &self,
        _req: apis::project::UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        unimplemented!()
    }
    async fn create_project_branch(
        &self,
        _: apis::project::CreateProjectBranchRequest,
    ) -> Result<ProjectBranchView, ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_branch_by_id(&self, _: i32) -> Result<ProjectBranchView, ProjectApiError> {
        unimplemented!()
    }
    async fn list_project_branches(
        &self,
        _: i32,
    ) -> Result<Vec<ProjectBranchView>, ProjectApiError> {
        unimplemented!()
    }
    async fn update_project_branch(
        &self,
        _: apis::project::UpdateProjectBranchRequest,
    ) -> Result<ProjectBranchView, ProjectApiError> {
        unimplemented!()
    }
}

fn view(code: &str, leaders: &[String]) -> ProjectView {
    ProjectView {
        id: 1,
        code: code.to_string(),
        description: String::new(),
        members: ProjectMemberView {
            leaders: leaders
                .iter()
                .map(|c| UserSummaryView {
                    code: c.to_string(),
                    name: c.to_string(),
                })
                .collect(),
            workers: vec![],
        },
        unblind_members: ProjectMemberView::default(),
        configurations: ProjectConfigurationView::default(),
        active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn is_leader_true_for_listed_leader() {
    let svc = Arc::new(FakeProject {
        leader_codes: vec!["alice".into(), "bob".into()],
    });
    let lookup = ProjectLookupImpl::new(svc);
    assert!(lookup.is_leader("p1", "alice").await.unwrap());
}

#[tokio::test]
async fn is_leader_false_for_non_leader() {
    let svc = Arc::new(FakeProject {
        leader_codes: vec!["alice".into()],
    });
    let lookup = ProjectLookupImpl::new(svc);
    assert!(!lookup.is_leader("p1", "carol").await.unwrap());
}

#[tokio::test]
async fn get_by_code_maps_not_found() {
    struct NotFound;
    #[async_trait]
    impl ProjectService for NotFound {
        async fn create_project(
            &self,
            _: apis::project::CreateProjectRequest,
        ) -> Result<ProjectView, ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_id(&self, _: i32) -> Result<ProjectView, ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_code(&self, _: &str) -> Result<ProjectView, ProjectApiError> {
            Err(ProjectApiError::NotFound)
        }
        async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError> {
            unimplemented!()
        }
        async fn update_project(
            &self,
            _: apis::project::UpdateProjectRequest,
        ) -> Result<ProjectView, ProjectApiError> {
            unimplemented!()
        }
        async fn create_project_branch(
            &self,
            _: apis::project::CreateProjectBranchRequest,
        ) -> Result<ProjectBranchView, ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_branch_by_id(
            &self,
            _: i32,
        ) -> Result<ProjectBranchView, ProjectApiError> {
            unimplemented!()
        }
        async fn list_project_branches(
            &self,
            _: i32,
        ) -> Result<Vec<ProjectBranchView>, ProjectApiError> {
            unimplemented!()
        }
        async fn update_project_branch(
            &self,
            _: apis::project::UpdateProjectBranchRequest,
        ) -> Result<ProjectBranchView, ProjectApiError> {
            unimplemented!()
        }
    }
    let lookup = ProjectLookupImpl::new(Arc::new(NotFound));
    let err = lookup.get_by_code("p1").await.unwrap_err();
    assert!(matches!(err, DomainError::ProjectNotFound(ref c) if c == "p1"));
}
