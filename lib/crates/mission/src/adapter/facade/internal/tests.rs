use std::sync::Arc;

use apis::mission::{
    Actor, AssigneeData, CreateMissionRequest, ListMissionsByProjectRequest,
    ListMissionsByUserRequest, MissionKind as ApiKind, MissionRole as ApiRole, MissionService,
};

use crate::usecase::{
    MissionIssueUsecase, MissionIssueUsecaseConfig, MissionUsecase, MissionUsecaseConfig,
};

#[path = "../../../test_support.rs"]
mod test_support;
use test_support::{FakeAssigneeRepo, FakeIssueRepo, FakeMissionRepo, FakeProject, FakeUser};

use super::service::MissionServiceImpl;

fn service()
-> MissionServiceImpl<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser, FakeIssueRepo> {
    let usecase = MissionUsecase::new(MissionUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        user_lookup: FakeUser,
    });
    let issue_usecase = MissionIssueUsecase::new(MissionIssueUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        issue_repo: FakeIssueRepo::default(),
    });
    MissionServiceImpl::from_usecase(usecase, issue_usecase)
}

#[tokio::test]
async fn facade_create_mission_for_non_leader_returns_forbidden() {
    let svc = Arc::new(service());
    let err = svc
        .create_mission(
            &Actor {
                user_code: "carol".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apis::mission::MissionApiError::Forbidden { .. }
    ));
}

#[tokio::test]
async fn facade_create_then_list_by_project() {
    let svc = Arc::new(service());
    let view = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Sdtm,
                mission_code: "c1".into(),
                assignees: vec![AssigneeData {
                    user_code: "u1".into(),
                    role: ApiRole::Dev,
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(view.mission_code, "c1");

    let list = svc
        .list_missions_by_project(ListMissionsByProjectRequest {
            project_code: "p1".into(),
            kind: None,
        })
        .await
        .unwrap();
    assert_eq!(list.len(), 1);

    let user_view = svc
        .list_missions_by_user(ListMissionsByUserRequest {
            user_code: "u1".into(),
        })
        .await
        .unwrap();
    assert_eq!(user_view.len(), 1);
}

#[tokio::test]
async fn facade_add_assignee_then_remove() {
    let svc = Arc::new(service());
    let m = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap();

    let a = svc
        .add_assignee(
            &Actor {
                user_code: "alice".into(),
            },
            m.id,
            AssigneeData {
                user_code: "u2".into(),
                role: ApiRole::Qc,
            },
        )
        .await
        .unwrap();
    assert_eq!(a.user_code, "u2");

    svc.remove_assignee(
        &Actor {
            user_code: "alice".into(),
        },
        m.id,
        a.id,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn facade_duplicate_assignee_returns_duplicate_error() {
    let svc = Arc::new(service());
    let m = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap();

    // Add u1/Dev first via the standalone endpoint so the
    // (fake) assignee repo sees it.
    svc.add_assignee(
        &Actor {
            user_code: "alice".into(),
        },
        m.id,
        AssigneeData {
            user_code: "u1".into(),
            role: ApiRole::Dev,
        },
    )
    .await
    .unwrap();

    let err = svc
        .add_assignee(
            &Actor {
                user_code: "alice".into(),
            },
            m.id,
            AssigneeData {
                user_code: "u1".into(),
                role: ApiRole::Dev,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apis::mission::MissionApiError::DuplicateAssignee { .. }
    ));
}

#[tokio::test]
async fn facade_delete_cascades_assignees() {
    let svc = Arc::new(service());
    let m = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![AssigneeData {
                    user_code: "u1".into(),
                    role: ApiRole::Dev,
                }],
            },
        )
        .await
        .unwrap();
    svc.delete_mission(
        &Actor {
            user_code: "alice".into(),
        },
        m.id,
    )
    .await
    .unwrap();
    let err = svc.get_mission_by_id(m.id).await.unwrap_err();
    assert!(matches!(err, apis::mission::MissionApiError::NotFound));

    let user_view = svc
        .list_missions_by_user(ListMissionsByUserRequest {
            user_code: "u1".into(),
        })
        .await
        .unwrap();
    assert!(user_view.is_empty());
}
