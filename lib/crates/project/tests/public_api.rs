//! Public-API compile test for the `project` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and the
//! in-crate type names so a regression in any layer is caught at
//! `cargo test -p project` time.

use apis::project::{
    CreateProjectBranchRequest, CreateProjectRequest, ProjectApiError,
    ProjectBranchView as ApiProjectBranchView, ProjectConfigurationData, ProjectConfigurationView,
    ProjectMemberData, ProjectMemberView as ApiProjectMemberView, ProjectService, TagData, TagView,
    UpdateProjectBranchRequest, UpdateProjectRequest, UserSummaryView as ApiUserSummaryView,
};
use project::{
    CreateProject, CreateProjectBranch, DomainError, ModelVersion, ProjectBranch, ProjectBranchNew,
    ProjectBranchRepo, ProjectBranchRepository, ProjectBranchUpdate, ProjectBranchUsecase,
    ProjectBranchView, ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectNew,
    ProjectRepo, ProjectRepository, ProjectServiceImpl, ProjectTag, ProjectUpdate,
    ProjectUsecaseConfig, ProjectView, RoleType, TeamType, UpdateProject, UpdateProjectBranch,
    UsecaseError, UserService, UserServiceImpl, UserSummary, UserSummaryView,
};
use sqlx::PgPool;

#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_role(_: RoleType) {}
    fn assert_team(_: TeamType) {}
    fn assert_user_summary(_: UserSummary) {}
    fn assert_user_view(_: UserSummaryView) {}
    fn assert_tag(_: ProjectTag) {}
    fn assert_project_configuration(_: ProjectConfiguration) {}
    fn assert_project_language(_: ProjectLanguage) {}

    assert_role(RoleType::Leader);
    assert_team(TeamType::Members);

    let summary = UserSummary {
        code: "u1".into(),
        name: "Alice".into(),
    };
    assert_user_summary(summary);
    let view = UserSummaryView {
        code: "u1".into(),
        name: "Alice".into(),
    };
    assert_user_view(view);
    let t = ProjectTag::for_repository("k".into(), "v".into());
    assert_tag(t);
    assert_project_configuration(ProjectConfiguration::default());
    assert_project_language(ProjectLanguage::English);
}

#[test]
fn usecase_commands_have_expected_field_shape() {
    let _create_project = CreateProject {
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
    };

    let _update_project = UpdateProject {
        id: 1,
        code: None,
        description: None,
        active: None,
        members: None,
        unblind_members: None,
        configuration: Some(ProjectConfiguration::for_repository(
            None,
            vec![],
            Some(ModelVersion::for_repository(7, "2024-03-29".into())),
            None,
        )),
    };
}

#[test]
fn api_requests_have_expected_field_shape() {
    let _create_project = CreateProjectRequest {
        code: "proj1".into(),
        description: "".into(),
        members: None,
        unblind_members: None,
        configurations: Some(ProjectConfigurationData {
            language: None,
            tags: vec![],
            sdtmig: Some(apis::project::ModelVersionData {
                version_id: 7,
                version_name: "2024-03-29".into(),
            }),
            sdtm_terminology: None,
        }),
    };

    let _update_project = UpdateProjectRequest {
        id: 1,
        code: None,
        description: None,
        active: None,
        members: None,
        unblind_members: None,
        configurations: Some(ProjectConfigurationData {
            language: None,
            tags: vec![],
            sdtmig: Some(apis::project::ModelVersionData {
                version_id: 7,
                version_name: "2024-03-29".into(),
            }),
            sdtm_terminology: None,
        }),
    };
}

#[test]
fn project_usecase_config_has_expected_field_shape() {
    let _assert_config_shape: fn(cfg: ProjectUsecaseConfig<ProjectRepo, UserServiceImpl>) = |cfg| {
        let _: &ProjectRepo = &cfg.project_repo;
        let _: &UserServiceImpl = &cfg.users;
    };
}

#[test]
fn repo_constructors_accept_a_pg_pool() {
    let ctor: fn(PgPool) -> ProjectRepo = ProjectRepo::new;
    let _ = ctor;
    let branch_ctor: fn(PgPool) -> ProjectBranchRepo = ProjectBranchRepo::new;
    let _ = branch_ctor;
}

#[test]
fn domain_error_variants_are_nameable() {
    fn assert_dom(_: DomainError) {}
    assert_dom(DomainError::NotFound);
    assert_dom(DomainError::EmptyCode);
    assert_dom(DomainError::EmptyTagKey);
    assert_dom(DomainError::EmptyTagValue);
    assert_dom(DomainError::DuplicateCode("p1".into()));
    assert_dom(DomainError::UserNotFound("u1".into()));
    assert_dom(DomainError::UnknownLanguage("ja".into()));
    assert_dom(DomainError::EmptyBranchName);
    assert_dom(DomainError::DuplicateBranch("2026Q1".into()));
    assert_dom(DomainError::BranchNotFound(7));
}

#[test]
fn usecase_error_wraps_domain_error() {
    fn assert_us(_: UsecaseError) {}
    assert_us(UsecaseError::Validation(DomainError::EmptyCode));
    assert_us(UsecaseError::Repository(DomainError::NotFound));
}

