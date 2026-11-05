//! Server bootstrap.
//!
//! `run` is the single entry point called by `main.rs`. It:
//! 1. Loads [`Config`] from the environment.
//! 2. Builds a Postgres pool + in-memory token-version cache.
//! 3. Wires `AuthUsecase` → `AuthServiceImpl` → `Arc<dyn AuthService>`.
//! 4. Binds the HTTP listener and serves the router returned by
//!    [`transport::http::router`].

use std::sync::Arc;
use std::time::Duration;

use apis::auth::AuthService;
use apis::crf::CrfService;
use apis::domain_model::DomainModelService;
use apis::mission::MissionService;
use apis::sdtm_project::SdtmProjectService;
use apis::structures::StructureService;
use apis::terminology::TerminologyService;
use apis::user::UserService;
use auth::{
    AuthServiceImpl, AuthUsecase, AuthUsecaseConfig, DomainIdentityRepo, InMemoryTokenVersionCache,
    TokenVersionCache, UserCredentialsRepo,
};
use auth::{UserService as AuthUserService, UserServiceImpl as AuthUserServiceImpl};
use crf::{
    AnnotationRepoPg, CrfBulkFormRepoPg, CrfFormRepoPg, CrfItemRepoPg, CrfOptionRepoPg,
    CrfServiceImpl, CrfUnitRepoPg, CrfVersionRepoPg, DomainAnnotationRepoPg, ProjectLookupImpl,
};
use domain_model::{
    DomainModelServiceImpl, DomainModelUsecase, DomainModelUsecaseConfig, SdtmDomainRepoPg,
    SdtmVariableRepoPg, SdtmVersionRepoPg,
};
use mission::{
    AssigneeRepo, IssueRepo, MissionRepo, MissionServiceImpl,
    ProjectLookupImpl as MissionProjectLookupImpl, UserLookupImpl as MissionUserLookupImpl,
};
use sdtm_project::{
    ProjectLookupImpl as SdtmProjectLookupImpl, SdtmProjectDomainOverrideRepoPg,
    SdtmProjectDomainRepoPg, SdtmProjectServiceImpl, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
    SdtmProjectVariableOverrideRepoPg, SdtmProjectVariableRepoPg, SdtmProjectVersionRepoPg,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use structures::{
    NodeRepoPg, ProjectLookupImpl as StructuresProjectLookupImpl, StructureRepoPg,
    StructureServiceImpl, StructureUsecase, StructureUsecaseConfig,
};
use terminology::{
    CodeItemRepo, CodeListRepo, TerminologyServiceImpl, TerminologyUsecase,
    TerminologyUsecaseConfig, TerminologyVersionRepo,
};
use tokio::net::TcpListener;
use user::{UserRepo, UserServiceImpl, UserUsecase};

use crate::config::Config;
use crate::state::AppState;
use crate::transport;

/// Bootstrap the aegis-server HTTP service and run until interrupted.
///
/// `config` is normally built via [`Config::from_env`] in `main.rs`.
/// The function only returns on bind / serve failure or shutdown.
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let log_dir = std::env::var("AEGIS_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    let _log_guard = logging_utils::init_tracing(
        &logging_utils::LoggingConfig {
            log_dir: std::path::PathBuf::from(log_dir.clone()),
            file_name_prefix: "aegis-server.log".into(),
        },
        // The server is the sink for client logs, not a client: it
        // has nothing of its own to submit.
        Vec::new(),
    )
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        format!("init_tracing: {e}").into()
    })?;

    let pool = build_pool(&config.database_url).await?;
    let cache: Arc<dyn TokenVersionCache> = Arc::new(InMemoryTokenVersionCache::new());

    let auth = build_auth_service(&config, pool.clone(), cache)?;
    let user = build_user_service(pool.clone());
    let project = build_project_service(pool.clone(), user.clone());
    let terminology = build_terminology_service(pool.clone());
    let domain_model = build_domain_model_service(pool.clone());
    let mission = build_mission_service(pool.clone(), project.clone(), user.clone());
    let sdtm_project = build_sdtm_project_service(pool.clone(), project.clone());
    let structures = build_structure_service(pool.clone(), project.clone());
    let crf = build_crf_service(pool, project.clone());

    // Build the client-log ingestor. Reuses the same `log_dir` as the
    // server's tracing logs; per-source separation comes from the
    // file-name prefix (`aegis-server.log` vs the configured ingestor
    // prefix, default `aegis-desktop.log`).
    let log_ingest_cfg = logging_utils::LogIngestorConfig::builder()
        .log_dir(std::path::PathBuf::from(&log_dir))
        .file_name_prefix(config.log_ingest_prefix.clone())
        .build();
    let log_ingestor = Arc::new(logging_utils::LogIngestor::new(log_ingest_cfg).map_err(
        |e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("log_ingestor init: {e}").into()
        },
    )?);

    let state = AppState {
        auth,
        user,
        project,
        terminology,
        domain_model,
        mission,
        crf,
        sdtm_project,
        structures,
        log_ingestor,
    };
    let app = transport::http::router(state);

    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "aegis-server listening");

    let shutdown = shutdown_signal();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM and then resolve.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Build a Postgres connection pool with a sensible timeout /
