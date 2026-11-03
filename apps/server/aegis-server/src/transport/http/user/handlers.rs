//! HTTP handlers for the user CRUD namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from [`crate::transport::http::dto`])
//!    into an apis DTO.
//! 2. Calls the corresponding [`apis::user::UserService`] method on
//!    [`crate::state::AppState`].
//! 3. Translates the apis response back into a wire DTO.
//!
//! `UserApiError` is funnelled through [`ApiError::from`] so each
//! route returns `Result<Json<T>, ApiError>` and the error mapping
//! in [`error`](crate::transport::http::error) does the rest.
//!
//! The `AuthClaims` extractor in the argument list is what gates
//! the route on a valid `Authorization: Bearer <token>` header. The
//! handler bodies ignore the claims value (no role checks at this
//! stage).

// `UserService` is in scope so the future handler bodies (Tasks 5-8)
// can call `state.user.X(...)`; the stub bodies don't use it yet.
#[allow(unused_imports)]
use apis::user::UserService;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto::{self, PathCode};
use crate::transport::http::error::ApiError;

// Stubs: Tasks 5-8 replace these bodies with real implementations.
// They exist now so the router (Task 4) compiles end-to-end. Each
// stub carries a minimal `#[utoipa::path]` annotation so the
// `routes!` macro can register it; Tasks 5-8 expand the annotation
// with the right method, path, and request/response schemas.

