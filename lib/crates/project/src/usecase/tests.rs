//! Tests for the usecase layer.
//!
//! Mock repository + a mock `UserService` stand in for the real
//! adapters so the orchestration + view projection can be exercised
//! without infrastructure.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use crate::domain::{
    DomainError, ModelVersion, Project, ProjectBranch, ProjectBranchNew, ProjectBranchRepository,
    ProjectBranchUpdate, ProjectConfiguration, ProjectMember, ProjectNew, ProjectRepository,
    ProjectTag, ProjectUpdate, UserService, UserSummary,
};
use crate::usecase::commands::{
    CreateProject, CreateProjectBranch, UpdateProject, UpdateProjectBranch,
};
use crate::usecase::error::UsecaseError;
use crate::usecase::project_branch_usecase::ProjectBranchUsecase;
use crate::usecase::project_usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- mock project repo ----------

#[derive(Default)]
struct MockProjectState {
    projects: HashMap<i32, Project>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockProjectRepo {
    state: Arc<Mutex<MockProjectState>>,
}

impl MockProjectRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockProjectState {
                projects: HashMap::new(),
                next_id: 1,
            })),
        }
    }
}

#[async_trait]
impl ProjectRepository for MockProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.projects.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint projects_code_unique)".into(),
            ));
        }
        let id = s.next_id;
        s.next_id += 1;
        let now = mock_now();
        let members = input.members.unwrap_or_default();
        let unblind_members = input.unblind_members.unwrap_or_default();
        let configuration = input.configuration.unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            members,
            unblind_members,
            configuration,
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
        Ok(project)
    }
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        self.state
            .lock()
            .unwrap()
            .projects
            .get(&id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        self.state
            .lock()
            .unwrap()
            .projects
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
        if let Some(ref code) = input.code {
            let dup = s
                .projects
                .values()
                .any(|other| other.code == *code && other.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
        }
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref code) = input.code {
            p.code = code.clone();
        }
        if let Some(ref desc) = input.description {
            p.description = desc.clone();
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        // Replace membership wholesale per team.
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
        }
        if let Some(ref configuration) = input.configuration {
            p.configurations = configuration.clone();
        }
        Ok(p.clone())
    }
}

// ---------- mock branch repo ----------

#[derive(Default)]
struct MockBranchState {
    branches: HashMap<i32, ProjectBranch>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockBranchRepo {
    state: Arc<Mutex<MockBranchState>>,
}

impl MockBranchRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockBranchState {
                branches: HashMap::new(),
                next_id: 1,
            })),
        }
    }
}

#[async_trait]
impl ProjectBranchRepository for MockBranchRepo {
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.branches
            .values()
            .any(|b| b.project_id == input.project_id && b.branch == input.branch)
        {
            return Err(DomainError::DuplicateBranch(input.branch));
        }
        let id = s.next_id;
        s.next_id += 1;
        let now = mock_now();
        // The switch: a branch created as active deactivates the
        // project's other active branches.
        if input.active {
            for b in s.branches.values_mut() {
                if b.project_id == input.project_id {
                    b.active = false;
                }
            }
        }
        let branch = ProjectBranch::for_repository(
            id,
            input.project_id,
            input.branch,
            input.active,
            now,
            now,
        );
        s.branches.insert(id, branch.clone());
        Ok(branch)
    }
    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError> {
        self.state
            .lock()
            .unwrap()
            .branches
            .get(&id)
            .cloned()
            .ok_or(DomainError::BranchNotFound(id))
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
        // The switch: activating this branch deactivates the project's
        // other active branches.
        if input.active == Some(true) {
            for o in s.branches.values_mut() {
                if o.project_id == input.project_id && o.id != input.id {
                    o.active = false;
                }
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
        if let Some(a) = input.active {
            b.active = a;
        }
        Ok(b.clone())
    }
}

// ---------- mock user service ----------

#[derive(Clone, Default)]
struct MockUserService {
    users: Arc<Mutex<Vec<UserSummary>>>,
}

impl MockUserService {
    fn with_users(users: Vec<UserSummary>) -> Self {
        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[async_trait]
impl UserService for MockUserService {
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

// ---------- fixtures ----------

fn make_usecase() -> (
    MockProjectRepo,
    MockUserService,
    ProjectUsecase<MockProjectRepo, MockUserService>,
) {
    let projects = MockProjectRepo::new();
    let users = MockUserService::with_users(vec![
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
        project_repo: projects.clone(),
        users: users.clone(),
    });
    (projects, users, usecase)
}

// ---------- tests ----------

#[tokio::test]
async fn create_project_without_membership_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: None,
        })
        .await
        .expect("create");
    assert_eq!(view.code, "proj1");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
    assert!(view.configurations.tags.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_membership() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(ProjectMember::default()),
            configuration: None,
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders.len(), 1);
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
            configuration: None,
        })
        .await
        .expect_err("unknown member rejected");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::UserNotFound(ref c)) if c == "ghost"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn create_project_with_configuration_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![
                    ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                    ProjectTag::for_repository("Region".into(), "EU".into()),
                ],
                None,
                None,
            )),
        })
        .await
        .expect("create");
    assert_eq!(view.configurations.tags.len(), 2);
    assert_eq!(view.configurations.tags[0].key, "Product");
    assert_eq!(view.configurations.tags[0].value, "DEMO-001");
    assert_eq!(view.configurations.tags[1].key, "Region");
    assert_eq!(view.configurations.tags[1].value, "EU");
}

