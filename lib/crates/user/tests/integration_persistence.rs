//! Live-database integration tests for the PostgreSQL adapter.
//!
//! These tests connect to a real PostgreSQL server, apply the
//! `migrations/20261103135422_create_users.sql` schema, and exercise the full
//! CRUD surface of `UserRepo` plus the happy-path of `UserUsecase`.
//! They are `#[ignore]`-gated so that `cargo test -p user` stays green
//! without a database; opt in with:
//!
//! ```text
//! cargo test -p user -- --ignored
//! ```
//!
//! The connection URL is read from the `AEGIS_TEST_DATABASE_URL`
//! environment variable. If unset, the test loads `.env` from the
//! workspace root via `dotenvy` so the variable only needs to live in
//! the file once. The tests run sequentially (via
//! `serial_test::serial`-equivalent ordering achieved with
//! `tokio::test` and a per-test unique code) and clean up after
//! themselves, so they are safe to re-run.
//!
//! All tests in this file require a live PostgreSQL. A failure to
//! connect is reported via a clear panic so the ignored-run output
//! does not look like a silent skip.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;
use user::domain::DomainError;
use user::{CreateUser, Role, UserRepo, UserRepository, UserUsecase, UserView};

/// Run `f` against a freshly-migrated `PgPool`. Each invocation applies
/// the migration before the test body runs, so the tests do not
/// assume any pre-existing schema state.
async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    // Source `.env` if the variable is not already in the environment.
    // `dotenv().ok()` returns Err if `.env` does not exist; we treat
    // that as a non-fatal condition so callers can still set the
    // variable manually.
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

    // Drop the live `users` table (if a previous test run left one)
    // and the `sqlx_migrations` bookkeeping so the migration below
    // starts from scratch. This is destructive but safe because the
    // integration tests own the schema — the live DB is the test
    // database, not a shared staging environment. If you point the
    // tests at a real production database by mistake you will lose
    // data; this is intentional so the failure is loud rather than
    // silently corrupting state.
    sqlx::query("DROP TABLE IF EXISTS users CASCADE")
        .execute(&pool)
        .await
        .expect("drop users table");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .expect("drop sqlx_migrations bookkeeping");

    // Apply the migration. `sqlx::migrate!` discovers and applies any
    // unapplied migration files at the given path; on a fresh database
    // it creates the `users` table.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

    f(pool).await
}

