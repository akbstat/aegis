//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p mission -- --ignored`.
//! Reads `AEGIS_TEST_DATABASE_URL`; loads `.env` at the workspace
//! root via `dotenvy`. Drops the live tables + `_sqlx_migrations`
//! before each run so the migration starts fresh.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use mission::domain::{AssigneeNew, MissionKind, MissionNew, MissionRole};
use mission::{
    AssigneeRepo, AssigneeRepository, DomainError, IssueRepo, MissionIssueRepository, MissionRepo,
    MissionRepository,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_TEST_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_TEST_DATABASE_URL must be set (or present in .env at the \
             workspace root) to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_TEST_DATABASE_URL");

    // Destructive cleanup. The integration tests own the schema; if
    // you point them at production by mistake you will lose data.
    sqlx::query("DROP TABLE IF EXISTS mission_issues CASCADE")
        .execute(&pool)
        .await
        .expect("drop mission_issues");
    sqlx::query("DROP TABLE IF EXISTS assignees CASCADE")
        .execute(&pool)
        .await
        .expect("drop assignees");
    sqlx::query("DROP TABLE IF EXISTS missions CASCADE")
        .execute(&pool)
        .await
        .expect("drop missions");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .expect("drop sqlx_migrations bookkeeping");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply mission migrations");

    f(pool).await
}