#[tokio::test]
async fn create_project_with_duplicate_tag_keys_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![
                    ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                    ProjectTag::for_repository("Product".into(), "DEMO-002".into()),
                ],
                None,
                None,
            )),
        })
        .await
        .expect("create");
    assert_eq!(view.configurations.tags.len(), 2);
    assert_eq!(view.configurations.tags[0].value, "DEMO-001");
    assert_eq!(view.configurations.tags[1].value, "DEMO-002");
}

#[tokio::test]
async fn create_project_with_empty_tag_key_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![ProjectTag::for_repository("".into(), "v".into())],
                None,
                None,
            )),
        })
        .await
        .expect_err("empty key rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyTagKey)
    ));
}

#[tokio::test]
async fn create_project_with_empty_tag_value_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![ProjectTag::for_repository("k".into(), "   ".into())],
                None,
                None,
            )),
        })
        .await
        .expect_err("empty value rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyTagValue)
    ));
}

#[tokio::test]
async fn create_project_with_sdtmig_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![],
                Some(ModelVersion::for_repository(7, "2024-03-29".into())),
                None,
            )),
        })
        .await
        .expect("create");
    let sdtmig = view.configurations.sdtmig.expect("sdtmig present");
    assert_eq!(sdtmig.version_id, 7);
    assert_eq!(sdtmig.version_name, "2024-03-29");
}

#[tokio::test]
async fn create_project_with_empty_sdtmig_name_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![],
                Some(ModelVersion::for_repository(7, "   ".into())),
                None,
            )),
        })
        .await
        .expect_err("empty sdtmig name rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptySdtmigName)
    ));
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
            configuration: None,
        })
        .await
        .expect("create");
    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            members: Some(ProjectMember {
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
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![ProjectTag::for_repository("k1".into(), "v1".into())],
                None,
                None,
            )),
        })
        .await
        .expect("create");
    assert_eq!(created.configurations.tags.len(), 1);

    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![
                    ProjectTag::for_repository("k2".into(), "v2".into()),
                    ProjectTag::for_repository("k3".into(), "v3".into()),
                ],
                None,
                None,
            )),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.configurations.tags.len(), 2);
    assert_eq!(updated.configurations.tags[0].key, "k2");
    assert_eq!(updated.configurations.tags[1].key, "k3");
}

#[tokio::test]
async fn update_project_leaves_configuration_unchanged_when_none() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![ProjectTag::for_repository("k1".into(), "v1".into())],
                None,
                None,
            )),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            description: Some("new".into()),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.configurations.tags.len(), 1);
    assert_eq!(updated.configurations.tags[0].key, "k1");
}

#[tokio::test]
async fn list_projects_returns_all_views() {
    let (_projects, _users, usecase) = make_usecase();
    let _ = usecase
        .create_project(CreateProject {
            code: "p1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: None,
        })
        .await
        .unwrap();
    let _ = usecase
        .create_project(CreateProject {
            code: "p2".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: None,
        })
        .await
        .unwrap();
    let list = usecase.list_projects().await.expect("list");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn project_usecase_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectUsecase<MockProjectRepo, MockUserService>>();
}

// ---------- branch usecase ----------