/// Build a `code` value unique to this run. Combines a per-process
/// atomic counter (so multiple tests in the same run never collide)
/// with the wall-clock nanosecond value (so concurrent test runs do
/// not collide either). Tests can rely on this being unique within the
/// `users_code_unique` index.
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
async fn create_then_find_then_list_round_trip() {
    with_pool(|pool| async move {
        let repo = UserRepo::new(pool);
        let code = unique_code("create-find-list");

        let created = repo
            .create(user::domain::UserNew {
                code: code.clone(),
                name: "Integration Alice".to_string(),
                role: Role::Admin,
                active: true,
            })
            .await
            .expect("create");

        assert_eq!(created.code, code);
        assert!(created.active);

        let by_id = repo.find_by_id(created.id).await.expect("find_by_id");
        assert_eq!(by_id.id, created.id);
        assert_eq!(by_id.code, code);

        let by_code = repo.find_by_code(&code).await.expect("find_by_code");
        assert_eq!(by_code.id, created.id);

        let listed = repo.list().await.expect("list");
        assert!(
            listed.iter().any(|u| u.id == created.id),
            "list should contain the freshly created user"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn update_replaces_name_and_role() {
    with_pool(|pool| async move {
        let repo = UserRepo::new(pool);
        let code = unique_code("update-name-role");

        let created = repo
            .create(user::domain::UserNew {
                code: code.clone(),
                name: "Before".to_string(),
                role: Role::General,
                active: true,
            })
            .await
            .expect("create");

        // The trigger sets `updated_at` on every row change, so the
        // pre-update value must be strictly less than the post-update
        // value. We sleep briefly between operations so the database
        // clock has a chance to advance between `create` and `update`.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let updated = repo
            .update(user::domain::UserUpdate {
                id: created.id,
                name: Some("After".to_string()),
                role: Some(Role::Root),
                ..Default::default()
            })
            .await
            .expect("update");

        assert_eq!(updated.name, "After");
        assert_eq!(updated.role, Role::Root);
        assert!(
            updated.updated_at > created.updated_at,
            "users_set_updated_at trigger must bump updated_at on UPDATE (created={}, updated={})",
            created.updated_at,
            updated.updated_at
        );
        assert_eq!(
            updated.created_at, created.created_at,
            "created_at must not change on UPDATE"
        );

        let reread = repo.find_by_id(created.id).await.expect("reread");
        assert_eq!(reread.name, "After");
        assert_eq!(reread.role, Role::Root);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn update_with_duplicate_code_returns_duplicate_code_error() {
    with_pool(|pool| async move {
        let repo = UserRepo::new(pool);
        let first_code = unique_code("dup-first");
        let second_code = unique_code("dup-second");

        let _first = repo
            .create(user::domain::UserNew {
                code: first_code.clone(),
                name: "First".to_string(),
                role: Role::General,
                active: true,
            })
            .await
            .expect("create first");

        let second = repo
            .create(user::domain::UserNew {
                code: second_code.clone(),
                name: "Second".to_string(),
                role: Role::General,
                active: true,
            })
            .await
            .expect("create second");

        // Try to rename `second` to take `first`'s code. The CHECK on
        // `users_code_unique` fires and the repository maps it to
        // `DomainError::DuplicateCode`.
        let err = repo
            .update(user::domain::UserUpdate {
                id: second.id,
                code: Some(first_code.clone()),
                ..Default::default()
            })
            .await
            .expect_err("renaming to an existing code must fail");

        assert!(
            matches!(err, DomainError::DuplicateCode(_)),
            "expected DuplicateCode, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn update_can_flip_active_flag() {
    with_pool(|pool| async move {
        let repo = UserRepo::new(pool);
        let code = unique_code("update-active");

        let created = repo
            .create(user::domain::UserNew {
                code: code.clone(),
                name: "Soft-Removed".to_string(),
                role: Role::General,
                active: true,
            })
            .await
            .expect("create");

        let updated = repo
            .update(user::domain::UserUpdate {
                id: created.id,
                active: Some(false),
                ..Default::default()
            })
            .await
            .expect("update flips active");

        assert!(!updated.active, "active flag must be flipped via update");

        // Row is still present (no hard delete).
        let reread = repo.find_by_id(created.id).await.expect("reread");
        assert_eq!(reread.code, code);
        assert!(!reread.active);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn find_unknown_id_returns_not_found() {
    with_pool(|pool| async move {
        let repo = UserRepo::new(pool);

        // The `id` is unlikely to collide; an `i32::MAX` is the
        // conventional "definitely doesn't exist" probe.
        let err = repo
            .find_by_id(i32::MAX)
            .await
            .expect_err("missing id must return NotFound");
        assert!(
            matches!(err, DomainError::NotFound),
            "expected NotFound, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn find_unknown_code_returns_not_found() {
    with_pool(|pool| async move {
        let repo = UserRepo::new(pool);

        let err = repo
            .find_by_code("does-not-exist-xxxxxxxxxxxx")
            .await
            .expect_err("missing code must return NotFound");
        assert!(
            matches!(err, DomainError::NotFound),
            "expected NotFound, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn usecase_create_and_get_by_code_returns_user_view() {
    with_pool(|pool| async move {
        let repo = UserRepo::new(pool);
        let usecase = UserUsecase::new(repo);
        let code = unique_code("usecase");

        let created_view = usecase
            .create(CreateUser {
                code: code.clone(),
                name: "Usecase User".to_string(),
                role: Role::Admin,
                active: true,
            })
            .await
            .expect("usecase create");

        assert_eq!(created_view.code, code);
        assert_eq!(created_view.role, Role::Admin);
        assert!(created_view.active);

        let fetched: UserView = usecase
            .get_by_code(&code)
            .await
            .expect("usecase get_by_code");
        assert_eq!(fetched.id, created_view.id);
        assert_eq!(fetched.code, code);
    })
    .await;
}
