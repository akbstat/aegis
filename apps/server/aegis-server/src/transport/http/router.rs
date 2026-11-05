//! Top-level HTTP router.
//!
//! Layout:
//! - `/api/auth/login`                  auth flows
//! - `/api/auth/login-domain`
//! - `/api/auth/refresh`
//! - `/api/auth/logout`
//! - `/api/auth/user-credential`        user-credential (self-service: PATCH only — creation is out of band)
//! - `/api/user`                         user CRUD
//! - `/api/user/{code}`
//! - `/api/terminology/*`                terminology CRUD
//! - `/api/domain-model/*`               SDTM domain model CRUD
//! - `/api/crf/*`                        Case Report Form CRUD
//! - `/api/mission/*`                    Mission CRUD (leader-only writes)
//! - `/api/sdtm-project/*`               per-project SDTM model tree CRUD
//! - `/api/structures/*`                  directory-structure CRUD
//! - `/healthz`                         liveness probe
//! - `/swagger-ui/`                      swagger-ui HTML
//! - `/swagger-ui/{*rest}`               swagger-ui assets
//! - `/api-docs/openapi.json`            OpenAPI v3 JSON document
//!
//! All `/api/*` routes are also wrapped in a `TraceLayer` so every
//! request emits a tracing span.

use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use logging_utils::TraceIdGenerator;

use crate::state::AppState;
use crate::transport::http::auth;
use crate::transport::http::crf::router as crf_router;
use crate::transport::http::domain_model::router as domain_model_router;
use crate::transport::http::healthz;
use crate::transport::http::log_ingest::router as log_ingest_router;
use crate::transport::http::mission::router as mission_router;
use crate::transport::http::openapi::ApiDoc;
use crate::transport::http::project::router as project_router;
use crate::transport::http::sdtm_project::router as sdtm_project_router;
use crate::transport::http::structures::router as structures_router;
use crate::transport::http::terminology::router as terminology_router;
use crate::transport::http::trace_id::TraceIdMakeSpan;
use crate::transport::http::user;