#[tokio::test]
async fn create_branch_rejects_a_blank_name() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let err = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "   ".into(),
            active: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyBranchName)
    ));
}

#[tokio::test]
async fn create_branch_stores_the_trimmed_name() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let view = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "  2026Q1  ".into(),
            active: false,
        })
        .await
        .expect("create");
    assert_eq!(view.branch, "2026Q1");
}

#[tokio::test]
async fn update_branch_rejects_a_blank_name() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let err = usecase
        .update_branch(UpdateProjectBranch {
            id: 1,
            project_id: 1,
            branch: Some("".into()),
            active: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyBranchName)
    ));
}

#[tokio::test]
async fn list_branches_returns_only_the_project_s_own_branches() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "a".into(),
            active: false,
        })
        .await
        .expect("create a");
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 2,
            branch: "b".into(),
            active: false,
        })
        .await
        .expect("create b");
    let branches = usecase.list_branches(1).await.expect("list");
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].branch, "a");
}

#[tokio::test]
async fn update_branch_renames_the_branch() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let created = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "old".into(),
            active: false,
        })
        .await
        .expect("create");
    let updated = usecase
        .update_branch(UpdateProjectBranch {
            id: created.id,
            project_id: 1,
            branch: Some("new".into()),
            active: None,
        })
        .await
        .expect("update");
    assert_eq!(updated.branch, "new");
}

#[tokio::test]
async fn create_branch_rejects_a_name_that_collides_only_after_trimming() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "2026Q1".into(),
            active: false,
        })
        .await
        .expect("create");
    let err = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "  2026Q1  ".into(),
            active: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::DuplicateBranch(_))
    ));
}

#[tokio::test]
async fn list_branches_of_a_project_with_no_branches_is_empty() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let branches = usecase.list_branches(1).await.expect("list");
    assert!(branches.is_empty());
}

#[tokio::test]
async fn get_branch_by_id_reports_an_unknown_branch_as_not_found() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let err = usecase.get_branch_by_id(42).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::BranchNotFound(42))
    ));
}

#[tokio::test]
async fn project_branch_usecase_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectBranchUsecase<MockBranchRepo>>();
}

#[tokio::test]
async fn activating_a_branch_deactivates_the_project_s_other_branches() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let first = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "a".into(),
            active: true,
        })
        .await
        .expect("create a");
    let second = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "b".into(),
            active: false,
        })
        .await
        .expect("create b");

    usecase
        .update_branch(UpdateProjectBranch {
            id: second.id,
            project_id: 1,
            branch: None,
            active: Some(true),
        })
        .await
        .expect("activate b");

    let branches = usecase.list_branches(1).await.expect("list");
    let active: Vec<_> = branches.iter().filter(|b| b.active).collect();
    assert_eq!(active.len(), 1, "exactly one branch may be active");
    assert_eq!(active[0].id, second.id);
    let first_after = branches.iter().find(|b| b.id == first.id).expect("a");
    assert!(
        !first_after.active,
        "the previous active branch is deactivated"
    );
}

#[tokio::test]
async fn creating_an_active_branch_deactivates_the_project_s_other_branches() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "a".into(),
            active: true,
        })
        .await
        .expect("create a");
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "b".into(),
            active: true,
        })
        .await
        .expect("create b");

    let branches = usecase.list_branches(1).await.expect("list");
    let active: Vec<_> = branches.iter().filter(|b| b.active).collect();
    assert_eq!(active.len(), 1, "exactly one branch may be active");
    assert_eq!(active[0].branch, "b");
}

#[tokio::test]
async fn activating_a_branch_leaves_another_project_s_branches_alone() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "a".into(),
            active: true,
        })
        .await
        .expect("create a");
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 2,
            branch: "b".into(),
            active: true,
        })
        .await
        .expect("create b");
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 2,
            branch: "c".into(),
            active: false,
        })
        .await
        .expect("create c");

    usecase
        .update_branch(UpdateProjectBranch {
            id: 3,
            project_id: 2,
            branch: None,
            active: Some(true),
        })
        .await
        .expect("activate c");

    let p1 = usecase.list_branches(1).await.expect("list p1");
    assert!(p1[0].active, "project 1's branch is untouched");
    let p2 = usecase.list_branches(2).await.expect("list p2");
    let active: Vec<_> = p2.iter().filter(|b| b.active).collect();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].branch, "c");
}