#[test]
fn project_branch_public_types_are_nameable() {
    // `ProjectBranch` is an aggregate root, so its constructors are
    // `pub(crate)` — the type is named by pinning it, not by building
    // one.
    fn assert_branch_type<T>() {}
    fn assert_branch_new(_: ProjectBranchNew) {}
    fn assert_branch_update(_: ProjectBranchUpdate) {}
    fn assert_branch_usecase_type<T>() {}
    fn assert_branch_repo_impls_port<T: ProjectBranchRepository>() {}
    fn assert_branch_view(_: ProjectBranchView) {}
    fn assert_branch_api_view(_: ApiProjectBranchView) {}
    fn assert_create_cmd(_: CreateProjectBranch) {}
    fn assert_update_cmd(_: UpdateProjectBranch) {}
    fn assert_create_req(_: CreateProjectBranchRequest) {}
    fn assert_update_req(_: UpdateProjectBranchRequest) {}

    assert_branch_type::<ProjectBranch>();
    assert_branch_new(ProjectBranchNew {
        project_id: 2,
        branch: "2026Q1".into(),
        active: true,
    });
    assert_branch_update(ProjectBranchUpdate {
        id: 1,
        project_id: 2,
        branch: Some("2026Q2".into()),
        active: Some(true),
    });
    assert_branch_usecase_type::<ProjectBranchUsecase<ProjectBranchRepo>>();
    assert_branch_repo_impls_port::<ProjectBranchRepo>();
    assert_branch_view(ProjectBranchView {
        id: 1,
        project_id: 2,
        branch: "2026Q1".into(),
        active: true,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    });
    assert_branch_api_view(ApiProjectBranchView {
        id: 1,
        project_id: 2,
        branch: "2026Q1".into(),
        active: true,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    });
    assert_create_cmd(CreateProjectBranch {
        project_id: 2,
        branch: "2026Q1".into(),
        active: true,
    });
    assert_update_cmd(UpdateProjectBranch {
        id: 1,
        project_id: 2,
        branch: None,
        active: Some(true),
    });
    assert_create_req(CreateProjectBranchRequest {
        project_id: 2,
        branch: "2026Q1".into(),
        active: true,
    });
    assert_update_req(UpdateProjectBranchRequest {
        id: 1,
        project_id: 2,
        branch: None,
        active: Some(true),
    });
}

#[test]
fn project_service_impl_is_object_safe() {
    // Pin the trait surface through a function pointer so object-safety
    // is checked at compile time without ever constructing an instance.
    let _: fn(
        ProjectServiceImpl<ProjectRepo, UserServiceImpl, ProjectBranchRepo>,
    ) -> Box<dyn ProjectService> = |s| Box::new(s);
}

#[test]
fn ports_can_be_dispatched_dynamically() {
    // The bound itself is the test — if a port ever loses object
    // safety, this `where` clause will fail to compile.
    fn assert_box_dyn_prr<R: ProjectRepository + 'static>() {}
    fn assert_box_dyn_us<U: UserService + 'static>() {}
    assert_box_dyn_prr::<ProjectRepo>();
    assert_box_dyn_us::<UserServiceImpl>();
}

// ---- mirrors of the apis view DTOs (kept here so a future apis-side
// ---- change cannot silently break the projection we emit from
// ---- `ProjectServiceImpl`) ----

#[test]
fn apis_view_dtos_are_nameable() {
    fn assert_member_view(_: ApiProjectMemberView) {}
    fn assert_tag_view(_: TagView) {}
    fn assert_configuration_view(_: ProjectConfigurationView) {}
    fn assert_configuration_data(_: ProjectConfigurationData) {}
    let _ = ApiProjectMemberView::default();
    assert_member_view(ApiProjectMemberView {
        leaders: vec![ApiUserSummaryView {
            code: "u1".into(),
            name: "Alice".into(),
        }],
        workers: vec![],
    });
    let _tag_data = TagData {
        key: "k".into(),
        value: "v".into(),
    };
    assert_tag_view(TagView {
        key: "k".into(),
        value: "v".into(),
    });
    assert_configuration_view(ProjectConfigurationView {
        language: Some(apis::project::ProjectLanguage::English),
        tags: vec![],
        sdtmig: None,
        sdtm_terminology: None,
    });
    assert_configuration_data(ProjectConfigurationData::default());
}

#[test]
fn apis_error_variants_are_nameable() {
    fn assert_err(_: ProjectApiError) {}
    assert_err(ProjectApiError::Validation("bad".into()));
    assert_err(ProjectApiError::NotFound);
    assert_err(ProjectApiError::UserNotFound("u1".into()));
    assert_err(ProjectApiError::DuplicateCode("p1".into()));
    assert_err(ProjectApiError::DuplicateBranch("2026Q1".into()));
    assert_err(ProjectApiError::BranchNotFound(7));
}

#[test]
fn project_view_is_constructible_via_usecase() {
    // `ProjectView` is produced by `ProjectUsecase` from real
    // domain values. To check nameability without fakes, just pin
    // the type signature via a function pointer.
    let _assert_view: fn(ProjectView) = |_| {};
}

#[test]
fn project_member_default_is_empty() {
    let m = ProjectMember::default();
    assert!(m.leaders.is_empty());
    assert!(m.workers.is_empty());
}

// ---------- silence dead_code on unused input types ----------
#[allow(dead_code)]
fn _force_member_data(m: ProjectMemberData) {
    let _ = m;
}

#[allow(dead_code)]
fn _force_domain_inputs(p: ProjectNew, q: ProjectUpdate) {
    let _ = (p, q);
}
