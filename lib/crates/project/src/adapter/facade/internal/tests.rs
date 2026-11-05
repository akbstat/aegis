//! End-to-end tests for the apis `ProjectService` facade, exercised
//! against in-memory repository + user-service fakes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use apis::project::{
    CreateProjectRequest, ProjectApiError, ProjectConfigurationData, ProjectService, TagData,
    UpdateProjectRequest,
};

use crate::adapter::facade::internal::ProjectServiceImpl;
use crate::domain::{
    DomainError, Project, ProjectBranch, ProjectBranchNew, ProjectBranchRepository,
    ProjectBranchUpdate, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    RoleType, TeamType, UserService, UserSummary,
};
use crate::usecase::{ProjectBranchUsecase, ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- in-memory fakes ----------

#[derive(Default)]
struct InMemProjectState {
    projects: HashMap<i32, Project>,
    next_id: AtomicI32,
}

#[derive(Clone, Default)]
struct InMemProjectRepo {
    state: Arc<Mutex<InMemProjectState>>,
}

impl InMemProjectRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemProjectState {
                projects: HashMap::new(),
                next_id: AtomicI32::new(1),
            })),
        }
    }
}

#[async_trait]
impl ProjectRepository for InMemProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.projects.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint projects_code_unique)".into(),
            ));
        }
        let id = s.next_id.fetch_add(1, Ordering::SeqCst);
        let now = mock_now();
        let members = input.members.clone().unwrap_or_default();
        let unblind = input.unblind_members.clone().unwrap_or_default();
        let configuration = input.configuration.clone().unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            members,
            unblind,
            configuration,
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
        Ok(project)
    }
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        let s = self.state.lock().unwrap();
        let p = s.projects.get(&id).cloned().ok_or(DomainError::NotFound)?;
        Ok(p)
    }
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        let s = self.state.lock().unwrap();
        s.projects
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Project>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .projects
            .values()
            .cloned()
            .collect())
    }
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref c) = input.code {
            let dup = s
                .projects
                .values()
                .any(|o| o.code == *c && o.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
        }
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref c) = input.code {
            p.code = c.clone();
        }
        if let Some(ref d) = input.description {
            p.description = d.clone();
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
        }
        if let Some(ref t) = input.configuration {
            p.configurations = t.clone();
        }
        Ok(p.clone())
    }
}

#[derive(Default)]
struct InMemBranchState {
    branches: HashMap<i32, ProjectBranch>,
    next_id: AtomicI32,
}

#[derive(Clone, Default)]
struct InMemBranchRepo {
    state: Arc<Mutex<InMemBranchState>>,
}

impl InMemBranchRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemBranchState {
                branches: HashMap::new(),
                next_id: AtomicI32::new(1),
            })),
        }
    }
}

#[async_trait]
impl ProjectBranchRepository for InMemBranchRepo {
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.branches
            .values()
            .any(|b| b.project_id == input.project_id && b.branch == input.branch)
        {
            return Err(DomainError::DuplicateBranch(input.branch));
        }
        let id = s.next_id.fetch_add(1, Ordering::SeqCst);
        let now = mock_now();
        let branch = ProjectBranch::for_repository(id, input.project_id, input.branch, now, now);
        s.branches.insert(id, branch.clone());
        Ok(branch)
    }
    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError> {
        let s = self.state.lock().unwrap();
        let b = s
            .branches
            .get(&id)
            .cloned()
            .ok_or(DomainError::BranchNotFound(id))?;
        Ok(b)
    }
    async fn list_by_project(&self, project_id: i32) -> Result<Vec<ProjectBranch>, DomainError> {
        let mut out: Vec<ProjectBranch> = self
            .state
            .lock()
            .unwrap()
            .branches
            .values()
            .filter(|b| b.project_id == project_id)
            .cloned()
            .collect();
        out.sort_by_key(|b| b.id);
        Ok(out)
    }
    async fn update(&self, input: ProjectBranchUpdate) -> Result<ProjectBranch, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref name) = input.branch {
            let dup = s
                .branches
                .values()
                .any(|o| o.project_id == input.project_id && o.branch == *name && o.id != input.id);
            if dup {
                return Err(DomainError::DuplicateBranch(name.clone()));
            }
        }
        let b = s
            .branches
            .get_mut(&input.id)
            .filter(|b| b.project_id == input.project_id)
            .ok_or(DomainError::BranchNotFound(input.id))?;
        if let Some(ref name) = input.branch {
            b.branch = name.clone();
        }
        Ok(b.clone())
    }
}