/// size policy.
async fn build_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Wrap the auth backend (usecase + cache + repos) in an
/// `AuthServiceImpl` so the HTTP layer can consume it through
/// `Arc<dyn AuthService>`.
fn build_auth_service(
    config: &Config,
    pool: PgPool,
    cache: Arc<dyn TokenVersionCache>,
) -> Result<Arc<dyn AuthService>, Box<dyn std::error::Error + Send + Sync>> {
    let credentials = UserCredentialsRepo::new(pool.clone());
    let identities = DomainIdentityRepo::new(pool.clone());

    // The auth usecase needs an `auth::UserService` (a domain port
    // from the auth crate, not the apis port) to validate codes
    // against the user table. We bridge the apis::user::UserService
    // (built from the user Postgres repo) into the auth domain's
    // port via `AuthUserServiceImpl`.
    let apis_user: Arc<dyn UserService> = build_user_service(pool);
    let auth_user: Arc<dyn AuthUserService> = Arc::new(AuthUserServiceImpl::new(apis_user));

    let usecase = AuthUsecase::new(AuthUsecaseConfig {
        credentials,
        identities,
        user_service: auth_user,
        cache,
        signing_key: config.signing_key.clone(),
        access_ttl: config.access_ttl,
        refresh_ttl: config.refresh_ttl,
        allow_domains: config.allow_domains.clone(),
    });
    Ok(Arc::new(AuthServiceImpl::new(usecase)))
}

/// Wrap the user-crate's `UserServiceImpl` in `Arc<dyn UserService>`
/// (the apis port) so the HTTP layer can use it.
fn build_user_service(pool: PgPool) -> Arc<dyn UserService> {
    let repo = UserRepo::new(pool);
    let usecase = UserUsecase::new(repo);
    let service = UserServiceImpl::new(usecase);
    Arc::new(service)
}

/// Wire the project crate's adapters into the apis `ProjectService`
/// trait object. The same shared `UserService` that drives user CRUD
/// and auth also hydrates project membership, so it is cloned into
/// the project-side adapter rather than rebuilt.
fn build_project_service(
    pool: PgPool,
    user: Arc<dyn apis::user::UserService>,
) -> Arc<dyn apis::project::ProjectService> {
    let project_repo = project::ProjectRepo::new(pool.clone());
    let branch_repo = project::ProjectBranchRepo::new(pool);
    let users = project::UserServiceImpl::new(user);
    let usecase = project::ProjectUsecase::new(project::ProjectUsecaseConfig {
        project_repo,
        users,
    });
    let branch_usecase = project::ProjectBranchUsecase::new(branch_repo);
    Arc::new(project::ProjectServiceImpl::new(usecase, branch_usecase))
}

/// Wire the terminology crate's adapters into the apis
/// `TerminologyService` trait object. The three Postgres repos
/// share the same pool as auth / user / project; no second pool
/// or env var is introduced.
fn build_terminology_service(pool: PgPool) -> Arc<dyn TerminologyService> {
    let version_repo = TerminologyVersionRepo::new(pool.clone());
    let code_list_repo = CodeListRepo::new(pool.clone());
    let code_item_repo = CodeItemRepo::new(pool);
    let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
        version_repo,
        code_list_repo,
        code_item_repo,
    });
    Arc::new(TerminologyServiceImpl::new(usecase))
}

/// Wire the domain-model crate's adapters into the apis
/// `DomainModelService` trait object. The three Postgres repos
/// share the same pool as auth / user / project / terminology;
/// no second pool or env var is introduced.
fn build_domain_model_service(pool: PgPool) -> Arc<dyn DomainModelService> {
    let version_repo = SdtmVersionRepoPg::new(pool.clone());
    let domain_repo = SdtmDomainRepoPg::new(pool.clone());
    let variable_repo = SdtmVariableRepoPg::new(pool);
    let usecase = DomainModelUsecase::new(DomainModelUsecaseConfig {
        version_repo,
        domain_repo,
        variable_repo,
    });
    Arc::new(DomainModelServiceImpl::new(usecase))
}

