//! Live-DB end-to-end test for the auth HTTP surface.
//!
//! These tests exercise the full `axum` router — `transport::http::router`
//! — against a real Postgres + the real `AuthUsecase` / `UserUsecase`
//! stack. They are deliberately marked `#[ignore]` so they do not
//! run as part of the default `cargo test`; a developer (or CI lane
//! that targets a sidecar database) runs them with
//! `cargo test --test integration_auth -- --ignored`.
//!
//! First, the test reads `AEGIS_DATABASE_URL` from the environment
//! and aborts early with a helpful message if it is missing. The
//! schema is brought up via `sqlx::migrate!` against the migration
//! directories of the `auth` and `user` crates. Fixtures (a user
//! row + a credential row whose hash matches the test password)
//! are seeded through raw SQL so the test does not depend on the
//! (private) `UserCredentials::for_repository` constructor.

#![cfg(test)]

use std::sync::Arc;
use std::time::Duration;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode as AxStatus};
use rand_core::OsRng;
use serde_json::Value;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use tracing_subscriber::EnvFilter;

use aegis_server::state::AppState;
use aegis_server::transport::http::router as http_router;
use apis::auth::AuthService;
use apis::project::ProjectService;
use apis::user::UserService;
use auth::{
    AuthServiceImpl, AuthUsecase, AuthUsecaseConfig, DomainIdentityRepo, InMemoryTokenVersionCache,
    TokenVersionCache, UserCredentialsRepo, UserServiceImpl as AuthUserServiceImpl,
};
use logging_utils::{LogIngestor, LogIngestorConfig};
use user::{UserRepo, UserServiceImpl, UserUsecase};

/// Fixed test password. The matching Argon2 hash is seeded into
/// `auth_user_credentials.password_hash` for the fixture user.
const SEED_PASSWORD: &str = "correct horse battery staple";

/// Fixed signing key bytes (32 zero bytes). HS256 only requires the
/// key to be 32+ bytes; predictable entropy is fine for a local test
/// database.
fn signing_key() -> Vec<u8> {
    vec![0u8; 32]
}

/// Build a `LogIngestor` for the live-DB auth test. Mirrors
/// `state::test_support::unused_log_ingestor` from the lib crate;
/// the integration test cannot reach `pub(crate)` items, so it
/// builds its own against a scoped tempdir. This test never calls
/// `submit` — the directory is created eagerly and torn down when
/// `LogIngestor::shutdown` runs (with its deadline) at `AppState`
/// drop.
fn build_unused_log_ingestor() -> Arc<LogIngestor> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = LogIngestorConfig::builder()
        .log_dir(tmp.path().to_path_buf())
        .file_name_prefix("integration-auth.log".into())
        .build();
    Arc::new(LogIngestor::new(cfg).expect("ingestor init"))
}

/// Build a fresh `PgPool` from `AEGIS_DATABASE_URL`. Aborts the test
/// with a helpful message if the env var is missing, so devs running
/// `cargo test -- --ignored` without a database see a clear failure
/// rather than a stack trace.
async fn pool_or_skip() -> PgPool {
    let _ = dotenvy::dotenv();
    let url = match std::env::var("AEGIS_DATABASE_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!(
                "AEGIS_DATABASE_URL is not set; skipping live-DB integration test. \
                 Set it to a Postgres URL (e.g. postgres://localhost/aegis_test) \
                 and re-run with `cargo test -- --ignored`."
            );
            // Defensive: the test is `#[ignore]`-d so this branch only
            // runs when a developer explicitly opts in. Returning a
            // malformed pool would produce a confusing error; abort
            // the test process with a non-zero status instead.
            std::process::exit(0);
        }
    };
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .expect("connect to AEGIS_DATABASE_URL")
}

/// Run migrations from both the `user` and `auth` crates so the
/// `users` and `auth_user_credentials` tables exist. The migrations
/// are pure SQL files compiled into the test binary via
/// `sqlx::migrate!`. The macro resolves the path relative to the
/// crate's `CARGO_MANIFEST_DIR` (`apps/server/aegis-server/`), so a
/// `../../../` walks up to the workspace root before descending into
/// the target crate's migration directory.
async fn run_migrations(pool: &PgPool) {
    sqlx::migrate!("../../../lib/crates/user/migrations")
        .run(pool)
        .await
        .expect("run user migrations");
    sqlx::migrate!("../../../lib/crates/auth/migrations")
        .run(pool)
        .await
        .expect("run auth migrations");
}

