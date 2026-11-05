//! HTTP handlers for the product and project namespaces.
//!
//! Each handler is a thin adapter over [`crate::transport::http::dto`]
//! and the apis DTOs. The `AuthClaims` extractor gates the route on
//! a valid `Authorization: Bearer <token>` header; the
//! [`require_admin_or_root`] helper enforces write authorization.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::{AuthClaims, require_admin_or_root};
use crate::transport::http::dto::{self, PathCode};
use crate::transport::http::error::ApiError;

/// Translate the wire DTO into the apis DTO for project membership.
fn member_data(value: dto::ProjectMemberDataRequest) -> apis::project::ProjectMemberData {
    apis::project::ProjectMemberData {
        leaders: value.leaders,
        workers: value.workers,
    }
}

/// Translate a wire configuration DTO into the apis DTO. The
/// configuration carries an optional language, a list of tags, an
/// optional SDTM-IG version pointer, and an optional SDTM
/// controlled-terminology version pointer; validation (non-empty
/// key/value for each tag, non-empty sdtmig/sdtm_terminology
/// names, known language value) is delegated to the domain layer —
/// the handler just passes through whatever the client supplied.
fn configuration_data(
    value: dto::ProjectConfigurationDataRequest,
) -> apis::project::ProjectConfigurationData {
    apis::project::ProjectConfigurationData {
        language: value.language.map(Into::into),
        tags: value
            .tags
            .into_iter()
            .map(|t| apis::project::TagData {
                key: t.key,
                value: t.value,
            })
            .collect(),
        sdtmig: value.sdtmig.map(Into::into),
        sdtm_terminology: value.sdtm_terminology.map(Into::into),
    }
}

// -- projects --------------------------------------------------------------