/// Build the directory-structure service.
///
/// The `project` service is already built in `run`, so the
/// project-existence check reuses the canonical `projects` table.
fn build_structure_service(
    pool: PgPool,
    project: Arc<dyn apis::project::ProjectService>,
) -> Arc<dyn StructureService> {
    let structures = StructureRepoPg::new(pool.clone());
    let nodes = NodeRepoPg::new(pool);
    let projects = Arc::new(StructuresProjectLookupImpl::new(project));
    let usecase = StructureUsecase::new(StructureUsecaseConfig {
        structures,
        nodes,
        projects,
    });
    Arc::new(StructureServiceImpl::new(usecase))
}

/// Wire the crf crate's adapters into the apis `CrfService` trait
/// object. The seven Postgres repos share the same pool as the rest
/// of the server; the project lookup is bridged off the shared
/// `ProjectService` so foreign-key existence checks against
/// `crf_versions` reuse the canonical project table.
fn build_crf_service(
    pool: PgPool,
    project: Arc<dyn apis::project::ProjectService>,
) -> Arc<dyn CrfService> {
    let version_repo = CrfVersionRepoPg::new(pool.clone());
    let form_repo = CrfFormRepoPg::new(pool.clone());
    let item_repo = CrfItemRepoPg::new(pool.clone());
    let option_repo = CrfOptionRepoPg::new(pool.clone());
    let unit_repo = CrfUnitRepoPg::new(pool.clone());
    let domain_annotation_repo = DomainAnnotationRepoPg::new(pool.clone());
    let annotation_repo = AnnotationRepoPg::new(pool.clone());
    let bulk_form_repo = CrfBulkFormRepoPg::new(pool.clone());
    let projects = Arc::new(ProjectLookupImpl::new(project));
    Arc::new(CrfServiceImpl::from_repos(
        version_repo,
        form_repo,
        item_repo,
        option_repo,
        unit_repo,
        domain_annotation_repo,
        annotation_repo,
        projects,
        Arc::new(bulk_form_repo),
    ))
}

/// Wire the sdtm-project crate's adapters into the apis
/// `SdtmProjectService` trait object. The five Postgres repos share
/// the same pool as the rest of the server; the project lookup is
/// bridged off the shared `ProjectService` so the
/// project-existence check reuses the canonical project table.
fn build_sdtm_project_service(
    pool: PgPool,
    project: Arc<dyn apis::project::ProjectService>,
) -> Arc<dyn SdtmProjectService> {
    let version_repo = SdtmProjectVersionRepoPg::new(pool.clone());
    let domain_repo = SdtmProjectDomainRepoPg::new(pool.clone());
    let domain_override_repo = SdtmProjectDomainOverrideRepoPg::new(pool.clone());
    let variable_repo = SdtmProjectVariableRepoPg::new(pool.clone());
    let variable_override_repo = SdtmProjectVariableOverrideRepoPg::new(pool);
    let projects = Arc::new(SdtmProjectLookupImpl::new(project));
    let usecase = SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
        version_repo,
        domain_repo,
        domain_override_repo,
        variable_repo,
        variable_override_repo,
        projects,
    });
    Arc::new(SdtmProjectServiceImpl::new(usecase))
}

/// Wire the mission crate's adapters into the apis `MissionService`
/// trait object. Two Postgres repos share the same pool as the rest
/// of the server; the project + user lookups are bridged off the
/// shared `ProjectService` / `UserService` so leader-membership
/// checks reuse the canonical tables and leader authorization stays
/// a single source of truth.
fn build_mission_service(
    pool: PgPool,
    project: Arc<dyn apis::project::ProjectService>,
    user: Arc<dyn UserService>,
) -> Arc<dyn MissionService> {
    let mission_repo = Arc::new(MissionRepo::new(pool.clone()));
    let assignee_repo = Arc::new(AssigneeRepo::new(pool.clone()));
    let issue_repo = Arc::new(IssueRepo::new(pool));
    let projects = Arc::new(MissionProjectLookupImpl::new(project));
    let users = Arc::new(MissionUserLookupImpl::new(user));
    Arc::new(MissionServiceImpl::from_repos(
        mission_repo,
        assignee_repo,
        projects,
        users,
        issue_repo,
    ))
}