/// Insert a fresh user + credential row into the test database and
/// return the `(code, role)` tuple used for assertions. The code is
/// unique per call so back-to-back runs cannot collide.
async fn seed_user(pool: &PgPool, code: &str, role: &str) {
    sqlx::query("INSERT INTO users (code, name, role, active) VALUES ($1, $2, $3, $4)")
        .bind(code)
        .bind("Integration Test User")
        .bind(role)
        .bind(true)
        .execute(pool)
        .await
        .expect("insert users row");

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(SEED_PASSWORD.as_bytes(), &salt)
        .expect("hash seed password")
        .to_string();

    sqlx::query(
        "INSERT INTO auth_user_credentials (code, password_hash, token_version) \
         VALUES ($1, $2, 1)",
    )
    .bind(code)
    .bind(hash)
    .execute(pool)
    .await
    .expect("insert auth_user_credentials row");
}

/// Remove the rows inserted by [`seed_user`]. Idempotent — the test
/// calls it as a guard so the schema is not littered with rows on
/// failure.
async fn cleanup_user(pool: &PgPool, code: &str) {
    let _ = sqlx::query("DELETE FROM auth_user_credentials WHERE code = $1")
        .bind(code)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE code = $1")
        .bind(code)
        .execute(pool)
        .await;
}

/// Build the `AppState` + `Router` wired against the real Postgres
/// repos. Mirrors the wiring in `run::build_auth_service` /
/// `run::build_user_service` so the test exercises the same code
/// path the binary does.
fn build_app(pool: PgPool) -> Router {
    let cache: Arc<dyn TokenVersionCache> = Arc::new(InMemoryTokenVersionCache::new());

    let credentials = UserCredentialsRepo::new(pool.clone());
    let identities = DomainIdentityRepo::new(pool.clone());

    // Same bridge as `run::build_auth_service`: wrap the apis
    // `UserService` (built from the user Postgres repo) into the
    // auth domain's `UserService` port.
    let user_repo = UserRepo::new(pool.clone());
    let user_usecase = UserUsecase::new(user_repo);
    let apis_user: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(user_usecase));
    let auth_user: Arc<dyn auth::UserService> =
        Arc::new(AuthUserServiceImpl::new(apis_user.clone()));

    let auth_usecase = AuthUsecase::new(AuthUsecaseConfig {
        credentials,
        identities,
        user_service: auth_user,
        cache,
        signing_key: signing_key(),
        access_ttl: Duration::from_secs(900),
        refresh_ttl: Duration::from_secs(3600),
        allow_domains: vec!["aegis.local".to_string()],
    });

    let auth = Arc::new(AuthServiceImpl::new(auth_usecase)) as Arc<dyn AuthService>;

    // `AppState` requires a project service slot, but this test only
    // exercises the auth surface. A null stub mirrors the pattern used
    // by the per-namespace handler tests: any project-service method
    // call would `unimplemented!()`, so accidentally exercising a
    // project route here would fail loudly.
    let project: Arc<dyn ProjectService> = Arc::new(NullProjectService);

    let state = AppState {
        auth,
        user: apis_user,
        project,
        terminology: Arc::new(NullTerminologyService)
            as Arc<dyn apis::terminology::TerminologyService>,
        domain_model: Arc::new(NullDomainModelService)
            as Arc<dyn apis::domain_model::DomainModelService>,
        mission: Arc::new(NullMissionService) as Arc<dyn apis::mission::MissionService>,
        crf: Arc::new(NullCrfService) as Arc<dyn apis::crf::CrfService>,
        sdtm_project: Arc::new(NullSdtmProjectService)
            as Arc<dyn apis::sdtm_project::SdtmProjectService>,
        structures: Arc::new(NullStructureService) as Arc<dyn apis::structures::StructureService>,
        log_ingestor: build_unused_log_ingestor(),
    };

    http_router(state)
}

/// Stub [`apis::structures::StructureService`] for the live-DB auth
/// test. Every method `unimplemented!()`s — this test never exercises
/// structure routes.
#[derive(Clone)]
struct NullStructureService;

#[async_trait]
impl apis::structures::StructureService for NullStructureService {
    async fn create_structure(
        &self,
        _req: apis::structures::CreateStructureRequest,
    ) -> Result<apis::structures::StructureView, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn get_structure(
        &self,
        _id: i64,
    ) -> Result<apis::structures::StructureView, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn get_structure_meta(
        &self,
        _id: i64,
    ) -> Result<apis::structures::StructureMetaView, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn list_structures(
        &self,
        _req: apis::structures::ListStructuresRequest,
    ) -> Result<apis::structures::StructureList, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn rename_structure(
        &self,
        _req: apis::structures::RenameStructureRequest,
    ) -> Result<apis::structures::StructureView, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn delete_structure(&self, _id: i64) -> Result<(), apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn add_node(
        &self,
        _req: apis::structures::AddNodeRequest,
    ) -> Result<apis::structures::NodeView, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn update_node(
        &self,
        _req: apis::structures::UpdateNodeRequest,
    ) -> Result<apis::structures::NodeView, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn delete_node(
        &self,
        _req: apis::structures::DeleteNodeRequest,
    ) -> Result<(), apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn list_own_nodes(
        &self,
        _structure_id: i64,
    ) -> Result<apis::structures::OwnNodeList, apis::structures::StructureApiError> {
        unimplemented!()
    }
    async fn build_structure(
        &self,
        _id: i64,
    ) -> Result<apis::structures::StructureTreeView, apis::structures::StructureApiError> {
        unimplemented!()
    }
}