/// `POST /api/project` — create a project.
#[utoipa::path(
    post, path = "", tag = "project",
    operation_id = "project_create",
    request_body = dto::CreateProjectRequest,
    responses(
        (status = 201, description = "Project created", body = dto::ProjectViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Referenced user not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Project code already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_project(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateProjectRequest>,
) -> Result<(StatusCode, Json<dto::ProjectViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .project
        .create_project(apis::project::CreateProjectRequest {
            code: req.code,
            description: req.description,
            members: req.members.map(member_data),
            unblind_members: req.unblind_members.map(member_data),
            configurations: req.configurations.map(configuration_data),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/project` — list projects.
#[utoipa::path(
    get, path = "", tag = "project",
    operation_id = "project_list",
    responses(
        (status = 200, description = "Projects list", body = dto::ProjectListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_projects(
    State(state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<dto::ProjectListResponse>, ApiError> {
    let views = state.project.list_projects().await?;
    let projects = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::ProjectListResponse { projects }))
}

/// `GET /api/project/{code}` — fetch a project by its code.
#[utoipa::path(
    get, path = "/{code}", tag = "project",
    operation_id = "project_get_by_code",
    params(
        ("code" = String, Path, description = "Project code to fetch"),
    ),
    responses(
        (status = 200, description = "Project found", body = dto::ProjectViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_project_by_code(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
) -> Result<Json<dto::ProjectViewResponse>, ApiError> {
    let view = state.project.get_project_by_code(&code).await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/project/{code}` — partial update of a project.
///
/// Membership semantics preserved through the wire DTO:
/// - `None` for a membership field leaves the corresponding team
///   unchanged.
/// - `Some(empty)` (a present `{}`) wipes the corresponding team's
///   rows.
///
/// `configurations` follows the same missing-vs-empty distinction.
/// Missing leaves the configuration alone; a present (possibly
/// empty) configuration replaces the whole configuration object.
#[utoipa::path(
    patch, path = "/{code}", tag = "project",
    operation_id = "project_update",
    params(
        ("code" = String, Path, description = "Project code to update"),
    ),
    request_body = dto::UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated", body = dto::ProjectViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project or member not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Project code already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_project(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
    Json(req): Json<dto::UpdateProjectRequest>,
) -> Result<Json<dto::ProjectViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let id = state.project.get_project_by_code(&code).await?.id;
    let view = state
        .project
        .update_project(apis::project::UpdateProjectRequest {
            id,
            code: req.code,
            description: req.description,
            active: req.active,
            members: req.members.map(member_data),
            unblind_members: req.unblind_members.map(member_data),
            configurations: req.configurations.map(configuration_data),
        })
        .await?;
    Ok(Json(view.into()))
}

#[cfg(test)]
mod tests {
    //! Handler tests for both `/api/product` and `/api/project`.
    //!
    //! `MockProjectService` is configurable per method; `MockAuth`
    //! returns a fixed role so the role guard is exercised on the
    //! write routes. The per-handler tests cover the success path,
    //! `401` for missing bearer, `403` for `General` writes, and
    //! the spec-approved `ProjectApiError` mappings.

    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use axum::routing::{get, post};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    use crate::state::test_support::{
        NullCrfService, NullDomainModelService, NullMissionService, NullSdtmProjectService,
        NullTerminologyService,
    };

    use apis::auth::{
        AuthApiError, AuthClaims as ApisAuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };

    /// Build an `AuthClaims` (the wrapper extractor in
    /// `crate::transport::http::auth::middleware`) with a
    /// configurable role for `require_admin_or_root` direct-call
    /// tests. Local to the test module so the production lib does
    /// not ship a helper that is only used by tests.
    fn role_claims(role: apis::user::Role) -> AuthClaims {
        AuthClaims(ApisAuthClaims {
            code: "caller".into(),
            role,
            token_version: 0,
        })
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

    #[derive(Clone, Default)]
    pub struct MockProjectService {
        pub create_project: Option<apis::project::ProjectView>,
        pub create_project_err: Option<apis::project::ProjectApiError>,
        pub get_project_by_code: Option<apis::project::ProjectView>,
        pub get_project_by_code_err: Option<apis::project::ProjectApiError>,
        pub list_projects: Option<Vec<apis::project::ProjectView>>,
        pub list_projects_err: Option<apis::project::ProjectApiError>,
        pub update_project: Option<apis::project::ProjectView>,
        pub update_project_err: Option<apis::project::ProjectApiError>,
        pub last_create_project_args: Arc<Mutex<Option<apis::project::CreateProjectRequest>>>,
        pub last_update_project_args: Arc<Mutex<Option<apis::project::UpdateProjectRequest>>>,
    }

    #[async_trait]
    impl apis::project::ProjectService for MockProjectService {
        async fn create_project(
            &self,
            req: apis::project::CreateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            *self.last_create_project_args.lock().unwrap() = Some(req);
            if let Some(err) = self.create_project_err.clone() {
                return Err(err);
            }
            Ok(self
                .create_project
                .clone()
                .expect("create_project result configured"))
        }
        async fn get_project_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!("not exposed at HTTP")
        }
        async fn get_project_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            if let Some(err) = self.get_project_by_code_err.clone() {
                return Err(err);
            }
            Ok(self
                .get_project_by_code
                .clone()
                .expect("get_project_by_code result configured"))
        }
        async fn list_projects(
            &self,
        ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
            if let Some(err) = self.list_projects_err.clone() {
                return Err(err);
            }
            Ok(self
                .list_projects
                .clone()
                .expect("list_projects result configured"))
        }
        async fn update_project(
            &self,
            req: apis::project::UpdateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            *self.last_update_project_args.lock().unwrap() = Some(req);
            if let Some(err) = self.update_project_err.clone() {
                return Err(err);
            }
            Ok(self
                .update_project
                .clone()
                .expect("update_project result configured"))
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

    #[derive(Clone)]
    pub struct MockAuth {
        pub role: apis::user::Role,
    }

    impl Default for MockAuth {
        fn default() -> Self {
            Self {
                role: apis::user::Role::Admin,
            }
        }
    }

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _req: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            unimplemented!()
        }
        async fn login_with_domain_user_info(
            &self,
            _req: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> {
            unimplemented!()
        }
        async fn verify(&self, _req: VerifyRequest) -> Result<ApisAuthClaims, AuthApiError> {
            Ok(ApisAuthClaims {
                code: "u1".into(),
                role: self.role,
                token_version: 0,
            })
        }
        async fn refresh(&self, _req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
            unimplemented!()
        }
        async fn find_user_credential_by_code(
            &self,
            _code: &str,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
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
            unimplemented!()
        }
        async fn register_user(
            &self,
            _req: RegisterUserRequest,
        ) -> Result<RegisterUserResponse, AuthApiError> {
            unimplemented!("not exercised by this handler")
        }
    }

    pub fn test_state(project: MockProjectService, auth: MockAuth) -> AppState {
        AppState {
            auth: Arc::new(auth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(project) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(NullMissionService) as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(NullCrfService) as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

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

    pub fn app(state: AppState) -> Router {
        Router::new()
            .route("/api/project", post(create_project).get(list_projects))
            .route(
                "/api/project/{code}",
                get(get_project_by_code).patch(update_project),
            )
            .with_state(state)
    }

    pub async fn read_json(response: axum::response::Response) -> (AxStatus, serde_json::Value) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    pub fn build_request(
        method: &str,
        uri: &str,
        body: Option<String>,
        auth: Option<&str>,
    ) -> Request<Body> {
        let mut b = Request::builder().method(method).uri(uri);
        if let Some(token) = auth {
            b = b.header("authorization", token);
        }
        if body.is_some() {
            b = b.header("content-type", "application/json");
        }
        b.body(body.map(Body::from).unwrap_or(Body::empty()))
            .unwrap()
    }

    // ---- role guard -------------------------------------------------

    #[test]
    fn write_guard_accepts_root_and_admin() {
        require_admin_or_root(&role_claims(apis::user::Role::Root)).unwrap();
        require_admin_or_root(&role_claims(apis::user::Role::Admin)).unwrap();
    }

    #[test]
    fn write_guard_rejects_general() {
        let err = require_admin_or_root(&role_claims(apis::user::Role::General)).unwrap_err();
        assert_eq!(err.status(), AxStatus::FORBIDDEN);
        assert_eq!(err.code(), "forbidden");
    }

    // ---- project handlers -------------------------------------------

    #[tokio::test]
    async fn create_project_root_returns_201() {
        let project = MockProjectService {
            create_project: Some(sample_project_view(9, "pr1")),
            ..Default::default()
        };
        let auth = MockAuth {
            role: apis::user::Role::Root,
        };
        let app = app(test_state(project.clone(), auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project",
                Some(
                    r#"{"code":"pr1","description":"x","members":{"leaders":["l1"]},"configurations":{"language":"en","tags":[{"key":"Product","value":"DEMO-001"}]}}"#
                        .to_string(),
                ),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CREATED);
        assert_eq!(body["id"], 9);
        let captured = project
            .last_create_project_args
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        assert_eq!(captured.code, "pr1");
        let members = captured.members.expect("members present");
        assert_eq!(members.leaders, vec!["l1".to_string()]);
        assert!(members.workers.is_empty());
        let configurations = captured.configurations.expect("configurations present");
        assert!(matches!(
            configurations.language,
            Some(apis::project::ProjectLanguage::English)
        ));
        assert_eq!(configurations.tags.len(), 1);
        assert_eq!(configurations.tags[0].key, "Product");
        assert_eq!(configurations.tags[0].value, "DEMO-001");
    }

    #[tokio::test]
    async fn create_project_general_returns_403() {
        let project = MockProjectService::default();
        let auth = MockAuth {
            role: apis::user::Role::General,
        };
        let app = app(test_state(project, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project",
                Some(r#"{"code":"pr1","description":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");
    }

    #[tokio::test]
    async fn create_project_without_authorization_returns_401() {
        let app = app(test_state(
            MockProjectService::default(),
            MockAuth::default(),
        ));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project",
                Some(r#"{"code":"pr1","description":"x"}"#.to_string()),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    #[tokio::test]
    async fn create_project_user_not_found_maps_to_404() {
        let project = MockProjectService {
            create_project_err: Some(apis::project::ProjectApiError::UserNotFound("u1".into())),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project",
                Some(r#"{"code":"pr1","description":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "user_not_found");
    }

    #[tokio::test]
    async fn create_project_duplicate_code_maps_to_409() {
        let project = MockProjectService {
            create_project_err: Some(apis::project::ProjectApiError::DuplicateCode("pr1".into())),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project",
                Some(r#"{"code":"pr1","description":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CONFLICT);
        assert_eq!(body["code"], "duplicate_code");
    }

    #[tokio::test]
    async fn create_project_validation_maps_to_400() {
        let project = MockProjectService {
            create_project_err: Some(apis::project::ProjectApiError::Validation("bad".into())),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project",
                Some(r#"{"code":"pr1","description":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
        assert_eq!(body["code"], "validation_failed");
    }

    #[tokio::test]
    async fn create_project_repository_maps_to_500() {
        let project = MockProjectService {
            create_project_err: Some(apis::project::ProjectApiError::Repository("db".into())),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project",
                Some(r#"{"code":"pr1","description":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "repository_error");
    }

    #[tokio::test]
    async fn list_projects_returns_200_with_array() {
        let project = MockProjectService {
            list_projects: Some(vec![sample_project_view(9, "pr1")]),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/project",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let projects = body["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["code"], "pr1");
    }

    #[tokio::test]
    async fn list_projects_without_authorization_returns_401() {
        let app = app(test_state(
            MockProjectService::default(),
            MockAuth::default(),
        ));
        let response = app
            .oneshot(build_request("GET", "/api/project", None, None))
            .await
            .unwrap();
        assert_eq!(read_json(response).await.0, AxStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn list_projects_general_reads_succeed() {
        let project = MockProjectService {
            list_projects: Some(Vec::new()),
            ..Default::default()
        };
        let auth = MockAuth {
            role: apis::user::Role::General,
        };
        let app = app(test_state(project, auth));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/project",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        assert_eq!(read_json(response).await.0, AxStatus::OK);
    }

    #[tokio::test]
    async fn get_project_by_code_returns_200() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/project/pr1",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["code"], "pr1");
    }

    #[tokio::test]
    async fn get_project_by_code_not_found_maps_to_404() {
        let project = MockProjectService {
            get_project_by_code_err: Some(apis::project::ProjectApiError::NotFound),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/project/pr1",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn get_project_by_code_without_authorization_returns_401() {
        let app = app(test_state(
            MockProjectService::default(),
            MockAuth::default(),
        ));
        let response = app
            .oneshot(build_request("GET", "/api/project/pr1", None, None))
            .await
            .unwrap();
        assert_eq!(read_json(response).await.0, AxStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn update_project_resolves_code_to_id_and_preserves_missing_members() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            update_project: Some(sample_project_view(9, "pr1")),
            ..Default::default()
        };
        let app = app(test_state(project.clone(), MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/project/pr1",
                Some(r#"{"description":"new"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, _body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let captured = project
            .last_update_project_args
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        assert_eq!(captured.id, 9);
        assert_eq!(captured.description.as_deref(), Some("new"));
        assert!(captured.members.is_none());
        assert!(captured.unblind_members.is_none());
    }

    #[tokio::test]
    async fn update_project_preserves_empty_members_as_some() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            update_project: Some(sample_project_view(9, "pr1")),
            ..Default::default()
        };
        let app = app(test_state(project.clone(), MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/project/pr1",
                Some(r#"{"members":{},"unblindMembers":{}}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, _body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let captured = project
            .last_update_project_args
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        let members = captured.members.expect("members present");
        assert!(members.leaders.is_empty());
        assert!(members.workers.is_empty());
        let unblind = captured.unblind_members.expect("unblind present");
        assert!(unblind.leaders.is_empty());
        assert!(unblind.workers.is_empty());
    }

    #[tokio::test]
    async fn update_project_preserves_non_empty_membership() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            update_project: Some(sample_project_view(9, "pr1")),
            ..Default::default()
        };
        let app = app(test_state(project.clone(), MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/project/pr1",
                Some(r#"{"unblindMembers":{"leaders":["l1"],"workers":["w1","w2"]}}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, _body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let captured = project
            .last_update_project_args
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        let unblind = captured.unblind_members.expect("unblind present");
        assert_eq!(unblind.leaders, vec!["l1".to_string()]);
        assert_eq!(unblind.workers, vec!["w1".to_string(), "w2".to_string()]);
        assert!(captured.members.is_none());
    }

    #[tokio::test]
    async fn update_project_general_returns_403() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            ..Default::default()
        };
        let auth = MockAuth {
            role: apis::user::Role::General,
        };
        let app = app(test_state(project, auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/project/pr1",
                Some(r#"{"description":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");
    }

    #[tokio::test]
    async fn update_project_lookup_not_found_maps_to_404() {
        let project = MockProjectService {
            get_project_by_code_err: Some(apis::project::ProjectApiError::NotFound),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/project/pr1",
                Some(r#"{"description":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn update_project_validation_maps_to_400() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            update_project_err: Some(apis::project::ProjectApiError::Validation("bad".into())),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/project/pr1",
                Some(r#"{"description":""}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
        assert_eq!(body["code"], "validation_failed");
    }
}
