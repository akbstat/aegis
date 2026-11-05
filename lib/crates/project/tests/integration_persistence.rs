//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p project -- --ignored`.
//! Reads the workspace-shared `AEGIS_DATABASE_URL` (same convention
//! as the `auth` and `user` crates); loads `.env` at the workspace
//! root via `dotenvy` so the variable only needs to live in `.env`.
//! Drops the live tables + `_sqlx_migrations` before each run so
//! the migration starts fresh.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use project::domain::{
    DomainError, ProjectBranch, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate,
    ProjectConfiguration, ProjectMember, ProjectNew, ProjectTag, ProjectUpdate,
};
use project::usecase::CreateProjectBranch;
use project::{
    ProjectBranchRepo, ProjectBranchUsecase, ProjectRepo, ProjectRepository, UsecaseError,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_DATABASE_URL must be set (or present in .env at the workspace root) \
             to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_DATABASE_URL");

    // Destructive cleanup. The integration tests own the schema; if
    // you point them at production by mistake you will lose data.
    sqlx::query("DROP TABLE IF EXISTS project_branches CASCADE")
        .execute(&pool)
        .await
        .expect("drop project_branches");
    sqlx::query("DROP TABLE IF EXISTS project_members CASCADE")
        .execute(&pool)
        .await
        .expect("drop project_members");
    sqlx::query("DROP TABLE IF EXISTS projects CASCADE")
        .execute(&pool)
        .await
        .expect("drop projects");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .expect("drop sqlx_migrations bookkeeping");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

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
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_no_membership_or_configuration_round_trip() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-shell"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");
        assert!(created.members.leaders.is_empty());
        assert!(created.members.workers.is_empty());
        assert!(created.unblind_members.leaders.is_empty());
        assert!(created.unblind_members.workers.is_empty());
        assert!(created.configurations.tags.is_empty());
        assert!(created.configurations.language.is_none());
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_membership_then_update_replaces_it() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-mem"),
                description: "".into(),
                members: Some(ProjectMember {
                    leaders: vec!["u1".into()],
                    workers: vec!["u2".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
                configuration: None,
            })
            .await
            .expect("create project");
        assert_eq!(created.members.leaders, vec!["u1".to_string()]);
        assert_eq!(created.members.workers, vec!["u2".to_string()]);
        assert!(created.unblind_members.leaders.is_empty());

        let updated = projects
            .update(ProjectUpdate {
                id: created.id,
                members: Some(ProjectMember {
                    leaders: vec![],
                    workers: vec!["u3".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
                ..Default::default()
            })
            .await
            .expect("update");
        assert!(updated.members.leaders.is_empty());
        assert_eq!(updated.members.workers, vec!["u3".to_string()]);
        assert!(updated.unblind_members.leaders.is_empty());
        assert!(updated.unblind_members.workers.is_empty());

        // Spot-check via direct query that no `unblind_members` rows
        // remain after the wipe.
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT team_type FROM project_members WHERE project_id = $1")
                .bind(created.id)
                .fetch_all(&pool)
                .await
                .expect("query members");
        assert!(rows.iter().all(|(t,)| t != "unblind_members"));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_configuration_round_trip() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-tags"),
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
            .expect("create project");
        assert_eq!(created.configurations.tags.len(), 2);
        assert_eq!(created.configurations.tags[0].key, "Product");
        assert_eq!(created.configurations.tags[1].value, "EU");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_update_replaces_configuration_whole_list() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-tags-replace"),
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
            .expect("create project");
        let updated = projects
            .update(ProjectUpdate {
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
        // Spot-check via direct JSONB query. The configuration payload
        // is an object; we pluck `tags` and compare as
        // `Vec<serde_json::Value>`.
        let raw: serde_json::Value =
            sqlx::query_scalar("SELECT configuration FROM projects WHERE id = $1")
                .bind(created.id)
                .fetch_one(&pool)
                .await
                .expect("query configuration");
        let raw_tags = raw
            .get("tags")
            .and_then(|v| v.as_array())
            .expect("tags array");
        assert_eq!(raw_tags.len(), 2);
        assert_eq!(raw_tags[0]["key"], "k2");
        assert_eq!(raw_tags[1]["value"], "v3");
    })
    .await;
}

// ---------- project branches ----------

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_create_then_activate_switches_the_active_branch() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-branch"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");

        let first = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "2026Q1".into(),
                active: true,
            })
            .await
            .expect("create first branch");
        assert!(first.active);

        // Creating a second *active* branch switches: the first one
        // must be deactivated.
        let second = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "2026Q2".into(),
                active: true,
            })
            .await
            .expect("create second branch");
        assert!(second.active);

        let listed = branches
            .list_by_project(project.id)
            .await
            .expect("list branches");
        assert_eq!(listed.len(), 2);
        let active: Vec<&ProjectBranch> = listed.iter().filter(|b| b.active).collect();
        assert_eq!(active.len(), 1, "at most one branch may be active");
        assert_eq!(active[0].branch, "2026Q2");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_activate_via_update_switches_the_active_branch() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-switch"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");

        let first = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "a".into(),
                active: false,
            })
            .await
            .expect("create a");
        let second = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "b".into(),
                active: false,
            })
            .await
            .expect("create b");

        branches
            .update(ProjectBranchUpdate {
                id: first.id,
                project_id: project.id,
                branch: None,
                active: Some(true),
            })
            .await
            .expect("activate a");
        // Activating the branch that is already active is a no-op, not
        // an error.
        let again = branches
            .update(ProjectBranchUpdate {
                id: first.id,
                project_id: project.id,
                branch: None,
                active: Some(true),
            })
            .await
            .expect("activate a again");
        assert!(again.active);

        let updated = branches
            .update(ProjectBranchUpdate {
                id: second.id,
                project_id: project.id,
                branch: None,
                active: Some(true),
            })
            .await
            .expect("activate b");
        assert!(updated.active);

        let listed = branches
            .list_by_project(project.id)
            .await
            .expect("list branches");
        let active: Vec<&ProjectBranch> = listed.iter().filter(|b| b.active).collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, second.id);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_rename_to_its_own_name_succeeds() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-rename"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");

        let branch = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "keep".into(),
                active: false,
            })
            .await
            .expect("create branch");

        let renamed = branches
            .update(ProjectBranchUpdate {
                id: branch.id,
                project_id: project.id,
                branch: Some("keep".into()),
                active: None,
            })
            .await
            .expect("rename to the same name");
        assert_eq!(renamed.branch, "keep");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_name_is_unique_within_a_project_but_free_across_projects() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-dup"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");
        let other = projects
            .create(ProjectNew {
                code: unique_code("proj-dup"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create other project");

        branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "shared".into(),
                active: false,
            })
            .await
            .expect("create branch");

        // The same name under another project is legal.
        branches
            .create(ProjectBranchNew {
                project_id: other.id,
                branch: "shared".into(),
                active: false,
            })
            .await
            .expect("create branch under the other project");

        // The same name under the same project is not.
        let err = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "shared".into(),
                active: false,
            })
            .await
            .expect_err("duplicate branch name");
        assert!(
            matches!(err, DomainError::DuplicateBranch(ref name) if name == "shared"),
            "expected DuplicateBranch naming the branch, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_of_another_project_is_reported_as_not_found() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-owner"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");
        let other = projects
            .create(ProjectNew {
                code: unique_code("proj-owner"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create other project");
        let branch = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "mine".into(),
                active: false,
            })
            .await
            .expect("create branch");

        let err = branches
            .update(ProjectBranchUpdate {
                id: branch.id,
                project_id: other.id,
                branch: Some("stolen".into()),
                active: None,
            })
            .await
            .expect_err("cross-project write");
        assert!(
            matches!(err, DomainError::BranchNotFound(_)),
            "expected BranchNotFound, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_for_an_unknown_project_is_rejected() {
    with_pool(|pool| async move {
        let branches = ProjectBranchRepo::new(pool);
        let err = branches
            .create(ProjectBranchNew {
                project_id: 999_999_999,
                branch: "orphan".into(),
                active: false,
            })
            .await
            .expect_err("foreign key violation");
        assert!(
            matches!(err, DomainError::BranchProjectNotFound(ref pid) if *pid == 999_999_999),
            "expected BranchProjectNotFound naming the project, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn a_name_that_collides_only_after_trimming_is_a_duplicate() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let usecase = ProjectBranchUsecase::new(ProjectBranchRepo::new(pool.clone()));
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-trim"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");

        usecase
            .create_branch(CreateProjectBranch {
                project_id: project.id,
                branch: "2026Q1".into(),
                active: false,
            })
            .await
            .expect("create branch");

        // The usecase trims the name, so this collides with the row
        // above once the real unique constraint sees the stored value.
        let err = usecase
            .create_branch(CreateProjectBranch {
                project_id: project.id,
                branch: "  2026Q1  ".into(),
                active: false,
            })
            .await
            .expect_err("trimmed duplicate");
        assert!(
            matches!(
                err,
                UsecaseError::Repository(DomainError::DuplicateBranch(_))
            ),
            "expected a duplicate branch, got {err:?}"
        );
    })
    .await;
}