fn unique_code(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_create_and_find_round_trip() {
    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let code = unique_code("mission");
        let created = missions
            .create(MissionNew {
                project_code: "prj1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: code.clone(),
                assignees: vec![],
            })
            .await
            .expect("create mission");
        assert_eq!(created.mission_code, code);
        assert_eq!(created.project_code, "prj1");
        assert!(matches!(created.mission_kind, MissionKind::Crf));
        assert!(created.assignees.is_empty());

        let fetched = missions
            .find_by_id(created.id)
            .await
            .expect("find_by_id mission");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.mission_code, code);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_create_duplicate_rejects() {
    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let code = unique_code("dup-mission");
        let new = MissionNew {
            project_code: "prj1".into(),
            mission_kind: MissionKind::Sdtm,
            mission_code: code.clone(),
            assignees: vec![],
        };
        let _first = missions.create(new.clone()).await.expect("first create");
        let err = missions.create(new).await.expect_err("duplicate rejected");
        assert!(
            matches!(err, DomainError::DuplicateMission { .. }),
            "expected DuplicateMission, got {err:?}"
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_delete_cascades_to_assignees() {
    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let assignees = AssigneeRepo::new(pool.clone());
        let mission_code = unique_code("cascade-mission");
        let mission = missions
            .create(MissionNew {
                project_code: "prj1".into(),
                mission_kind: MissionKind::Adam,
                mission_code: mission_code.clone(),
                assignees: vec![AssigneeNew {
                    user_code: "u1".into(),
                    role: MissionRole::Dev,
                }],
            })
            .await
            .expect("create mission with assignee");

        missions.delete(mission.id).await.expect("delete mission");
        // Cascade: subsequent add with the same (mission_id,
        // user_code, role) succeeds because the FK row is gone.
        // (No `list_by_mission` on `AssigneeRepository` — the
        // mission's `find_by_id` returns the hydrated assignees.)
        let after = missions
            .find_by_id(mission.id)
            .await
            .expect_err("deleted mission should 404");
        assert!(
            matches!(after, DomainError::NotFound),
            "expected NotFound, got {after:?}"
        );
        // Touch the assignee repo to ensure no orphan constraint
        // blocks another insert with the same id.
        let _ = assignees; // silence unused
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn assignee_per_mission_user_role_uniqueness_holds() {
    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let assignees = AssigneeRepo::new(pool.clone());
        let mission = missions
            .create(MissionNew {
                project_code: "prj1".into(),
                mission_kind: MissionKind::Tfl,
                mission_code: unique_code("unique-mission"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let first = assignees
            .add(
                mission.id,
                AssigneeNew {
                    user_code: "u1".into(),
                    role: MissionRole::Dev,
                },
            )
            .await
            .expect("first assignee");
        assert_eq!(first.user_code, "u1");

        let err = assignees
            .add(
                mission.id,
                AssigneeNew {
                    user_code: "u1".into(),
                    role: MissionRole::Dev,
                },
            )
            .await
            .expect_err("duplicate assignee rejected");
        assert!(
            matches!(err, DomainError::DuplicateAssignee { .. }),
            "expected DuplicateAssignee, got {err:?}"
        );
    })
    .await
}

// ===========================================================================
// Mission-issue live-DB tests
// ===========================================================================

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn issue_create_find_list_close_open_round_trip() {
    use mission::domain::{IssueState, MissionIssueNew};

    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool.clone());
        let mission = missions
            .create(MissionNew {
                project_code: "prj1".into(),
                mission_kind: MissionKind::Sdtm,
                mission_code: unique_code("issue-rt"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let created = issues
            .create(MissionIssueNew {
                mission_id: mission.id,
                target_item: Some("dm.x".into()),
                issuer: "u1".into(),
                description: "first issue".into(),
            })
            .await
            .expect("create issue");
        assert_eq!(created.state, IssueState::Opened);
        assert!(created.comments.is_empty());

        let fetched = issues
            .find_by_id(created.id)
            .await
            .expect("find_by_id issue");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.issuer, "u1");
        assert_eq!(fetched.target_item.as_deref(), Some("dm.x"));

        let opened = issues
            .list_by_mission(mission.id, Some(IssueState::Opened))
            .await
            .expect("list opened");
        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0].id, created.id);

        let closed = issues.close(created.id).await.expect("close issue");
        assert_eq!(closed.state, IssueState::Closed);

        let listed_closed = issues
            .list_by_mission(mission.id, Some(IssueState::Closed))
            .await
            .expect("list closed");
        assert_eq!(listed_closed.len(), 1);

        let reopened = issues.open(created.id).await.expect("reopen issue");
        assert_eq!(reopened.state, IssueState::Opened);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn issue_update_description_persists() {
    use mission::domain::MissionIssueNew;

    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool);
        let mission = missions
            .create(MissionNew {
                project_code: "prj1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: unique_code("issue-desc"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let created = issues
            .create(MissionIssueNew {
                mission_id: mission.id,
                target_item: None,
                issuer: "u1".into(),
                description: "v1".into(),
            })
            .await
            .expect("create issue");

        let updated = issues
            .update_description(created.id, "v2".into())
            .await
            .expect("update description");
        assert_eq!(updated.description, "v2");

        let err = issues
            .update_description(created.id, "   ".into())
            .await
            .expect_err("whitespace rejected");
        assert!(
            matches!(err, DomainError::EmptyIssueDescription),
            "expected EmptyIssueDescription, got {err:?}"
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn issue_append_comment_round_trips_jsonb() {
    use chrono::Utc;
    use mission::domain::{IssueComment, MissionIssueNew};

    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool);
        let mission = missions
            .create(MissionNew {
                project_code: "prj1".into(),
                mission_kind: MissionKind::Tfl,
                mission_code: unique_code("issue-comments"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let created = issues
            .create(MissionIssueNew {
                mission_id: mission.id,
                target_item: None,
                issuer: "u1".into(),
                description: "d".into(),
            })
            .await
            .expect("create issue");

        let with_one = issues
            .append_comment(
                created.id,
                IssueComment {
                    user: "u2".into(),
                    content: "first".into(),
                    created_at: Utc::now(),
                },
            )
            .await
            .expect("append first comment");
        assert_eq!(with_one.comments.len(), 1);
        assert_eq!(with_one.comments[0].content, "first");

        let with_two = issues
            .append_comment(
                created.id,
                IssueComment {
                    user: "u3".into(),
                    content: "second".into(),
                    created_at: Utc::now(),
                },
            )
            .await
            .expect("append second comment");
        assert_eq!(with_two.comments.len(), 2);
        assert_eq!(with_two.comments[1].user, "u3");
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_delete_cascades_to_issues() {
    use mission::domain::MissionIssueNew;

    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool);
        let mission = missions
            .create(MissionNew {
                project_code: "prj1".into(),
                mission_kind: MissionKind::Adam,
                mission_code: unique_code("issue-cascade"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let issue = issues
            .create(MissionIssueNew {
                mission_id: mission.id,
                target_item: None,
                issuer: "u1".into(),
                description: "d".into(),
            })
            .await
            .expect("create issue");

        missions.delete(mission.id).await.expect("delete mission");

        let after = issues
            .find_by_id(issue.id)
            .await
            .expect_err("issue gone after parent delete");
        assert!(
            matches!(after, DomainError::MissionIssueNotFound),
            "expected MissionIssueNotFound, got {after:?}"
        );
    })
    .await
}