#[derive(Clone, Default)]
struct InMemUserService {
    users: Arc<Mutex<Vec<UserSummary>>>,
}

impl InMemUserService {
    fn with_users(users: Vec<UserSummary>) -> Self {
        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[async_trait]
impl UserService for InMemUserService {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserSummary>, DomainError> {
        Ok(self.users.lock().unwrap().clone())
    }
}

fn make_service() -> ProjectServiceImpl<InMemProjectRepo, InMemUserService, InMemBranchRepo> {
    let projects = InMemProjectRepo::new();
    let users = InMemUserService::with_users(vec![
        UserSummary {
            code: "u1".into(),
            name: "Alice".into(),
        },
        UserSummary {
            code: "u2".into(),
            name: "Bob".into(),
        },
        UserSummary {
            code: "u3".into(),
            name: "Carol".into(),
        },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        project_repo: projects,
        users,
    });
    let branch_usecase = ProjectBranchUsecase::new(InMemBranchRepo::new());
    ProjectServiceImpl::new(usecase, branch_usecase)
}

fn tag(key: &str, value: &str) -> TagData {
    TagData {
        key: key.into(),
        value: value.into(),
    }
}

fn configuration_with_tags(tags: Vec<TagData>) -> ProjectConfigurationData {
    ProjectConfigurationData {
        language: None,
        tags,
        sdtmig: None,
        sdtm_terminology: None,
    }
}

#[tokio::test]
async fn create_project_with_none_membership_returns_empty_views() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configurations: None,
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
    assert!(view.configurations.tags.is_empty());
}

#[tokio::test]
async fn create_project_with_some_empty_membership_equivalent_to_none() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(Default::default()),
            unblind_members: Some(Default::default()),
            configurations: None,
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_full_membership() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u3".into()],
                workers: vec![],
            }),
            configurations: None,
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
    assert_eq!(view.unblind_members.leaders[0].code, "u3");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let service = make_service();
    let err = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
            configurations: None,
        })
        .await
        .expect_err("unknown member");
    assert!(matches!(err, ProjectApiError::UserNotFound(ref c) if c == "ghost"));
}

#[tokio::test]
async fn create_project_with_configuration_round_trips_through_ap_view() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configurations: Some(configuration_with_tags(vec![
                tag("Product", "DEMO-001"),
                tag("Region", "EU"),
            ])),
        })
        .await
        .expect("create");
    assert_eq!(view.configurations.tags.len(), 2);
    assert_eq!(view.configurations.tags[0].key, "Product");
    assert_eq!(view.configurations.tags[0].value, "DEMO-001");
    assert_eq!(view.configurations.tags[1].key, "Region");
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let service = make_service();
    let created = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
            configurations: None,
        })
        .await
        .expect("create");
    let updated = service
        .update_project(UpdateProjectRequest {
            id: created.id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec![],
                workers: vec!["u2".into(), "u3".into()],
            }),
            unblind_members: None,
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(updated.members.leaders.is_empty());
    assert_eq!(updated.members.workers.len(), 2);
}

#[tokio::test]
async fn update_project_replaces_configuration_whole_list() {
    let service = make_service();
    let created = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configurations: Some(configuration_with_tags(vec![tag("k1", "v1")])),
        })
        .await
        .expect("create");
    let updated = service
        .update_project(UpdateProjectRequest {
            id: created.id,
            configurations: Some(configuration_with_tags(vec![
                tag("k2", "v2"),
                tag("k3", "v3"),
            ])),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.configurations.tags.len(), 2);
    assert_eq!(updated.configurations.tags[0].key, "k2");
}