/// Build the full HTTP router with `state` attached.
///
/// The return type is `axum::Router` (the consumer-side type after
/// `split_for_parts()`). The `OpenApiRouter` is composed internally
/// so the per-handler `#[utoipa::path]` annotations are auto-
/// collected into the OpenAPI document; the swagger-ui is then
/// merged on top, and the resulting `Router` is wrapped in a
/// `TraceLayer` for tracing.
pub fn router(state: AppState) -> axum::Router {
    let project_routes = project_router::router();
    let terminology_routes = terminology_router::router();
    let domain_model_routes = domain_model_router::router();
    let crf_routes = crf_router::router();
    let mission_routes = mission_router::router();
    let sdtm_project_routes = sdtm_project_router::router();
    let structures_routes = structures_router::router();
    let api_routers = OpenApiRouter::new()
        .nest("/auth", auth::router())
        .nest("/user", user::router())
        .nest("/project", project_routes)
        .nest("/terminology", terminology_routes)
        .nest("/domain-model", domain_model_routes)
        .nest("/crf", crf_routes)
        .nest("/mission", mission_routes)
        .nest("/sdtm-project", sdtm_project_routes)
        .nest("/structures", structures_routes)
        .nest("/log-ingest", log_ingest_router::router());

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", api_routers)
        .nest("/healthz", healthz::router())
        .with_state(state)
        .split_for_parts();

    // `SwaggerUi::new("/swagger-ui").url(...)` returns `SwaggerUi`,
    // which has `From<SwaggerUi> for Router<S>` so we can convert
    // it into a `Router<()>`. `OpenApiRouter::merge` then takes
    // any `Router<S>` (the swagger-ui uses `S = ()` because its
    // handlers do not extract state).
    router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(TraceIdMakeSpan::new(TraceIdGenerator::new(None))),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    use apis::auth::{
        AuthApiError, AuthClaims as ApiAuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };

    #[derive(Clone)]
    struct MockAuth;

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _req: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            Ok(TokenPair {
                access_token: "ACCESS".into(),
                refresh_token: "REFRESH".into(),
            })
        }
        async fn login_with_domain_user_info(
            &self,
            _req: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> {
            Ok(TokenPair {
                access_token: "ACCESS".into(),
                refresh_token: "REFRESH".into(),
            })
        }
        async fn verify(&self, _req: VerifyRequest) -> Result<ApiAuthClaims, AuthApiError> {
            Ok(ApiAuthClaims {
                code: "u1".into(),
                role: apis::user::Role::Admin,
                token_version: 0,
            })
        }
        async fn refresh(&self, _req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
            Ok(RefreshResponse {
                access_token: "NEW".into(),
            })
        }
        async fn find_user_credential_by_code(
            &self,
            _code: &str,
        ) -> Result<UserCredentialView, AuthApiError> {
            Ok(UserCredentialView {
                user_code: "u1".into(),
                password_hash: "argon2id$v=19$m=...$...".into(),
                token_version: 0,
            })
        }
        async fn create_user_credential(
            &self,
            _req: CreateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn update_user_credential(
            &self,
            _req: UpdateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn remove_user_credential(
            &self,
            _code: &str,
        ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
            unimplemented!()
        }
        async fn logout(&self, _req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
            Ok(LogoutResponse::default())
        }
        async fn register_user(
            &self,
            req: RegisterUserRequest,
        ) -> Result<RegisterUserResponse, AuthApiError> {
            // Router-level integration tests don't exercise the
            // canonical registration flow — they only assert that the
            // route is mounted, the body is parsed, and the handler
            // reaches the `AuthService` port. Echo a synthetic 201
            // view built from the request body.
            Ok(RegisterUserResponse {
                user_code: req.user_code,
                user_name: req.user_name,
                role: apis::user::Role::General,
                active: false,
                domain_name: req.domain_name,
                hostname: req.hostname,
                sid: req.sid,
            })
        }
    }

    #[derive(Clone)]
    struct NullProjectService;

    #[async_trait]
    impl apis::project::ProjectService for NullProjectService {
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

    /// Null terminology service for tests that don't exercise the
    /// terminology surface. Re-exported from `crate::state::test_support`
    /// so every test module can share one implementation.

    #[derive(Clone)]
    struct NullUserService;

    #[async_trait]
    impl apis::user::UserService for NullUserService {
        async fn create(
            &self,
            _req: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn get_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn get_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn update(
            &self,
            _req: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
    }

    /// User mock for integration tests. Returns a fixed one-user
    /// list from `list()` and a fixed view from `get_by_code` /
    /// `create` / `update`; any other method panics. This is
    /// deliberately minimal — the per-handler tests in
    /// `user::handlers` cover the translation surface in detail.
    #[derive(Clone)]
    struct StubUserService;

    #[async_trait]
    impl apis::user::UserService for StubUserService {
        async fn create(
            &self,
            _req: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            Ok(sample_user_view(1, "u1"))
        }
        async fn get_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn get_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            Ok(sample_user_view(1, "u1"))
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            Ok(vec![sample_user_view(1, "u1")])
        }
        async fn update(
            &self,
            _req: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            Ok(sample_user_view(1, "u1"))
        }
    }

    fn sample_user_view(id: i32, code: &str) -> apis::user::UserView {
        apis::user::UserView {
            id,
            code: code.to_string(),
            name: format!("User {code}"),
            role: apis::user::Role::Admin,
            active: true,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn sample_project_view(id: i32, code: &str) -> apis::project::ProjectView {
        apis::project::ProjectView {
            id,
            code: code.to_string(),
            description: "sample".to_string(),
            members: apis::project::ProjectMemberView::default(),
            unblind_members: apis::project::ProjectMemberView::default(),
            configurations: apis::project::ProjectConfigurationView::default(),
            active: true,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn test_state() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(crate::state::test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(crate::state::test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(crate::state::test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(crate::state::test_support::NullCrfService)
                as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(crate::state::test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    /// State builder for user-integration tests: same `MockAuth`
    /// (so verify returns OK), but a `StubUserService` that returns
    /// positive responses for the user routes.
    fn test_state_with_user() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(StubUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(crate::state::test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(crate::state::test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(crate::state::test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(crate::state::test_support::NullCrfService)
                as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(crate::state::test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    /// State builder for project-integration tests: a `StubProjectService`
    /// that returns positive responses for the project routes.
    fn test_state_with_project() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(StubProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(crate::state::test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(crate::state::test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(crate::state::test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(crate::state::test_support::NullCrfService)
                as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(crate::state::test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    #[derive(Clone)]
    struct StubProjectService;

    #[async_trait]
    impl apis::project::ProjectService for StubProjectService {
        async fn create_project(
            &self,
            _req: apis::project::CreateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Ok(sample_project_view(1, "pr1"))
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
            Ok(sample_project_view(1, "pr1"))
        }
        async fn list_projects(
            &self,
        ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
            Ok(vec![sample_project_view(1, "pr1")])
        }
        async fn update_project(
            &self,
            _req: apis::project::UpdateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Ok(sample_project_view(1, "pr1"))
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

    #[tokio::test]
    async fn healthz_returns_200() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
    }

    #[tokio::test]
    async fn login_route_returns_200() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"u1","password":"p"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::NOT_FOUND);
    }

    /// `GET /healthz` with a valid `X-Trace-ID` header still
    /// returns 200 OK — proves the new `TraceIdMakeSpan` layer
    /// does not break the existing path. The value-level
    /// extraction is covered by the unit tests on
    /// `extract_trace_id` / `is_valid_trace_id` in
    /// `transport::http::trace_id`.
    #[tokio::test]
    async fn request_with_trace_id_header_succeeds() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/healthz")
                    .header("x-trace-id", "C-desktop-01ABCDEF01234567890ABCDEF")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
    }

    #[tokio::test]
    async fn swagger_ui_root_returns_200() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/swagger-ui/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Sibling redirects / assets may return 200 or 304; the
        // important assertion is that the route is registered.
        assert!(
            response.status().is_success() || response.status().is_redirection(),
            "expected 2xx or 3xx, got {}",
            response.status(),
        );
    }

    /// The `/api/sdtm-project` namespace must advertise every route
    /// with the BearerAuth requirement. No route advertises a 403:
    /// the namespace has no role gate, so a 403 can never be produced.
    ///
    /// This is a separate test from
    /// `openapi_json_returns_200_with_valid_doc` so that a failure
    /// here cannot be masked by that test's own pre-existing
    /// failures.
    #[tokio::test]
    async fn openapi_json_advertises_the_sdtm_project_namespace() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();

        for (method, path) in [
            ("post", "/api/sdtm-project/versions"),
            ("get", "/api/sdtm-project/versions"),
            ("get", "/api/sdtm-project/versions/{id}"),
            ("put", "/api/sdtm-project/versions/{id}"),
            ("delete", "/api/sdtm-project/versions/{id}"),
            ("post", "/api/sdtm-project/domains"),
            ("get", "/api/sdtm-project/domains/{id}"),
            ("get", "/api/sdtm-project/versions/{version_id}/domains"),
            ("put", "/api/sdtm-project/domains/{id}"),
            ("delete", "/api/sdtm-project/domains/{id}"),
            ("post", "/api/sdtm-project/domains/{domain_id}/override"),
            ("get", "/api/sdtm-project/domains/{domain_id}/override"),
            ("put", "/api/sdtm-project/domains/{domain_id}/override"),
            ("delete", "/api/sdtm-project/domains/{domain_id}/override"),
            ("post", "/api/sdtm-project/variables"),
            ("get", "/api/sdtm-project/variables/{id}"),
            ("get", "/api/sdtm-project/domains/{domain_id}/variables"),
            ("put", "/api/sdtm-project/variables/{id}"),
            ("delete", "/api/sdtm-project/variables/{id}"),
            ("post", "/api/sdtm-project/variables/{variable_id}/override"),
            ("get", "/api/sdtm-project/variables/{variable_id}/override"),
            ("put", "/api/sdtm-project/variables/{variable_id}/override"),
            (
                "delete",
                "/api/sdtm-project/variables/{variable_id}/override",
            ),
        ] {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
            let response_keys: Vec<&str> = op["responses"]
                .as_object()
                .expect("responses object")
                .keys()
                .map(|s| s.as_str())
                .collect();
            assert!(
                !response_keys.contains(&"403"),
                "{method} {path} must not advertise a 403 (got {response_keys:?})",
            );
        }

        // The two `GET …/override` routes answer with an envelope
        // carrying an `override` key, not a bare nullable override.
        // The OpenAPI document must say so, or a generated client
        // would read `null` where the server sends an object.
        for (path, schema) in [
            (
                "/api/sdtm-project/domains/{domain_id}/override",
                "SdtmProjectDomainOverrideResponse",
            ),
            (
                "/api/sdtm-project/variables/{variable_id}/override",
                "SdtmProjectVariableOverrideResponse",
            ),
        ] {
            let op = &doc["paths"][path]["get"];
            let schema_ref =
                op["responses"]["200"]["content"]["application/json"]["schema"]["$ref"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{path} GET has no $ref response schema"));
            assert_eq!(
                schema_ref,
                format!("#/components/schemas/{schema}"),
                "{path} GET must answer with the {schema} envelope",
            );
            let properties = &doc["components"]["schemas"][schema]["properties"];
            assert!(
                properties.get("override").is_some(),
                "{schema} must carry an `override` property: {properties}",
            );
        }
    }

    #[tokio::test]
    async fn openapi_json_advertises_the_structures_namespace() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Every route must be present and require BearerAuth.
        let routes: Vec<(&str, &str)> = vec![
            ("post", "/api/structures"),
            ("get", "/api/structures"),
            ("get", "/api/structures/{id}"),
            ("get", "/api/structures/{id}/meta"),
            ("put", "/api/structures/{id}"),
            ("delete", "/api/structures/{id}"),
            ("get", "/api/structures/{id}/tree"),
            ("get", "/api/structures/{id}/nodes"),
            ("post", "/api/structures/{id}/nodes"),
            ("put", "/api/structures/{id}/nodes/{node_id}"),
            ("delete", "/api/structures/{id}/nodes/{node_id}"),
        ];
        for (method, path) in &routes {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }

        // A write can target either a template (admin/root) or a
        // project structure (any authenticated user), so every write
        // route advertises a 403 — and no read route does.
        for (method, path) in &routes {
            let op = &doc["paths"][path][method];
            let response_keys: Vec<&str> = op["responses"]
                .as_object()
                .expect("responses object")
                .keys()
                .map(|s| s.as_str())
                .collect();
            let is_write = matches!(*method, "post" | "put" | "delete");
            if is_write {
                assert!(
                    response_keys.contains(&"403"),
                    "{method} {path} must advertise a 403 (got {response_keys:?})",
                );
            } else {
                assert!(
                    !response_keys.contains(&"403"),
                    "{method} {path} must not advertise a 403 (got {response_keys:?})",
                );
            }
        }
    }

    #[tokio::test]
    async fn openapi_json_returns_200_with_valid_doc() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 256 * 1024)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(doc["info"]["title"], "aegis-server API");
        assert!(doc["paths"]["/api/auth/login"].is_object());
        assert!(doc["paths"]["/api/auth/login-domain"].is_object());
        assert!(doc["paths"]["/api/auth/refresh"].is_object());
        assert!(doc["paths"]["/api/auth/logout"].is_object());
        assert!(doc["paths"]["/api/auth/user-credential"].is_object());
        assert!(doc["paths"]["/healthz"].is_object());
        // domain-model endpoints are auto-registered by the
        // OpenApiRouter nest; assert the surface is present.
        assert!(doc["paths"]["/api/domain-model/versions"].is_object());
        assert!(doc["paths"]["/api/domain-model/versions/{id}"].is_object());
        assert!(doc["paths"]["/api/domain-model/domains"].is_object());
        assert!(doc["paths"]["/api/domain-model/domains/{id}"].is_object());
        assert!(doc["paths"]["/api/domain-model/versions/{version_id}/domains"].is_object());
        assert!(doc["paths"]["/api/domain-model/variables"].is_object());
        assert!(doc["paths"]["/api/domain-model/variables/{id}"].is_object());
        assert!(doc["paths"]["/api/domain-model/domains/{domain_id}/variables"].is_object());

        // The Bearer security scheme must be registered so
        // refresh / logout advertise the BearerAuth requirement.
        let schemes = &doc["components"]["securitySchemes"];
        assert!(
            schemes["BearerAuth"].is_object(),
            "BearerAuth scheme missing"
        );
        assert_eq!(schemes["BearerAuth"]["type"], "http");
        assert_eq!(schemes["BearerAuth"]["scheme"], "bearer");
        assert_eq!(schemes["BearerAuth"]["bearerFormat"], "JWT");

        // refresh + logout must reference the security scheme;
        // login + login-domain + healthz must not.
        let refresh = &doc["paths"]["/api/auth/refresh"]["post"];
        assert_eq!(refresh["security"][0]["BearerAuth"], serde_json::json!([]));
        let logout = &doc["paths"]["/api/auth/logout"]["post"];
        assert_eq!(logout["security"][0]["BearerAuth"], serde_json::json!([]));
        assert!(doc["paths"]["/api/auth/login"]["post"]["security"].is_null());
        assert!(doc["paths"]["/api/auth/login-domain"]["post"]["security"].is_null());
        assert!(doc["paths"]["/healthz"]["get"]["security"].is_null());

        // /api/user namespace must advertise every CRUD verb with
        // the BearerAuth requirement — the gate is the whole point
        // of the router.
        for (method, path) in [
            ("post", "/api/user"),
            ("get", "/api/user"),
            ("get", "/api/user/{code}"),
            ("patch", "/api/user/{code}"),
        ] {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }

        // /api/project namespace must advertise every verb with the
        // BearerAuth requirement. Write operations (POST/PATCH) must
        // additionally advertise a 403 response, because non-admin
        // callers are rejected at the role guard.
        for (method, path) in [
            ("post", "/api/project"),
            ("get", "/api/project"),
            ("get", "/api/project/{code}"),
            ("patch", "/api/project/{code}"),
        ] {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }
        for (method, path) in [("post", "/api/project"), ("patch", "/api/project/{code}")] {
            let op = &doc["paths"][path][method];
            let response_keys: Vec<&str> = op["responses"]
                .as_object()
                .expect("responses object")
                .keys()
                .map(|s| s.as_str())
                .collect();
            assert!(
                response_keys.contains(&"403"),
                "{method} {path} must advertise a 403 response (got {response_keys:?})",
            );
        }

        // /api/terminology namespace must advertise every verb with
        // the BearerAuth requirement. Read routes (GET) call the
        // usecase without a role guard; write routes (POST / PATCH /
        // DELETE) additionally advertise a 403 response, because the
        // shared `require_admin_or_root` helper rejects general
        // callers before the usecase is invoked.
        let terminology_reads = [
            ("get", "/api/terminology/versions"),
            ("get", "/api/terminology/versions/{id}"),
            ("get", "/api/terminology/code-lists"),
            ("get", "/api/terminology/code-items"),
            ("get", "/api/terminology/code-items/by-version-and-code"),
        ];
        let terminology_writes = [
            ("post", "/api/terminology/versions"),
            ("patch", "/api/terminology/versions/{id}"),
            ("delete", "/api/terminology/versions/{id}"),
            ("post", "/api/terminology/code-lists"),
            ("patch", "/api/terminology/code-lists/{id}"),
            ("delete", "/api/terminology/code-lists/{id}"),
            ("post", "/api/terminology/code-items"),
            ("patch", "/api/terminology/code-items/{id}"),
            ("delete", "/api/terminology/code-items/{id}"),
        ];
        for (method, path) in terminology_reads.iter().chain(terminology_writes.iter()) {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi",);
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }
        for (method, path) in terminology_writes.iter() {
            let op = &doc["paths"][path][method];
            let response_keys: Vec<&str> = op["responses"]
                .as_object()
                .expect("responses object")
                .keys()
                .map(|s| s.as_str())
                .collect();
            assert!(
                response_keys.contains(&"403"),
                "{method} {path} must advertise a 403 response (got {response_keys:?})",
            );
        }

        // /api/auth/user-credential namespace advertises two verbs:
        // `PATCH` for self-service rotation and `POST` for the
        // administrator-only registration route. Both sit under
        // BearerAuth. `user_code` for PATCH is derived from the
        // token, so there is no `/{code}` path; the POST body
        // carries `user_code` explicitly.
        let patch_op = &doc["paths"]["/api/auth/user-credential"]["patch"];
        assert!(
            patch_op.is_object(),
            "missing patch /api/auth/user-credential in openapi"
        );
        assert_eq!(
            patch_op["security"][0]["BearerAuth"],
            serde_json::json!([]),
            "patch /api/auth/user-credential must require BearerAuth",
        );
        let post_op = &doc["paths"]["/api/auth/user-credential"]["post"];
        assert!(
            post_op.is_object(),
            "POST /api/auth/user-credential must be advertised for the \
             administrator registration route"
        );
        assert_eq!(
            post_op["security"][0]["BearerAuth"],
            serde_json::json!([]),
            "POST /api/auth/user-credential must require BearerAuth",
        );

        // /api/crf namespace advertises every verb with the
        // BearerAuth requirement. The CRF surface mirrors the
        // terminology role policy: read routes (GET) are open to any
        // authenticated caller, write routes (POST / PATCH / DELETE)
        // additionally advertise a 403 response because the shared
        // `require_admin_or_root` helper rejects general callers
        // before the usecase is invoked.
        let crf_reads = [
            ("get", "/api/crf/projects/{project_code}/versions"),
            ("get", "/api/crf/versions/{id}"),
            ("get", "/api/crf/versions/{version_id}/forms"),
            ("get", "/api/crf/versions/{version_id}/forms/search"),
            ("get", "/api/crf/forms/{id}"),
            ("get", "/api/crf/forms/{form_id}/items"),
            ("get", "/api/crf/versions/{version_id}/items/search"),
            ("get", "/api/crf/items/{id}"),
            ("get", "/api/crf/items/{item_id}/options"),
            ("get", "/api/crf/versions/{version_id}/options/search"),
            ("get", "/api/crf/options/{id}"),
            ("get", "/api/crf/items/{item_id}/units"),
            ("get", "/api/crf/versions/{version_id}/units/search"),
            ("get", "/api/crf/units/{id}"),
            ("get", "/api/crf/forms/{form_id}/domain-annotations"),
            (
                "get",
                "/api/crf/versions/{version_id}/domain-annotations/search",
            ),
            ("get", "/api/crf/domain-annotations/{id}"),
            ("get", "/api/crf/forms/{form_id}/annotations"),
            ("get", "/api/crf/items/{item_id}/annotations"),
            ("get", "/api/crf/options/{option_id}/annotations"),
            ("get", "/api/crf/units/{unit_id}/annotations"),
            ("get", "/api/crf/versions/{version_id}/annotations/search"),
            ("get", "/api/crf/annotations/{id}"),
        ];
        let crf_writes = [
            ("post", "/api/crf/projects/{project_code}/versions"),
            ("patch", "/api/crf/versions/{id}"),
            ("delete", "/api/crf/versions/{id}"),
            ("post", "/api/crf/versions/{version_id}/forms"),
            ("post", "/api/crf/versions/{version_id}/forms/bulk"),
            ("patch", "/api/crf/forms/{id}"),
            ("delete", "/api/crf/forms/{id}"),
            ("post", "/api/crf/forms/{form_id}/items"),
            ("patch", "/api/crf/items/{id}"),
            ("delete", "/api/crf/items/{id}"),
            ("post", "/api/crf/items/{item_id}/options"),
            ("patch", "/api/crf/options/{id}"),
            ("delete", "/api/crf/options/{id}"),
            ("post", "/api/crf/items/{item_id}/units"),
            ("patch", "/api/crf/units/{id}"),
            ("delete", "/api/crf/units/{id}"),
            ("post", "/api/crf/forms/{form_id}/domain-annotations"),
            ("patch", "/api/crf/domain-annotations/{id}"),
            ("delete", "/api/crf/domain-annotations/{id}"),
            ("post", "/api/crf/annotations"),
            ("patch", "/api/crf/annotations/{id}"),
            ("delete", "/api/crf/annotations/{id}"),
        ];
        for (method, path) in crf_reads.iter().chain(crf_writes.iter()) {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }
        for (method, path) in crf_writes.iter() {
            let op = &doc["paths"][path][method];
            let response_keys: Vec<&str> = op["responses"]
                .as_object()
                .expect("responses object")
                .keys()
                .map(|s| s.as_str())
                .collect();
            assert!(
                response_keys.contains(&"403"),
                "{method} {path} must advertise a 403 response (got {response_keys:?})",
            );
        }
        assert!(doc["paths"]["/api/log-ingest/submit"].is_object());
    }

    // ---- /api/user integration --------------------------------------

    /// `GET /api/user` round-trips through the top-level router:
    /// `AuthClaims` verifies the bearer, the user router hands off
    /// to `StubUserService.list()`, and the projected body comes
    /// back as 200 OK with the expected shape.
    #[tokio::test]
    async fn user_list_route_is_wired() {
        let app = router(test_state_with_user());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let users = value["users"].as_array().expect("users array");
        assert_eq!(users.len(), 1);
        assert_eq!(users[0]["code"], "u1");
        assert_eq!(users[0]["role"], "admin");
    }

    /// `GET /api/user/{code}` round-trips through the top-level
    /// router: 200 OK with the projected body.
    #[tokio::test]
    async fn user_get_by_code_route_is_wired() {
        let app = router(test_state_with_user());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user/u1")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], "u1");
    }

    /// No bearer at all: the top-level router must reject every
    /// `/api/user/*` route with 401, regardless of HTTP method.
    /// Sample one representative method (GET) — the AuthClaims
    /// extractor gates all four user routes uniformly.
    #[tokio::test]
    async fn user_route_without_authorization_returns_401() {
        let app = router(test_state_with_user());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], "token_verification_failed");
    }

    // ---- /api/project integration ------------------------------------

    #[tokio::test]
    async fn project_list_route_is_wired() {
        let app = router(test_state_with_project());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/project")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let projects = value["projects"].as_array().expect("projects array");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["code"], "pr1");
    }

    #[tokio::test]
    async fn project_route_without_authorization_returns_401() {
        let app = router(test_state_with_project());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/project")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }

    // ---- /api/auth/user-credential integration ----------------------

    /// `POST /api/auth/user-credential` IS a supported method — the
    /// administrator-only registration route. The router must register
    /// it alongside the existing `PATCH` handler. Because the canonical
    /// backend would dispatch this to the auth usecase, the router-only
    /// fixture (which has no `register_user` behaviour) surfaces that
    /// request as a `500` once it reaches the handler. The negative
    /// shape (`405`) is the assertion that the path is registered —
    /// `unimplemented!()` blows up inside the handler rather than the
    /// router.
    #[tokio::test]
    async fn user_credential_create_route_is_registered() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/user-credential")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer good")
                    .body(Body::from(
                        r#"{"user_code":"u2","user_name":"Bob","domain_name":"aegis.local","hostname":"h","sid":"s","password":"p"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        // The path is registered for POST (alongside PATCH); the
        // router-only MockAuth's `register_user` returns
        // `unimplemented!()`, so we expect a 5xx once the request
        // reaches the handler. The important negative is `405` (the
        // path is NOT unregistered) and `404` (the path IS mounted).
        assert_ne!(
            response.status(),
            AxStatus::METHOD_NOT_ALLOWED,
            "POST should be registered alongside PATCH"
        );
        assert_ne!(
            response.status(),
            AxStatus::NOT_FOUND,
            "POST path must be mounted on the router"
        );
    }

    /// No bearer at all: the top-level router must reject the
    /// `/api/auth/user-credential` PATCH route with 401. (POST is now
    /// also mounted for the administrator registration endpoint; its
    /// 401 path is covered by `user_credential_create_route_without_authorization_returns_401`.)
    #[tokio::test]
    async fn user_credential_route_without_authorization_returns_401() {
        let app = router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/auth/user-credential")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], "token_verification_failed");
    }

    /// POST shares the same bearer-auth gate as PATCH. Without a
    /// token the router must reject the request with 401 — the
    /// admin/root gate lives in the handler, but the AuthClaims
    /// extractor fires first.
    #[tokio::test]
    async fn user_credential_create_route_without_authorization_returns_401() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/user-credential")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"user_code":"u2","user_name":"Bob","domain_name":"aegis.local","hostname":"h","sid":"s","password":"p"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }

    // ---- /api/terminology integration --------------------------------

    /// `MockAuth::verify` returns `Role::Admin` so the role gate in the
    /// terminology write handlers passes — the `StubTerminologyService`
    /// below only needs to satisfy the usecase contract.
    fn sample_terminology_version_view(id: i64) -> apis::terminology::TerminologyVersionView {
        apis::terminology::TerminologyVersionView {
            id,
            kind: apis::terminology::TerminologyKind::Sdtm,
            name: format!("v{id}"),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn sample_code_list_view(id: i64) -> apis::terminology::CodeListView {
        apis::terminology::CodeListView {
            id,
            version_id: 1,
            code: format!("C{id}"),
            extensible: true,
            name: format!("codelist {id}"),
            submission_value: format!("C{id}"),
            synonym: String::new(),
            definition: String::new(),
            nci_preferred_term: String::new(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    /// Minimal `TerminologyService` for router-integration tests.
    /// Returns a single-version list from `list_versions`, a single
    /// version from `get_version_by_id` / `create_version` /
    /// `update_version`, and panics on every
    /// other call. The per-handler tests in
    /// `terminology::handlers::tests` cover the rest of the
    /// surface.
    #[derive(Clone)]
    struct StubTerminologyService;

    #[async_trait]
    impl apis::terminology::TerminologyService for StubTerminologyService {
        async fn create_version(
            &self,
            req: apis::terminology::CreateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            Ok(apis::terminology::TerminologyVersionView {
                id: 1,
                kind: req.kind,
                name: req.name,
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        }
        async fn list_versions(
            &self,
        ) -> Result<
            Vec<apis::terminology::TerminologyVersionView>,
            apis::terminology::TerminologyApiError,
        > {
            Ok(vec![sample_terminology_version_view(1)])
        }
        async fn get_version_by_id(
            &self,
            id: i64,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            Ok(sample_terminology_version_view(id))
        }
        async fn update_version(
            &self,
            req: apis::terminology::UpdateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            Ok(sample_terminology_version_view(req.id))
        }
        async fn delete_version(
            &self,
            _id: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            Ok(())
        }
        async fn create_code_list(
            &self,
            _req: apis::terminology::CreateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_lists(
            &self,
            query: apis::terminology::CodeListListQuery,
        ) -> Result<
            apis::terminology::Page<apis::terminology::CodeListView>,
            apis::terminology::TerminologyApiError,
        > {
            // Echo the offset back as a single-item page so tests
            // can observe that the query reaches the service.
            let id = query.offset as i64 + 1;
            Ok(apis::terminology::Page {
                items: vec![sample_code_list_view(id)],
                next_offset: None,
            })
        }
        async fn get_code_list_by_id(
            &self,
            id: i64,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            Ok(sample_code_list_view(id))
        }
        async fn update_code_list(
            &self,
            _req: apis::terminology::UpdateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
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
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_items(
            &self,
            query: apis::terminology::CodeItemListQuery,
        ) -> Result<
            apis::terminology::Page<apis::terminology::CodeItemView>,
            apis::terminology::TerminologyApiError,
        > {
            let id = query.offset as i64 + 1;
            // `version_id` and `codelist_id` are now optional on
            // the query: when the caller restricts to one owning
            // version / codelist we echo it back, when they omit
            // it we fall back to 0 (this stub only exists to
            // satisfy the router test, so any deterministic
            // default is fine).
            let version_id = query.version_id.unwrap_or(0);
            let codelist_id = query.codelist_id.unwrap_or(0);
            Ok(apis::terminology::Page {
                items: vec![apis::terminology::CodeItemView {
                    id,
                    codelist_id,
                    version_id,
                    code: format!("CI{id}"),
                    submission_value: String::new(),
                    synonym: String::new(),
                    definition: String::new(),
                    nci_preferred_term: String::new(),
                    created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                }],
                next_offset: None,
            })
        }
        async fn list_code_items_by_version_and_code(
            &self,
            _version_id: i64,
            _code: &str,
        ) -> Result<Vec<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn update_code_item(
            &self,
            _req: apis::terminology::UpdateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError>
        {
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

    /// State builder for terminology-integration tests: `MockAuth`
    /// (so verify returns OK with Role::Admin) plus a
    /// `StubTerminologyService` that returns positive responses
    /// for the version routes.
    fn test_state_with_terminology() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(StubTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(crate::state::test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(crate::state::test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(crate::state::test_support::NullCrfService)
                as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(crate::state::test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    /// `GET /api/terminology/versions` round-trips through the
    /// top-level router: 200 OK with the projected body.
    #[tokio::test]
    async fn terminology_list_versions_route_is_wired() {
        let app = router(test_state_with_terminology());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/terminology/versions")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let versions = value["versions"].as_array().expect("versions array");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0]["name"], "v1");
        assert_eq!(versions[0]["kind"], "sdtm");
    }

    /// No bearer at all: the top-level router must reject every
    /// `/api/terminology/*` route with 401, regardless of HTTP
    /// method. Sample one representative read route (GET).
    #[tokio::test]
    async fn terminology_route_without_authorization_returns_401() {
        let app = router(test_state_with_terminology());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/terminology/versions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }

    /// `GET /api/terminology/code-lists` returns the unified paged
    /// body. The stub echoes `offset` as the row id so we can
    /// verify the offset query reached the service.
    #[tokio::test]
    async fn terminology_list_code_lists_route_is_wired() {
        let app = router(test_state_with_terminology());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/terminology/code-lists?versionId=1&offset=5&limit=10&fragment=AGE")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let items = value["items"].as_array().expect("items array");
        assert_eq!(items.len(), 1);
        // offset=5 → stub returns id 6.
        assert_eq!(items[0]["id"], 6);
        assert!(value.get("nextOffset").is_none() || value["nextOffset"].is_null());
    }

    /// `GET /api/terminology/code-items` returns the unified paged
    /// body. Same echo trick as the codelists test.
    #[tokio::test]
    async fn terminology_list_code_items_route_is_wired() {
        let app = router(test_state_with_terminology());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/terminology/code-items?codelistId=11&offset=3&limit=4")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let items = value["items"].as_array().expect("items array");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["codelistId"], 11);
        assert_eq!(items[0]["id"], 4);
    }

    // Note: validation of reserved `tsquery` characters in `fragment`
    // is exercised at the usecase layer (see
    // `terminology::usecase::tests::list_code_lists_rejects_fragment_with_reserved_tsquery_chars`).
    // The router-level `StubTerminologyService` skips the usecase so the
    // handler cannot observe validation here.

    // ---- /api/crf integration ----------------------------------------

    /// Stub that returns a single version from `list_versions_by_project`
    /// / `get_version_by_id` and panics on every other call.
    /// Mirrors the terminology stub.
    #[derive(Clone)]
    struct StubCrfService;

    fn sample_crf_version_view(id: i64, project_code: &str) -> apis::crf::CrfVersionView {
        apis::crf::CrfVersionView {
            id,
            project_code: project_code.to_string(),
            name: format!("v{id}"),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[async_trait]
    impl apis::crf::CrfService for StubCrfService {
        async fn create_version(
            &self,
            req: apis::crf::CreateCrfVersionRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            Ok(sample_crf_version_view(1, &req.project_code))
        }
        async fn get_version_by_id(
            &self,
            req: apis::crf::GetCrfVersionByIdRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            Ok(sample_crf_version_view(req.id, "pr1"))
        }
        async fn list_versions_by_project(
            &self,
            req: apis::crf::ListCrfVersionsByProjectRequest,
        ) -> Result<Vec<apis::crf::CrfVersionView>, apis::crf::CrfApiError> {
            Ok(vec![sample_crf_version_view(1, &req.project_code)])
        }
        async fn update_version(
            &self,
            req: apis::crf::UpdateCrfVersionRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            Ok(sample_crf_version_view(req.id, "pr1"))
        }
        async fn delete_version(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            Ok(())
        }
        // Everything else panics — the per-handler tests in
        // `crf::handlers::tests` cover the translation surface in
        // detail; the router-level test only needs one wired route
        // per aggregate to prove the routes are mounted under
        // `/api/crf`.
        async fn create_form(
            &self,
            _req: apis::crf::CreateCrfFormRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn bulk_create_form(
            &self,
            _req: apis::crf::BulkCreateCrfFormRequest,
        ) -> Result<apis::crf::BulkCreateCrfFormResult, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_form_by_id(
            &self,
            req: apis::crf::GetCrfFormByIdRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            Ok(apis::crf::CrfFormView {
                id: req.id,
                version_id: 1,
                code: format!("F{}", req.id),
                name: format!("Form {}", req.id),
                order: 1,
                not_submitted: false,
                approved: false,
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        }
        async fn get_form_detail(
            &self,
            req: apis::crf::GetCrfFormDetailRequest,
        ) -> Result<apis::crf::CrfFormDetailView, apis::crf::CrfApiError> {
            use chrono::TimeZone;
            let now = chrono::Utc.timestamp_opt(0, 0).unwrap();
            let form_view = apis::crf::CrfFormView {
                id: req.form_id,
                version_id: 1,
                code: format!("F{}", req.form_id),
                name: format!("Form {}", req.form_id),
                order: 0,
                not_submitted: false,
                approved: false,
                created_at: now,
                updated_at: now,
            };
            Ok(apis::crf::CrfFormDetailView {
                form: form_view,
                form_annotations: Vec::new(),
                items: Vec::new(),
                domain_annotations: Vec::new(),
            })
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
            Ok(())
        }
        async fn set_approved(
            &self,
            _req: apis::crf::SetCrfApprovedRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
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
            req: apis::crf::GetCrfItemByIdRequest,
        ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
            Ok(apis::crf::CrfItemView {
                id: req.id,
                form_id: 1,
                code: format!("I{}", req.id),
                name: format!("Item {}", req.id),
                kind: apis::crf::CrfItemKind::Text,
                order: 1,
                not_submitted: false,
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
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
            Ok(())
        }
        async fn create_option(
            &self,
            _req: apis::crf::CreateCrfOptionRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_option_by_id(
            &self,
            req: apis::crf::GetCrfOptionByIdRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            Ok(apis::crf::CrfOptionView {
                id: req.id,
                item_id: 1,
                value: format!("O{}", req.id),
                not_submitted: false,
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
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
            Ok(())
        }
        async fn create_unit(
            &self,
            _req: apis::crf::CreateCrfUnitRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_unit_by_id(
            &self,
            req: apis::crf::GetCrfUnitByIdRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            Ok(apis::crf::CrfUnitView {
                id: req.id,
                item_id: 1,
                value: format!("U{}", req.id),
                not_submitted: false,
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
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
            Ok(())
        }
        async fn create_domain_annotation(
            &self,
            _req: apis::crf::CreateDomainAnnotationRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_domain_annotation_by_id(
            &self,
            req: apis::crf::GetDomainAnnotationByIdRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            Ok(apis::crf::DomainAnnotationView {
                id: req.id,
                form_id: 1,
                name: format!("DA{}", req.id),
                description: String::new(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
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
            Ok(())
        }
        async fn create_annotation(
            &self,
            _req: apis::crf::CreateAnnotationRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_annotation_by_id(
            &self,
            req: apis::crf::GetAnnotationByIdRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            Ok(apis::crf::AnnotationView {
                id: req.id,
                domain_annotation_id: 1,
                content: format!("A{}", req.id),
                assign: false,
                owner: apis::crf::AnnotationOwner::Form(1),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
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
            Ok(())
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

    fn test_state_with_crf() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(crate::state::test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(crate::state::test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(crate::state::test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(StubCrfService) as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(crate::state::test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    /// `GET /api/crf/projects/{project_code}/versions` round-trips
    /// through the top-level router and returns 200 OK with the
    /// projected body.
    #[tokio::test]
    async fn crf_list_versions_by_project_route_is_wired() {
        let app = router(test_state_with_crf());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/crf/projects/pr1/versions")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let versions = value["versions"].as_array().expect("versions array");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0]["projectCode"], "pr1");
        assert_eq!(versions[0]["name"], "v1");
    }

    /// `GET /api/crf/versions/{id}` round-trips through the
    /// top-level router and returns 200 OK with the projected body.
    #[tokio::test]
    async fn crf_get_version_by_id_route_is_wired() {
        let app = router(test_state_with_crf());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/crf/versions/7")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["id"], 7);
        assert_eq!(value["projectCode"], "pr1");
    }

    /// No bearer: every `/api/crf/*` route must reject the
    /// request with 401. Sample one read route (GET).
    #[tokio::test]
    async fn crf_route_without_authorization_returns_401() {
        let app = router(test_state_with_crf());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/crf/versions/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }

    /// `DELETE /api/crf/versions/{id}` round-trips through the
    /// router: 204 No Content.
    #[tokio::test]
    async fn crf_delete_version_route_is_wired() {
        let app = router(test_state_with_crf());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/crf/versions/1")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::NO_CONTENT);
    }
}
