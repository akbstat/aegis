//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p project -- --ignored`.
//! Reads `AEGIS_TEST_DATABASE_URL` — the disposable test database,
//! never the production one; loads `.env` at the workspace root via
//! `dotenvy` so the variable only needs to live in `.env`.
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
    let url = std::env::var("AEGIS_TEST_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_TEST_DATABASE_URL must be set (or present in .env at the workspace root) \
             to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_TEST_DATABASE_URL");

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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn the_active_column_and_its_index_are_gone_from_the_schema() {
    with_pool(|pool| async move {
        let columns: Vec<(String,)> = sqlx::query_as(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_name = 'project_branches'",
        )
        .fetch_all(&pool)
        .await
        .expect("query columns");
        let names: Vec<&str> = columns.iter().map(|(c,)| c.as_str()).collect();
        assert!(
            !names.contains(&"active"),
            "the `active` column must be gone; got {names:?}"
        );

        let indexes: Vec<(String,)> = sqlx::query_as(
            "SELECT indexname FROM pg_indexes \
             WHERE tablename = 'project_branches'",
        )
        .fetch_all(&pool)
        .await
        .expect("query indexes");
        let index_names: Vec<&str> = indexes.iter().map(|(i,)| i.as_str()).collect();
        assert!(
            !index_names.contains(&"project_branches_one_active_per_project"),
            "the partial unique index must be gone; got {index_names:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_create_then_list_returns_the_project_s_branches() {
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

        branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "2026Q1".into(),
            })
            .await
            .expect("create first branch");
        branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "2026Q2".into(),
            })
            .await
            .expect("create second branch");

        let listed: Vec<ProjectBranch> = branches
            .list_by_project(project.id)
            .await
            .expect("list branches");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].branch, "2026Q1");
        assert_eq!(listed[1].branch, "2026Q2");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_rename_via_update_replaces_the_name() {
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

        let branch = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "a".into(),
            })
            .await
            .expect("create a");

        let renamed = branches
            .update(ProjectBranchUpdate {
                id: branch.id,
                project_id: project.id,
                branch: Some("b".into()),
            })
            .await
            .expect("rename a");
        assert_eq!(renamed.branch, "b");

        let listed: Vec<ProjectBranch> = branches
            .list_by_project(project.id)
            .await
            .expect("list branches");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].branch, "b");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
            })
            .await
            .expect("create branch");

        let renamed = branches
            .update(ProjectBranchUpdate {
                id: branch.id,
                project_id: project.id,
                branch: Some("keep".into()),
            })
            .await
            .expect("rename to the same name");
        assert_eq!(renamed.branch, "keep");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
            })
            .await
            .expect("create branch");

        // The same name under another project is legal.
        branches
            .create(ProjectBranchNew {
                project_id: other.id,
                branch: "shared".into(),
            })
            .await
            .expect("create branch under the other project");

        // The same name under the same project is not.
        let err = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "shared".into(),
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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
            })
            .await
            .expect("create branch");

        let err = branches
            .update(ProjectBranchUpdate {
                id: branch.id,
                project_id: other.id,
                branch: Some("stolen".into()),
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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_for_an_unknown_project_is_rejected() {
    with_pool(|pool| async move {
        let branches = ProjectBranchRepo::new(pool);
        let err = branches
            .create(ProjectBranchNew {
                project_id: 999_999_999,
                branch: "orphan".into(),
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
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
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
            })
            .await
            .expect("create branch");

        // The usecase trims the name, so this collides with the row
        // above once the real unique constraint sees the stored value.
        let err = usecase
            .create_branch(CreateProjectBranch {
                project_id: project.id,
                branch: "  2026Q1  ".into(),
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