#[tokio::test]
async fn project_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectServiceImpl<InMemProjectRepo, InMemUserService, InMemBranchRepo>>();
}

#[tokio::test]
async fn project_service_box_dyn_compiles() {
    let service = make_service();
    let _boxed: Box<dyn ProjectService> = Box::new(service);
}

#[tokio::test]
async fn create_project_with_sdtmig_round_trips_through_ap_view() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configurations: Some(apis::project::ProjectConfigurationData {
                language: None,
                tags: vec![],
                sdtmig: Some(apis::project::ModelVersionData {
                    version_id: 7,
                    version_name: "2024-03-29".into(),
                }),
                sdtm_terminology: None,
            }),
        })
        .await
        .expect("create");
    let sdtmig = view.configurations.sdtmig.expect("sdtmig present");
    assert_eq!(sdtmig.version_id, 7);
    assert_eq!(sdtmig.version_name, "2024-03-29");
}

// silence unused import warnings for enums the tests exercise via
// the in-memory fakes
#[allow(dead_code)]
fn _force_use_team_role() -> (TeamType, RoleType) {
    (TeamType::Members, RoleType::Leader)
}

#[allow(dead_code)]
fn _force_use_project_member() -> ProjectMember {
    ProjectMember::default()
}

#[allow(dead_code)]
fn _force_use_project_tag() -> ProjectTag {
    ProjectTag::for_repository("k".into(), "v".into())
}

// ---------- branch methods through the port ----------

#[tokio::test]
async fn create_project_branch_through_the_port() {
    let service = make_service();
    let view = service
        .create_project_branch(apis::project::CreateProjectBranchRequest {
            project_id: 1,
            branch: "2026Q1".into(),
        })
        .await
        .expect("create");
    assert_eq!(view.branch, "2026Q1");
    assert_eq!(view.project_id, 1);
}

#[tokio::test]
async fn create_project_branch_with_a_blank_name_is_a_validation_error() {
    let service = make_service();
    let err = service
        .create_project_branch(apis::project::CreateProjectBranchRequest {
            project_id: 1,
            branch: "   ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ProjectApiError::Validation(_)));
}

#[tokio::test]
async fn create_project_branch_with_a_duplicate_name_is_a_conflict() {
    let service = make_service();
    service
        .create_project_branch(apis::project::CreateProjectBranchRequest {
            project_id: 1,
            branch: "2026Q1".into(),
        })
        .await
        .expect("create");
    let err = service
        .create_project_branch(apis::project::CreateProjectBranchRequest {
            project_id: 1,
            branch: "2026Q1".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ProjectApiError::DuplicateBranch(_)));
}

#[tokio::test]
async fn list_project_branches_through_the_port() {
    let service = make_service();
    service
        .create_project_branch(apis::project::CreateProjectBranchRequest {
            project_id: 1,
            branch: "2026Q1".into(),
        })
        .await
        .expect("create");
    let views = service.list_project_branches(1).await.expect("list");
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].branch, "2026Q1");
}

#[tokio::test]
async fn update_project_branch_renames_through_the_port() {
    let service = make_service();
    let created = service
        .create_project_branch(apis::project::CreateProjectBranchRequest {
            project_id: 1,
            branch: "old".into(),
        })
        .await
        .expect("create");
    let updated = service
        .update_project_branch(apis::project::UpdateProjectBranchRequest {
            id: created.id,
            project_id: 1,
            branch: Some("new".into()),
        })
        .await
        .expect("update");
    assert_eq!(updated.branch, "new");
}

#[tokio::test]
async fn get_project_branch_by_id_reports_an_unknown_branch_as_not_found() {
    let service = make_service();
    let err = service.get_project_branch_by_id(42).await.unwrap_err();
    assert!(matches!(err, ProjectApiError::BranchNotFound(42)));
}