/// `POST /api/user` — create a user.
///
/// Wire DTO → apis DTO translation happens at the boundary; the
/// backend adapter receives an `apis::user::CreateUserRequest`
/// (which deliberately omits `password`) and returns a full
/// [`apis::user::UserView`]. The 201 response body is the wire
/// projection of that view.
#[utoipa::path(
    post, path = "/", tag = "user",
    operation_id = "user_create",
    request_body = dto::CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = dto::UserViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "User code already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository / hashing failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<dto::CreateUserRequest>,
) -> Result<(StatusCode, Json<dto::UserViewResponse>), ApiError> {
    let view = state
        .user
        .create(apis::user::CreateUserRequest {
            code: req.code,
            name: req.name,
            role: req.role.into(),
            active: true,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/user` — list all users.
///
/// The backend's `list` returns every user; the response wraps the
/// vector in [`dto::UserListResponse`] so future pagination metadata
/// (`total`, `next_cursor`, …) can land without breaking the wire
/// shape.
#[utoipa::path(
    get, path = "/", tag = "user",
    operation_id = "user_list",
    responses(
        (status = 200, description = "Users list", body = dto::UserListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list(
    State(state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<dto::UserListResponse>, ApiError> {
    let views = state.user.list().await?;
    let users = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::UserListResponse { users }))
}

/// `GET /api/user/{code}` — fetch a user by their `code`.
///
/// The `{code}` URL parameter is extracted via
/// [`dto::PathCode`]; the handler threads the bare `code` string
/// into `state.user.get_by_code` and returns the projected view.
#[utoipa::path(
    get, path = "/{code}", tag = "user",
    operation_id = "user_get_by_code",
    params(
        ("code" = String, Path, description = "User code to fetch"),
    ),
    responses(
        (status = 200, description = "User found", body = dto::UserViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_by_code(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    let view = state.user.get_by_code(&code).await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/user/{code}` — partial update of a user.
///
/// The wire DTO is `UpdateUserRequest` (every field optional,
/// `skip_serializing_if = "Option::is_none"` for lossless
/// round-trips). The handler:
/// 1. Resolves the URL `{code}` to an internal `id` via
///    `state.user.get_by_code` — this is what makes the URL
///    contract stable while the backend identity model can evolve.
/// 2. Threads the optional fields through `From<Role>` so the
///    wire `Role` enum maps cleanly to `apis::user::Role`.
/// 3. Calls `state.user.update`, which returns the projected view.
#[utoipa::path(
    patch, path = "/{code}", tag = "user",
    operation_id = "user_update",
    params(
        ("code" = String, Path, description = "User code to update"),
    ),
    request_body = dto::UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = dto::UserViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "User code already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository / hashing failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
    Json(req): Json<dto::UpdateUserRequest>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    // Resolve URL `{code}` to internal `id` so the wire contract
    // is stable even if the backend's identity model evolves.
    let id = state.user.get_by_code(&code).await?.id;
    let view = state
        .user
        .update(apis::user::UpdateUserRequest {
            id,
            code: req.code,
            name: req.name,
            role: req.role.map(Into::into),
            active: req.active,
        })
        .await?;
    Ok(Json(view.into()))
}

#[cfg(test)]
mod tests {
    //! Test scaffolding shared by the per-handler tests in Tasks 5-8.
    //!
    //! `MockUserService` and `MockAuth` are configurable fakes; each
    //! per-handler test sets the relevant fields and asserts the
    //! response status / body / error. `NullAuth` and `NullUserService`
    //! exist for the auth module's existing tests and are not used
    //! here.

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
        AuthApiError, AuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };

    /// Configurable mock for [`apis::user::UserService`]. Each method
    /// returns either a preconfigured success value or a preconfigured
    /// error; the `*_args` fields record the last request the method
    /// received so handler-translation tests can assert on it.
    #[derive(Clone, Default)]
    pub struct MockUserService {
        pub create: Option<apis::user::UserView>,
        pub create_err: Option<apis::user::UserApiError>,
        pub get_by_code: Option<apis::user::UserView>,
        pub get_by_code_err: Option<apis::user::UserApiError>,
        pub list: Option<Vec<apis::user::UserView>>,
        pub list_err: Option<apis::user::UserApiError>,
        pub update: Option<apis::user::UserView>,
        pub update_err: Option<apis::user::UserApiError>,

        pub last_create_args: Arc<Mutex<Option<apis::user::CreateUserRequest>>>,
        pub last_update_args: Arc<Mutex<Option<apis::user::UpdateUserRequest>>>,
    }

    #[async_trait]
    impl apis::user::UserService for MockUserService {
        async fn create(
            &self,
            req: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            *self.last_create_args.lock().unwrap() = Some(req);
            if let Some(err) = self.create_err.clone() {
                return Err(err);
            }
            Ok(self.create.clone().expect("create result configured"))
        }
        async fn get_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!("not exercised by the user router")
        }
        async fn get_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            if let Some(err) = self.get_by_code_err.clone() {
                return Err(err);
            }
            Ok(self
                .get_by_code
                .clone()
                .expect("get_by_code result configured"))
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            if let Some(err) = self.list_err.clone() {
                return Err(err);
            }
            Ok(self.list.clone().expect("list result configured"))
        }
        async fn update(
            &self,
            req: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            *self.last_update_args.lock().unwrap() = Some(req);
            if let Some(err) = self.update_err.clone() {
                return Err(err);
            }
            Ok(self.update.clone().expect("update result configured"))
        }
    }

    /// Configurable mock for [`apis::auth::AuthService`]. Only the
    /// `verify` method is exercised (by `AuthClaims::from_request_parts`);
    /// the rest are stubbed with `unimplemented!()`.
    #[derive(Clone, Default)]
    pub struct MockAuth {
        pub verify_ok: bool,
        pub verify_err: Option<AuthApiError>,
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
        async fn verify(&self, _req: VerifyRequest) -> Result<AuthClaims, AuthApiError> {
            if let Some(err) = self.verify_err.clone() {
                return Err(err);
            }
            assert!(
                self.verify_ok,
                "verify_ok must be set when no error is configured"
            );
            Ok(AuthClaims {
                code: "u1".into(),
                role: apis::user::Role::Admin,
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

    /// Configurable mock for [`apis::project::ProjectService`]. User
    /// router tests do not exercise project routes; the stub simply
    /// returns `NotFound` so an accidental reference is loud.
    #[derive(Clone)]
    pub struct NullProjectService;

    #[async_trait]
    impl apis::project::ProjectService for NullProjectService {
        async fn create_project(
            &self,
            _req: apis::project::CreateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Err(apis::project::ProjectApiError::NotFound)
        }
        async fn get_project_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Err(apis::project::ProjectApiError::NotFound)
        }
        async fn get_project_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Err(apis::project::ProjectApiError::NotFound)
        }
        async fn list_projects(
            &self,
        ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
            Ok(Vec::new())
        }
        async fn update_project(
            &self,
            _req: apis::project::UpdateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Err(apis::project::ProjectApiError::NotFound)
        }
    }

    /// Build an `AppState` from the supplied mocks.
    pub fn test_state(user: MockUserService, auth: MockAuth) -> AppState {
        AppState {
            auth: Arc::new(auth) as Arc<dyn AuthService>,
            user: Arc::new(user) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
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

    /// Build a router that mounts all four handlers at the
    /// `/api/user` prefix (matching the production nest path). Each
    /// per-handler test in Tasks 5-8 drives this router via
    /// `tower::ServiceExt::oneshot`.
    pub fn app(state: AppState) -> Router {
        Router::new()
            .route("/api/user", post(create).get(list))
            .route("/api/user/{code}", get(get_by_code).patch(update))
            .with_state(state)
    }

    /// Decode a response into (status, JSON body) for assertions.
    pub async fn read_json(response: axum::response::Response) -> (AxStatus, serde_json::Value) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    /// Construct a sample `UserView` for test fixtures.
    pub fn sample_user(id: i32, code: &str) -> apis::user::UserView {
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

    /// Drive a request with the supplied Authorization header value
    /// (or `None` to omit it). Body, when present, is owned by the
    /// caller so the resulting `Request<Body>` is `'static`.
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

    // ---- create ----------------------------------------------------

    #[tokio::test]
    async fn create_returns_201_with_user_view_on_success() {
        // Build the mock's response with `name = "Alice"` so the
        // response assertion matches the request body — this test
        // exercises end-to-end translation, not just the DTO pass.
        let mut alice = sample_user(42, "u1");
        alice.name = "Alice".to_string();
        let user = MockUserService {
            create: Some(alice),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user.clone(), auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"u1","name":"Alice","role":"admin"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CREATED);
        assert_eq!(body["id"], 42);
        assert_eq!(body["code"], "u1");
        assert_eq!(body["name"], "Alice");
        assert_eq!(body["role"], "admin");
        assert_eq!(body["active"], true);

        // Verify the wire->apis translation captured the right DTO.
        let captured = user.last_create_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.code, "u1");
        assert_eq!(captured.name, "Alice");
        assert!(matches!(captured.role, apis::user::Role::Admin));
    }

    #[tokio::test]
    async fn create_maps_duplicate_code_to_409() {
        let user = MockUserService {
            create_err: Some(apis::user::UserApiError::DuplicateCode("u1".into())),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"u1","name":"Alice","role":"admin"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CONFLICT);
        assert_eq!(body["code"], "duplicate_code");
    }

    #[tokio::test]
    async fn create_maps_validation_to_400() {
        let user = MockUserService {
            create_err: Some(apis::user::UserApiError::Validation("empty code".into())),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"","name":"Alice","role":"admin"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
        assert_eq!(body["code"], "validation_failed");
    }

    #[tokio::test]
    async fn create_without_authorization_returns_401() {
        // No Authorization header — `AuthClaims` rejects the request
        // before the handler body runs.
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"u1","name":"Alice","role":"admin"}"#.to_string()),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- list ------------------------------------------------------

    #[tokio::test]
    async fn list_returns_200_with_users_on_success() {
        let user = MockUserService {
            list: Some(vec![
                sample_user(1, "u1"),
                sample_user(2, "u2"),
                sample_user(3, "u3"),
            ]),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let users = body["users"].as_array().expect("users array");
        assert_eq!(users.len(), 3);
        assert_eq!(users[0]["code"], "u1");
        assert_eq!(users[1]["code"], "u2");
        assert_eq!(users[2]["code"], "u3");
        assert_eq!(users[0]["id"], 1);
        assert_eq!(users[1]["id"], 2);
        assert_eq!(users[2]["id"], 3);
    }

    #[tokio::test]
    async fn list_returns_200_with_empty_array_when_no_users() {
        let user = MockUserService {
            list: Some(vec![]),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["users"].as_array().map(|a| a.len()), Some(0));
    }

    #[tokio::test]
    async fn list_without_authorization_returns_401() {
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user", None, None))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- get_by_code -----------------------------------------------

    #[tokio::test]
    async fn get_by_code_returns_200_with_user_view_on_success() {
        let user = MockUserService {
            get_by_code: Some(sample_user(42, "u1")),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/user/u1",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["id"], 42);
        assert_eq!(body["code"], "u1");
        assert_eq!(body["name"], "User u1");
        assert_eq!(body["role"], "admin");
        assert_eq!(body["active"], true);
    }

    #[tokio::test]
    async fn get_by_code_maps_not_found_to_404() {
        let user = MockUserService {
            get_by_code_err: Some(apis::user::UserApiError::NotFound),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/user/missing",
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
    async fn get_by_code_without_authorization_returns_401() {
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user/u1", None, None))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- update ----------------------------------------------------

    #[tokio::test]
    async fn update_returns_200_with_user_view_on_success() {
        // `update` first resolves the URL `{code}` to an internal
        // `id` via `get_by_code`, then forwards the partial update
        // to `state.user.update`. The mock returns id=42 for
        // get_by_code("u1") and the resulting UserView from update.
        let mut updated = sample_user(42, "u1");
        updated.name = "Alice".to_string();
        let user = MockUserService {
            get_by_code: Some(sample_user(42, "u1")),
            update: Some(updated),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user.clone(), auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/u1",
                Some(r#"{"name":"Alice"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["id"], 42);
        assert_eq!(body["code"], "u1");
        assert_eq!(body["name"], "Alice");

        // Verify the handler resolved the URL code to id=42 and
        // forwarded only the supplied `name` field.
        let captured = user.last_update_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.id, 42);
        assert!(captured.code.is_none());
        assert_eq!(captured.name.as_deref(), Some("Alice"));
        assert!(captured.role.is_none());
        assert!(captured.active.is_none());
    }

    #[tokio::test]
    async fn update_maps_validation_to_400() {
        let user = MockUserService {
            get_by_code: Some(sample_user(42, "u1")),
            update_err: Some(apis::user::UserApiError::Validation("bad".into())),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/u1",
                Some(r#"{"name":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
        assert_eq!(body["code"], "validation_failed");
    }

    #[tokio::test]
    async fn update_maps_not_found_to_404() {
        // If the URL code doesn't resolve, the update never runs;
        // get_by_code's NotFound bubbles out.
        let user = MockUserService {
            get_by_code_err: Some(apis::user::UserApiError::NotFound),
            ..Default::default()
        };
        let auth = MockAuth {
            verify_ok: true,
            ..Default::default()
        };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/missing",
                Some(r#"{"name":"x"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn update_without_authorization_returns_401() {
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/u1",
                Some(r#"{"name":"x"}"#.to_string()),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
}