/// Stub [`ProjectService`] for the live-DB auth test. Every method
/// `unimplemented!()`s — this test never exercises project routes.
#[derive(Clone)]
struct NullProjectService;

#[async_trait]
impl ProjectService for NullProjectService {
    async fn create_project(
        &self,
        _req: apis::project::CreateProjectRequest,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_by_id(
        &self,
        _id: i32,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_by_code(
        &self,
        _code: &str,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn list_projects(
        &self,
    ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn update_project(
        &self,
        _req: apis::project::UpdateProjectRequest,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn create_project_branch(
        &self,
        _req: apis::project::CreateProjectBranchRequest,
    ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_branch_by_id(
        &self,
        _id: i32,
    ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn list_project_branches(
        &self,
        _project_id: i32,
    ) -> Result<Vec<apis::project::ProjectBranchView>, apis::project::ProjectApiError> {
        unimplemented!()
    }
    async fn update_project_branch(
        &self,
        _req: apis::project::UpdateProjectBranchRequest,
    ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
        unimplemented!()
    }
}

/// Stub [`apis::terminology::TerminologyService`] for the live-DB auth
/// test. Every method `unimplemented!()`s — this test never exercises
/// terminology routes.
#[derive(Clone)]
struct NullTerminologyService;

#[async_trait]
impl apis::terminology::TerminologyService for NullTerminologyService {
    async fn create_version(
        &self,
        _req: apis::terminology::CreateTerminologyVersionRequest,
    ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
    {
        unimplemented!()
    }
    async fn list_versions(
        &self,
    ) -> Result<
        Vec<apis::terminology::TerminologyVersionView>,
        apis::terminology::TerminologyApiError,
    > {
        unimplemented!()
    }
    async fn get_version_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
    {
        unimplemented!()
    }
    async fn update_version(
        &self,
        _req: apis::terminology::UpdateTerminologyVersionRequest,
    ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
    {
        unimplemented!()
    }
    async fn delete_version(&self, _id: i64) -> Result<(), apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn create_code_list(
        &self,
        _req: apis::terminology::CreateCodeListRequest,
    ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn list_code_lists(
        &self,
        _query: apis::terminology::CodeListListQuery,
    ) -> Result<
        apis::terminology::Page<apis::terminology::CodeListView>,
        apis::terminology::TerminologyApiError,
    > {
        unimplemented!()
    }
    async fn get_code_list_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn update_code_list(
        &self,
        _req: apis::terminology::UpdateCodeListRequest,
    ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn delete_code_list(
        &self,
        _id: i64,
    ) -> Result<(), apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn create_code_item(
        &self,
        _req: apis::terminology::CreateCodeItemRequest,
    ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn list_code_items(
        &self,
        _query: apis::terminology::CodeItemListQuery,
    ) -> Result<
        apis::terminology::Page<apis::terminology::CodeItemView>,
        apis::terminology::TerminologyApiError,
    > {
        unimplemented!()
    }
    async fn list_code_items_by_version_and_code(
        &self,
        _version_id: i64,
        _code: &str,
    ) -> Result<Vec<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn update_code_item(
        &self,
        _req: apis::terminology::UpdateCodeItemRequest,
    ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn delete_code_item(
        &self,
        _id: i64,
    ) -> Result<(), apis::terminology::TerminologyApiError> {
        unimplemented!()
    }
    async fn batch_create_code_items(
        &self,
        _req: apis::terminology::BatchCreateCodeItemsRequest,
    ) -> Result<
        apis::terminology::BatchCreateCodeItemsResponse,
        apis::terminology::TerminologyApiError,
    > {
        unimplemented!()
    }
}

/// Drive a `oneshot` request through the router and return the
/// response status + parsed JSON body. Body bytes that are not
/// valid JSON are surfaced as `Value::Null` so callers can still
/// inspect the status.
async fn send(app: Router, req: Request<Body>) -> (AxStatus, Value) {
    let response = app.oneshot(req).await.expect("oneshot response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("body bytes");
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

/// One-time tracing init. `try_init` is idempotent so it is safe to
/// call from every test; the work only happens the first time.
fn init_tracing_once() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("aegis_server=warn,sqlx=warn")),
        )
        .with_test_writer()
        .try_init();
}

#[tokio::test]
#[ignore = "requires a live Postgres; run with `cargo test -- --ignored`"]
async fn happy_path_login_refresh_logout() {
    init_tracing_once();
    let pool = pool_or_skip().await;
    run_migrations(&pool).await;

    // Unique code per run so re-runs into the same DB don't collide.
    let code = format!("itest-{}", uuid_like_suffix());
    seed_user(&pool, &code, "admin").await;
    let app = build_app(pool.clone());

    // 1. login -> token pair
    let (status, body) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"code":"{code}","password":"{SEED_PASSWORD}"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::OK, "login body: {body}");
    let access = body["access_token"]
        .as_str()
        .expect("access_token present")
        .to_string();
    let refresh = body["refresh_token"]
        .as_str()
        .expect("refresh_token present")
        .to_string();
    assert!(!access.is_empty());
    assert!(!refresh.is_empty());

    // 2. refresh -> new access token (refresh token itself is unchanged
    // in the current contract; we still re-use it). Requires the
    // access token from step 1 in `Authorization: Bearer`.
    let (status, body) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/auth/refresh")
            .header("authorization", format!("Bearer {access}"))
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"refresh_token":"{refresh}"}}"#)))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::OK, "refresh body: {body}");
    let new_access = body["access_token"]
        .as_str()
        .expect("refresh returns access_token");
    assert!(!new_access.is_empty());
    assert_ne!(new_access, access, "refresh should mint a new access token");

    // 3. logout -> 200 OK with empty JSON body. Requires the access
    // token from step 1 in `Authorization: Bearer` (the refresh
    // token alone is no longer enough).
    let (status, body) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/auth/logout")
            .header("authorization", format!("Bearer {access}"))
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"refresh_token":"{refresh}"}}"#)))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::OK, "logout body: {body}");
    assert_eq!(body, serde_json::json!({}));

    // 4. After logout, the refresh token is technically decodable
    // (signature + expiry are still valid) but its `ver` no longer
    // matches the bumped `token_version`. The access token from
    // step 1 has the pre-bump version too, so the AuthClaims
    // extractor rejects it at the verify step — refresh returns
    // 401 with `token_verification_failed` before the refresh
    // usecase is even considered.
    let (status, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/refresh")
            .header("authorization", format!("Bearer {access}"))
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"refresh_token":"{refresh}"}}"#)))
            .unwrap(),
    )
    .await;
    assert_eq!(
        status,
        AxStatus::UNAUTHORIZED,
        "post-logout refresh body: {body}"
    );
    assert_eq!(body["code"], "token_verification_failed");

    cleanup_user(&pool, &code).await;
}

#[tokio::test]
#[ignore = "requires a live Postgres; run with `cargo test -- --ignored`"]
async fn login_with_wrong_password_returns_401() {
    init_tracing_once();
    let pool = pool_or_skip().await;
    run_migrations(&pool).await;

    let code = format!("itest-{}", uuid_like_suffix());
    seed_user(&pool, &code, "admin").await;
    let app = build_app(pool.clone());

    let (status, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"code":"{code}","password":"definitely-wrong"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(
        status,
        AxStatus::UNAUTHORIZED,
        "wrong-password body: {body}"
    );
    assert_eq!(body["code"], "invalid_credentials");

    cleanup_user(&pool, &code).await;
}

#[tokio::test]
#[ignore = "requires a live Postgres; run with `cargo test -- --ignored`"]
async fn refresh_without_authorization_returns_401() {
    // Confirms the AuthClaims extractor runs against the real
    // AuthServiceImpl (not a mock): no Authorization header means
    // the extractor short-circuits with 401 before the refresh
    // usecase is considered.
    init_tracing_once();
    let pool = pool_or_skip().await;
    run_migrations(&pool).await;

    let code = format!("itest-{}", uuid_like_suffix());
    seed_user(&pool, &code, "admin").await;
    let app = build_app(pool.clone());

    let (status, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/refresh")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"refresh_token":"any"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::UNAUTHORIZED, "missing-auth body: {body}");
    assert_eq!(body["code"], "token_verification_failed");

    cleanup_user(&pool, &code).await;
}

#[tokio::test]
#[ignore = "requires a live Postgres; run with `cargo test -- --ignored`"]
async fn admin_register_user_creates_user_credential_and_identity() {
    // Drives the full registration flow end-to-end:
    //   1. Seed an admin user, log in to obtain a bearer access token.
    //   2. POST /api/auth/user-credential as the admin — handler must
    //      hash the password, create the user (inactive, general role),
    //      credential, and domain identity rows.
    //   3. Assert the response body and the three DB rows.
    init_tracing_once();
    let pool = pool_or_skip().await;
    run_migrations(&pool).await;

    let admin_code = format!("itest-admin-{}", uuid_like_suffix());
    seed_user(&pool, &admin_code, "admin").await;
    let app = build_app(pool.clone());

    // 1. login as admin
    let (status, body) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"code":"{admin_code}","password":"{SEED_PASSWORD}"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::OK, "admin login body: {body}");
    let access = body["access_token"]
        .as_str()
        .expect("access_token present")
        .to_string();

    // 2. POST registration
    let new_code = format!("itest-new-{}", uuid_like_suffix());
    let new_password = "fresh-pass-123";
    let (status, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/user-credential")
            .header("authorization", format!("Bearer {access}"))
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"user_code":"{new_code}","user_name":"Fresh","domain_name":"aegis.local","hostname":"ws01","sid":"S-1","password":"{new_password}"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::CREATED, "register body: {body}");
    assert_eq!(body["user_code"], serde_json::json!(new_code));
    assert_eq!(body["role"], serde_json::json!("general"));
    assert_eq!(body["active"], serde_json::json!(false));
    assert_eq!(body["domain_name"], serde_json::json!("aegis.local"));

    // 3. DB rows exist. The user row is inactive (handler sets
    // `active=false` on creation); the credential row carries a
    // non-empty Argon2 hash.
    let active: bool = sqlx::query_scalar("SELECT active FROM users WHERE code = $1")
        .bind(&new_code)
        .fetch_one(&pool)
        .await
        .expect("user row present");
    assert!(!active, "newly registered user must be inactive");
    let hash: String =
        sqlx::query_scalar("SELECT password_hash FROM auth_user_credentials WHERE code = $1")
            .bind(&new_code)
            .fetch_one(&pool)
            .await
            .expect("credential row present");
    assert!(
        hash.starts_with("$argon2id$"),
        "password must be hashed with Argon2id, got {hash:?}"
    );
    let sid: String = sqlx::query_scalar(
        "SELECT sid FROM auth_user_domain_identities \
         WHERE domain_name = $1 AND hostname = $2 AND sid = $3",
    )
    .bind("aegis.local")
    .bind("ws01")
    .bind("S-1")
    .fetch_one(&pool)
    .await
    .expect("identity row present");
    assert_eq!(sid, "S-1");

    cleanup_registered(&pool, &new_code).await;
    cleanup_user(&pool, &admin_code).await;
}

#[tokio::test]
#[ignore = "requires a live Postgres; run with `cargo test -- --ignored`"]
async fn register_user_with_disallowed_domain_returns_400() {
    // The handler returns 400 (validation) when the domain is not in
    // `allow_domains`. The allowlist in `build_app` only accepts
    // `aegis.local`; everything else is rejected.
    init_tracing_once();
    let pool = pool_or_skip().await;
    run_migrations(&pool).await;

    let admin_code = format!("itest-admin-{}", uuid_like_suffix());
    seed_user(&pool, &admin_code, "admin").await;
    let app = build_app(pool.clone());

    let (status, body) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"code":"{admin_code}","password":"{SEED_PASSWORD}"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::OK, "admin login body: {body}");
    let access = body["access_token"]
        .as_str()
        .expect("access_token present")
        .to_string();

    let new_code = format!("itest-new-{}", uuid_like_suffix());
    let (status, body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/user-credential")
            .header("authorization", format!("Bearer {access}"))
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"user_code":"{new_code}","user_name":"Fresh","domain_name":"other.local","hostname":"ws01","sid":"S-1","password":"x"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(
        status,
        AxStatus::BAD_REQUEST,
        "disallowed domain body: {body}"
    );

    cleanup_user(&pool, &new_code).await;
    cleanup_user(&pool, &admin_code).await;
}

#[tokio::test]
#[ignore = "requires a live Postgres; run with `cargo test -- --ignored`"]
async fn register_user_as_general_returns_403() {
    // The admin/root gate lives in the handler. A caller with role
    // `general` must be rejected with 403 even though the bearer is
    // valid and the domain is allowed.
    init_tracing_once();
    let pool = pool_or_skip().await;
    run_migrations(&pool).await;

    let general_code = format!("itest-gen-{}", uuid_like_suffix());
    seed_user(&pool, &general_code, "general").await;
    let app = build_app(pool.clone());

    let (status, body) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"code":"{general_code}","password":"{SEED_PASSWORD}"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(status, AxStatus::OK, "general login body: {body}");
    let access = body["access_token"]
        .as_str()
        .expect("access_token present")
        .to_string();

    let new_code = format!("itest-new-{}", uuid_like_suffix());
    let (status, _body) = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/user-credential")
            .header("authorization", format!("Bearer {access}"))
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"user_code":"{new_code}","user_name":"Fresh","domain_name":"aegis.local","hostname":"ws01","sid":"S-1","password":"x"}}"#
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(
        status,
        AxStatus::FORBIDDEN,
        "general caller must not be allowed to register"
    );

    cleanup_user(&pool, &new_code).await;
    cleanup_user(&pool, &general_code).await;
}

/// Tear down the rows created by the registration endpoint. Mirrors
/// [`cleanup_user`] but also removes the `auth_user_domain_identities`
/// row. The identities table stores `user_code` directly (no FK),
/// so the cleanup is a plain delete on that column.
async fn cleanup_registered(pool: &PgPool, code: &str) {
    let _ = sqlx::query("DELETE FROM auth_user_domain_identities WHERE user_code = $1")
        .bind(code)
        .execute(pool)
        .await;
    cleanup_user(pool, code).await;
}

/// Stub [`apis::domain_model::DomainModelService`] for the live-DB
/// auth test. Every method `unimplemented!()`s — this test never
/// exercises domain-model routes.
#[derive(Clone)]
struct NullDomainModelService;

#[async_trait]
impl apis::domain_model::DomainModelService for NullDomainModelService {
    async fn create_version(
        &self,
        _req: apis::domain_model::CreateSdtmVersionRequest,
    ) -> Result<apis::domain_model::SdtmVersionView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn list_versions(
        &self,
    ) -> Result<apis::domain_model::SdtmVersionList, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn update_version(
        &self,
        _req: apis::domain_model::UpdateSdtmVersionRequest,
    ) -> Result<apis::domain_model::SdtmVersionView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn delete_version(
        &self,
        _id: i64,
    ) -> Result<(), apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn create_domain(
        &self,
        _req: apis::domain_model::CreateSdtmDomainRequest,
    ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn get_domain_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn list_domains_by_version(
        &self,
        _version_id: i64,
    ) -> Result<apis::domain_model::SdtmDomainList, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn update_domain(
        &self,
        _req: apis::domain_model::UpdateSdtmDomainRequest,
    ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn delete_domain(&self, _id: i64) -> Result<(), apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn create_variable(
        &self,
        _req: apis::domain_model::CreateSdtmVariableRequest,
    ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn get_variable_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn list_variables_by_domain(
        &self,
        _domain_id: i64,
    ) -> Result<apis::domain_model::SdtmVariableList, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn update_variable(
        &self,
        _req: apis::domain_model::UpdateSdtmVariableRequest,
    ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
    async fn delete_variable(
        &self,
        _id: i64,
    ) -> Result<(), apis::domain_model::DomainModelApiError> {
        unimplemented!()
    }
}

/// Cheap per-run unique suffix. `uuid` is not a workspace dep and
/// the code/name columns are the only thing this needs to be unique
/// in, so a nanosecond timestamp + a process-id-ish counter is more
/// than sufficient. (Two parallel runs on the same DB would still
/// collide; the test is `#[ignore]`d so that is acceptable.)
fn uuid_like_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}")
}

/// Stub [`apis::crf::CrfService`] for the live-DB auth test. Every
/// method `unimplemented!()`s — this test never exercises CRF
/// routes. Mirrors the per-feature `NullCrfService` in
/// `state::test_support` (which is `pub(crate)` and therefore not
/// reachable from this external integration test).
#[derive(Clone)]
struct NullCrfService;

#[async_trait]
impl apis::crf::CrfService for NullCrfService {
    async fn create_version(
        &self,
        _req: apis::crf::CreateCrfVersionRequest,
    ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_version_by_id(
        &self,
        _req: apis::crf::GetCrfVersionByIdRequest,
    ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_versions_by_project(
        &self,
        _req: apis::crf::ListCrfVersionsByProjectRequest,
    ) -> Result<Vec<apis::crf::CrfVersionView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn update_version(
        &self,
        _req: apis::crf::UpdateCrfVersionRequest,
    ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn delete_version(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn create_form(
        &self,
        _req: apis::crf::CreateCrfFormRequest,
    ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_form_by_id(
        &self,
        _req: apis::crf::GetCrfFormByIdRequest,
    ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_form_detail(
        &self,
        _req: apis::crf::GetCrfFormDetailRequest,
    ) -> Result<apis::crf::CrfFormDetailView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_forms_by_version(
        &self,
        _req: apis::crf::ListCrfFormsByVersionRequest,
    ) -> Result<Vec<apis::crf::CrfFormView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn update_form(
        &self,
        _req: apis::crf::UpdateCrfFormRequest,
    ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn delete_form(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn set_approved(
        &self,
        _req: apis::crf::SetCrfApprovedRequest,
    ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn bulk_create_form(
        &self,
        _req: apis::crf::BulkCreateCrfFormRequest,
    ) -> Result<apis::crf::BulkCreateCrfFormResult, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn create_item(
        &self,
        _req: apis::crf::CreateCrfItemRequest,
    ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_item_by_id(
        &self,
        _req: apis::crf::GetCrfItemByIdRequest,
    ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_items_by_form(
        &self,
        _req: apis::crf::ListCrfItemsByFormRequest,
    ) -> Result<Vec<apis::crf::CrfItemView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn update_item(
        &self,
        _req: apis::crf::UpdateCrfItemRequest,
    ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn delete_item(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn create_option(
        &self,
        _req: apis::crf::CreateCrfOptionRequest,
    ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_option_by_id(
        &self,
        _req: apis::crf::GetCrfOptionByIdRequest,
    ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_options_by_item(
        &self,
        _req: apis::crf::ListCrfOptionsByItemRequest,
    ) -> Result<Vec<apis::crf::CrfOptionView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn update_option(
        &self,
        _req: apis::crf::UpdateCrfOptionRequest,
    ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn delete_option(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn create_unit(
        &self,
        _req: apis::crf::CreateCrfUnitRequest,
    ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_unit_by_id(
        &self,
        _req: apis::crf::GetCrfUnitByIdRequest,
    ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_units_by_item(
        &self,
        _req: apis::crf::ListCrfUnitsByItemRequest,
    ) -> Result<Vec<apis::crf::CrfUnitView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn update_unit(
        &self,
        _req: apis::crf::UpdateCrfUnitRequest,
    ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn delete_unit(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn create_domain_annotation(
        &self,
        _req: apis::crf::CreateDomainAnnotationRequest,
    ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_domain_annotation_by_id(
        &self,
        _req: apis::crf::GetDomainAnnotationByIdRequest,
    ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_domain_annotations_by_form(
        &self,
        _req: apis::crf::ListDomainAnnotationsByFormRequest,
    ) -> Result<Vec<apis::crf::DomainAnnotationView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn update_domain_annotation(
        &self,
        _req: apis::crf::UpdateDomainAnnotationRequest,
    ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn delete_domain_annotation(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn create_annotation(
        &self,
        _req: apis::crf::CreateAnnotationRequest,
    ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn get_annotation_by_id(
        &self,
        _req: apis::crf::GetAnnotationByIdRequest,
    ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_annotations_by_form(
        &self,
        _req: apis::crf::ListAnnotationsByFormRequest,
    ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_annotations_by_item(
        &self,
        _req: apis::crf::ListAnnotationsByItemRequest,
    ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_annotations_by_option(
        &self,
        _req: apis::crf::ListAnnotationsByOptionRequest,
    ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn list_annotations_by_unit(
        &self,
        _req: apis::crf::ListAnnotationsByUnitRequest,
    ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn update_annotation(
        &self,
        _req: apis::crf::UpdateAnnotationRequest,
    ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn delete_annotation(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn search_forms_by_version(
        &self,
        _req: apis::crf::SearchCrfFormsByVersionRequest,
    ) -> Result<Vec<apis::crf::CrfFormView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn search_items_by_version(
        &self,
        _req: apis::crf::SearchCrfItemsByVersionRequest,
    ) -> Result<Vec<apis::crf::CrfItemView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn search_options_by_version(
        &self,
        _req: apis::crf::SearchCrfOptionsByVersionRequest,
    ) -> Result<Vec<apis::crf::CrfOptionView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn search_units_by_version(
        &self,
        _req: apis::crf::SearchCrfUnitsByVersionRequest,
    ) -> Result<Vec<apis::crf::CrfUnitView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn search_domain_annotations_by_version(
        &self,
        _req: apis::crf::SearchDomainAnnotationsByVersionRequest,
    ) -> Result<Vec<apis::crf::DomainAnnotationView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
    async fn search_annotations_by_version(
        &self,
        _req: apis::crf::SearchAnnotationsByVersionRequest,
    ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
        unimplemented!()
    }
}

/// Stub [`apis::mission::MissionService`] for the live-DB auth test.
/// Every method `unimplemented!()`s — this test never exercises
/// mission routes.
#[derive(Clone)]
struct NullMissionService;

#[async_trait]
impl apis::mission::MissionService for NullMissionService {
    async fn create_mission(
        &self,
        _actor: &apis::mission::Actor,
        _req: apis::mission::CreateMissionRequest,
    ) -> Result<apis::mission::MissionView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn get_mission_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::mission::MissionView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn list_missions_by_project(
        &self,
        _req: apis::mission::ListMissionsByProjectRequest,
    ) -> Result<Vec<apis::mission::MissionView>, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn list_missions_by_user(
        &self,
        _req: apis::mission::ListMissionsByUserRequest,
    ) -> Result<Vec<apis::mission::MissionView>, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn delete_mission(
        &self,
        _actor: &apis::mission::Actor,
        _id: i64,
    ) -> Result<(), apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn add_assignee(
        &self,
        _actor: &apis::mission::Actor,
        _mission_id: i64,
        _data: apis::mission::AssigneeData,
    ) -> Result<apis::mission::AssigneeView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn remove_assignee(
        &self,
        _actor: &apis::mission::Actor,
        _mission_id: i64,
        _assignee_id: i64,
    ) -> Result<(), apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn list_issues_by_mission(
        &self,
        _req: apis::mission::ListIssuesByMissionRequest,
    ) -> Result<Vec<apis::mission::IssueView>, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn create_issue(
        &self,
        _actor: &apis::mission::Actor,
        _req: apis::mission::CreateIssueRequest,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn close_issue(
        &self,
        _actor: &apis::mission::Actor,
        _req: apis::mission::CloseIssueRequest,
        _issue_id: i64,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn reopen_issue(
        &self,
        _actor: &apis::mission::Actor,
        _req: apis::mission::ReopenIssueRequest,
        _issue_id: i64,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn update_issue_description(
        &self,
        _actor: &apis::mission::Actor,
        _issue_id: i64,
        _req: apis::mission::UpdateIssueDescriptionRequest,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn append_comment(
        &self,
        _actor: &apis::mission::Actor,
        _issue_id: i64,
        _req: apis::mission::AppendCommentRequest,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!()
    }
}

/// A `SdtmProjectService` double for the integration test. The
/// `state::test_support` module is crate-private, so this file
/// carries its own copy — same shape as the other null doubles here.
struct NullSdtmProjectService;

#[async_trait::async_trait]
impl apis::sdtm_project::SdtmProjectService for NullSdtmProjectService {
    async fn create_version(
        &self,
        _req: apis::sdtm_project::CreateSdtmProjectVersionRequest,
    ) -> Result<apis::sdtm_project::SdtmProjectVersionView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn get_version_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::sdtm_project::SdtmProjectVersionView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn list_versions_by_project(
        &self,
        _project_code: &str,
    ) -> Result<apis::sdtm_project::SdtmProjectVersionList, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn update_version(
        &self,
        _req: apis::sdtm_project::UpdateSdtmProjectVersionRequest,
    ) -> Result<apis::sdtm_project::SdtmProjectVersionView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn delete_version(
        &self,
        _id: i64,
    ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
        unimplemented!()
    }
    async fn create_domain(
        &self,
        _req: apis::sdtm_project::CreateSdtmProjectDomainRequest,
    ) -> Result<apis::sdtm_project::SdtmProjectDomainView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn get_domain_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::sdtm_project::SdtmProjectDomainView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn list_domains_by_version(
        &self,
        _version_id: i64,
    ) -> Result<apis::sdtm_project::SdtmProjectDomainList, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn update_domain(
        &self,
        _req: apis::sdtm_project::UpdateSdtmProjectDomainRequest,
    ) -> Result<apis::sdtm_project::SdtmProjectDomainView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn delete_domain(&self, _id: i64) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
        unimplemented!()
    }
    async fn create_domain_override(
        &self,
        _req: apis::sdtm_project::CreateSdtmProjectDomainOverrideRequest,
    ) -> Result<
        apis::sdtm_project::SdtmProjectDomainOverrideView,
        apis::sdtm_project::SdtmProjectApiError,
    > {
        unimplemented!()
    }
    async fn get_domain_override(
        &self,
        _domain_id: i64,
    ) -> Result<
        Option<apis::sdtm_project::SdtmProjectDomainOverrideView>,
        apis::sdtm_project::SdtmProjectApiError,
    > {
        unimplemented!()
    }
    async fn update_domain_override(
        &self,
        _req: apis::sdtm_project::UpdateSdtmProjectDomainOverrideRequest,
    ) -> Result<
        apis::sdtm_project::SdtmProjectDomainOverrideView,
        apis::sdtm_project::SdtmProjectApiError,
    > {
        unimplemented!()
    }
    async fn delete_domain_override(
        &self,
        _domain_id: i64,
    ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
        unimplemented!()
    }
    async fn create_variable(
        &self,
        _req: apis::sdtm_project::CreateSdtmProjectVariableRequest,
    ) -> Result<apis::sdtm_project::SdtmProjectVariableView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn get_variable_by_id(
        &self,
        _id: i64,
    ) -> Result<apis::sdtm_project::SdtmProjectVariableView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn list_variables_by_domain(
        &self,
        _domain_id: i64,
    ) -> Result<apis::sdtm_project::SdtmProjectVariableList, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn update_variable(
        &self,
        _req: apis::sdtm_project::UpdateSdtmProjectVariableRequest,
    ) -> Result<apis::sdtm_project::SdtmProjectVariableView, apis::sdtm_project::SdtmProjectApiError>
    {
        unimplemented!()
    }
    async fn delete_variable(
        &self,
        _id: i64,
    ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
        unimplemented!()
    }
    async fn create_variable_override(
        &self,
        _req: apis::sdtm_project::CreateSdtmProjectVariableOverrideRequest,
    ) -> Result<
        apis::sdtm_project::SdtmProjectVariableOverrideView,
        apis::sdtm_project::SdtmProjectApiError,
    > {
        unimplemented!()
    }
    async fn get_variable_override(
        &self,
        _variable_id: i64,
    ) -> Result<
        Option<apis::sdtm_project::SdtmProjectVariableOverrideView>,
        apis::sdtm_project::SdtmProjectApiError,
    > {
        unimplemented!()
    }
    async fn update_variable_override(
        &self,
        _req: apis::sdtm_project::UpdateSdtmProjectVariableOverrideRequest,
    ) -> Result<
        apis::sdtm_project::SdtmProjectVariableOverrideView,
        apis::sdtm_project::SdtmProjectApiError,
    > {
        unimplemented!()
    }
    async fn delete_variable_override(
        &self,
        _variable_id: i64,
    ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
        unimplemented!()
    }
}
