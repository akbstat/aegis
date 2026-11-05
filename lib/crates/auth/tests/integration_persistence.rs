//! Live-database integration tests for the PostgreSQL adapter.
//!
//! These tests connect to a real PostgreSQL server, apply the
//! `migrations/*_create_auth_user_credentials.sql` and
//! `migrations/*_create_auth_user_domain_identities.sql` schemas, and
//! exercise the full surface of `UserCredentialsRepo` and
//! `DomainIdentityRepo`. They are `#[ignore]`-gated so that
//! `cargo test -p auth` stays green without a database; opt in with:
//!
//! ```text
//! cargo test -p auth -- --ignored
//! ```
//!
//! The connection URL is read from the `AEGIS_TEST_DATABASE_URL`
//! environment variable. If unset, the test loads `.env` from the
//! workspace root via `dotenvy`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sqlx::PgPool;

use auth::{
    AuthUsecase, AuthUsecaseConfig, DomainError, DomainIdentity, DomainIdentityRepo,
    DomainIdentityRepository, UserCredentialsRepo, UserCredentialsRepository, UserService,
    UserSummary,
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

    sqlx::query("DROP TABLE IF EXISTS auth_user_domain_identities CASCADE")
        .execute(&pool)
        .await
        .expect("drop auth_user_domain_identities");
    sqlx::query("DROP TABLE IF EXISTS auth_user_credentials CASCADE")
        .execute(&pool)
        .await
        .expect("drop auth_user_credentials");
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
async fn create_then_find_credentials_round_trip() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool.clone());
        let code = unique_code("cred");

        // Seed via direct INSERT because `UserCredentials::for_repository`
        // is `pub(crate)` and not reachable from this integration test.
        sqlx::query(
            "INSERT INTO auth_user_credentials (code, password_hash, token_version) \
             VALUES ($1, $2, $3)",
        )
        .bind(&code)
        .bind("hash")
        .bind(1_i32)
        .execute(&pool)
        .await
        .expect("seed insert");

        let fetched = repo.find_by_code(&code).await.expect("find succeeds");
        assert_eq!(fetched.code, code);
        assert_eq!(fetched.password_hash, "hash");
        assert_eq!(fetched.token_version, 1);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn bump_token_version_returns_incremented_value() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool.clone());
        let code = unique_code("bump");

        sqlx::query(
            "INSERT INTO auth_user_credentials (code, password_hash, token_version) \
             VALUES ($1, $2, $3)",
        )
        .bind(&code)
        .bind("hash")
        .bind(5_i32)
        .execute(&pool)
        .await
        .expect("seed insert");

        let v1 = repo.bump_token_version(&code).await.expect("bump 1");
        let v2 = repo.bump_token_version(&code).await.expect("bump 2");
        assert_eq!(v1, 6);
        assert_eq!(v2, 7);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn find_credentials_unknown_code_returns_not_found() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool);
        let err = repo
            .find_by_code("does-not-exist-xxxxxxxxxxxx")
            .await
            .expect_err("unknown code rejected");
        assert!(matches!(err, auth::DomainError::NotFound));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn bump_token_version_unknown_code_returns_not_found() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool);
        let err = repo
            .bump_token_version("does-not-exist-xxxxxxxxxxxx")
            .await
            .expect_err("unknown code rejected");
        assert!(matches!(err, auth::DomainError::NotFound));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn create_then_find_domain_identity_round_trip() {
    with_pool(|pool| async move {
        let repo = DomainIdentityRepo::new(pool.clone());
        let code = unique_code("ident");

        sqlx::query(
            "INSERT INTO auth_user_domain_identities \
             (user_code, domain_name, hostname, sid) VALUES ($1, $2, $3, $4)",
        )
        .bind(&code)
        .bind("DOM")
        .bind("host")
        .bind("S-1-5")
        .execute(&pool)
        .await
        .expect("insert identity");

        let id: DomainIdentity = repo
            .find(&code, "DOM", "host", "S-1-5")
            .await
            .expect("find succeeds");
        assert_eq!(id.user_code, code);
        assert_eq!(id.domain_name, "DOM");
        assert_eq!(id.hostname, "host");
        assert_eq!(id.sid, "S-1-5");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn find_domain_identity_unmatched_triple_returns_not_found() {
    with_pool(|pool| async move {
        let repo = DomainIdentityRepo::new(pool);
        let code = unique_code("ident-miss");
        let err = repo
            .find(&code, "DOM", "host", "S-1-5")
            .await
            .expect_err("unmatched triple rejected");
        assert!(matches!(err, auth::DomainError::NotFound));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL pointing at a live PostgreSQL"]
async fn usecase_can_be_constructed_from_real_repos() {
    with_pool(|pool| async move {
        // Construct a real `AuthUsecase` wired to the Postgres repos and
        // a fake user service. We don't exercise any usecase methods here
        // because that path is covered by the unit tests; the integration
        // test only asserts that the wiring compiles and constructs.
        let creds = UserCredentialsRepo::new(pool.clone());
        let ids = DomainIdentityRepo::new(pool);
        let cfg = AuthUsecaseConfig {
            credentials: creds,
            identities: ids,
            user_service: Arc::new(FakeUserService::new()),
            cache: Arc::new(auth::InMemoryTokenVersionCache::new()),
            signing_key: b"0123456789abcdef0123456789abcdef".to_vec(),
            access_ttl: std::time::Duration::from_secs(60),
            refresh_ttl: std::time::Duration::from_secs(3600),
            // The integration smoke test never calls `register_user`,
            // so any non-empty allowlist would work; the empty default
            // matches the "deny all registrations" semantics that the
            // production binary opts into via AEGIS_AUTH_ALLOW_DOMAINS.
            allow_domains: Vec::new(),
        };
        let _usecase = AuthUsecase::new(cfg);
    })
    .await;
}

/// Minimal fake `UserService` for the integration smoke test only.
pub struct FakeUserService {
    #[allow(dead_code)]
    by_code: Mutex<HashMap<String, UserSummary>>,
}

impl Default for FakeUserService {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeUserService {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            by_code: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl UserService for FakeUserService {
    async fn get_by_code(&self, _code: &str) -> Result<UserSummary, DomainError> {
        Err(DomainError::NotFound)
    }
    async fn create(&self, _code: &str, _name: &str) -> Result<UserSummary, DomainError> {
        // The smoke test never exercises this path — `register_user`
        // requires `allow_domains` and a live fixture row, neither of
        // which the integration harness sets up.
        Err(DomainError::DomainNotAllowed(
            "register_user is not exercised by the integration smoke test".into(),
        ))
    }
}
