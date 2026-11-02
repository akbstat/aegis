//! HTTP handlers for the user-credential management namespace.
//!
//! Two routes, `BearerAuth`-gated:
//!
//! - `POST /api/auth/user-credential` — register a new user
//!   (admin/root only)
//! - `PATCH /api/auth/user-credential` — update own credential
//!
//! Each handler is a thin adapter that:
//! 1. Reads the user code from
//!    [`AuthClaims`](crate::transport::http::auth::middleware::AuthClaims)
//!    (the caller's bearer token).
//! 2. Translates the wire DTO (from
//!    [`crate::transport::http::dto`]) into an apis DTO — the body
//!    only carries the raw password.
//! 3. Calls the corresponding [`apis::auth::AuthService`] method on
//!    [`crate::state::AppState`]. The auth usecase hashes the
//!    plaintext password before persisting — the wire API never
//!    sees a pre-hashed value at update time.
//! 4. Translates the apis response back into a wire DTO.
//!
//! `AuthApiError` is funnelled through [`ApiError::from`] so the
//! route returns `Result<Json<T>, ApiError>` and the error mapping
//! in [`crate::transport::http::error`] does the rest.

use apis::auth::{RegisterUserRequest, UpdateUserCredentialRequest};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto;
use crate::transport::http::error::ApiError;

/// `POST /api/auth/user-credential` — register a new user, credential
/// row, and domain identity in one call.
///
/// Authorization is restricted to `Role::Root` and `Role::Admin`;
/// `Role::General` callers receive `403 Forbidden` and the auth
/// service is never invoked. The path is `/` so the route lives at
/// `POST /api/auth/user-credential` alongside the existing PATCH
/// route.
#[utoipa::path(
    post, path = "/", tag = "user-credential",
    operation_id = "auth_register_user",
    request_body = dto::RegisterUserRequest,
    responses(
        (status = 201, description = "User registered", body = dto::RegisterUserResponse),
        (status = 400, description = "Validation failed (incl. disallowed domain)", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not an admin or root", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<dto::RegisterUserRequest>,
) -> Result<(StatusCode, Json<dto::RegisterUserResponse>), ApiError> {
    let view = state
        .auth
        .register_user(RegisterUserRequest {
            user_code: req.user_code,
            user_name: req.user_name,
            domain_name: req.domain_name,
            hostname: req.hostname,
            sid: req.sid,
            password: req.password,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `PATCH /api/auth/user-credential` — partial update of the caller's
/// credential. The handler reads `user_code` from claims; the body
/// optionally carries a new raw `password`. An empty body is permitted
/// and returns the unchanged view (the apis trait defines this
/// behavior).
///
/// The path is `/` so the route lives at `PATCH /api/auth/user-credential`
/// after the parent `/user-credential` nest in
/// [`crate::transport::http::auth::router`].
#[utoipa::path(
    patch, path = "/", tag = "user-credential",
    operation_id = "auth_update_user_credential",
    request_body = dto::UpdateUserCredentialRequest,
    responses(
        (status = 200, description = "User credential updated", body = dto::UserCredentialViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User credential not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::UpdateUserCredentialRequest>,
) -> Result<Json<dto::UserCredentialViewResponse>, ApiError> {
    let view = state
        .auth
        .update_user_credential(UpdateUserCredentialRequest {
            user_code: claims.0.code,
            password: req.password,
        })
        .await?;
    Ok(Json(view.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use axum::routing::post;
    use std::sync::{Arc, Mutex};

    use crate::state::test_support::{
        NullCrfService, NullDomainModelService, NullMissionService, NullSdtmProjectService,
        NullTerminologyService,
    };
    use tower::ServiceExt;

    use apis::auth::{
        AuthApiError, AuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };

    /// Configurable mock for the credential-update handler. Each
    /// method stores the success variant and the failure variant
    /// separately; the `last_update_args` field captures the last
    /// request the handler forwarded so handler-translation tests
    /// can assert on it. `verify` is exercised by the `AuthClaims`
    /// extractor and yields a fixed claims value.
    ///
    /// `find_user_credential_by_code` and `remove_user_credential`
    /// remain in the trait surface (kept on the mock with
    /// `unimplemented!()`) — they are exercised by other test
    /// modules but not by these handler tests. `create_user_credential`
    /// was removed from the trait, so it no longer appears here.
    #[derive(Clone, Default)]
    struct MockAuth {
        update: Option<UserCredentialView>,
        update_err: Option<AuthApiError>,
        register: Option<RegisterUserResponse>,
        register_err: Option<AuthApiError>,
        verify_ok: bool,
        verify_role: Option<apis::user::Role>,
        verify_err: Option<AuthApiError>,

        last_update_args: Arc<Mutex<Option<UpdateUserCredentialRequest>>>,
        last_register_args: Arc<Mutex<Option<RegisterUserRequest>>>,
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
                role: self.verify_role.unwrap_or(apis::user::Role::Admin),
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
            unimplemented!("not exercised by the user-credential handlers")
        }
        async fn create_user_credential(
            &self,
            _req: CreateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn update_user_credential(
            &self,
            req: UpdateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            *self.last_update_args.lock().unwrap() = Some(req);
            if let Some(err) = self.update_err.clone() {
                return Err(err);
            }
            Ok(self.update.clone().expect("update result configured"))
        }
        async fn remove_user_credential(
            &self,
            _code: &str,
        ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
            unimplemented!("not exercised by the user-credential handlers")
        }
        async fn logout(&self, _req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
            unimplemented!()
        }
        async fn register_user(
            &self,
            req: RegisterUserRequest,
        ) -> Result<RegisterUserResponse, AuthApiError> {
            *self.last_register_args.lock().unwrap() = Some(req);
            if let Some(err) = self.register_err.clone() {
                return Err(err);
            }
            Ok(self.register.clone().expect("register result configured"))
        }
    }

    /// `UserService` lives on `AppState` but the credential handlers
    /// never use it. The trait is non-trivial (real impls need a
    /// `PgPool`) so this stub keeps the handler tests free of any
    /// user-service surface.
    #[derive(Clone)]
    struct NullUserService;

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
    }

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

    fn test_state(mock: MockAuth) -> AppState {
        AppState {
            auth: Arc::new(mock) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            crf: Arc::new(NullCrfService) as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            domain_model: Arc::new(NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(NullMissionService) as Arc<dyn apis::mission::MissionService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/api/auth/user-credential", post(register).patch(update))
            .with_state(state)
    }

    async fn read_json(response: axum::response::Response) -> (AxStatus, serde_json::Value) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    fn build_request(
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

    fn sample_credential(code: &str, token_version: u32) -> UserCredentialView {
        UserCredentialView {
            user_code: code.into(),
            password_hash: "argon2id$v=19$m=...$...".into(),
            token_version,
        }
    }

    fn sample_register_response() -> RegisterUserResponse {
        RegisterUserResponse {
            user_code: "u1".into(),
            user_name: "Alice".into(),
            role: apis::user::Role::General,
            active: false,
            domain_name: "example.com".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        }
    }

    fn empty_token() -> &'static str {
        "Bearer good"
    }

    // ---- update ----------------------------------------------------

    #[tokio::test]
    async fn update_returns_200_with_view_on_success() {
        let mock = MockAuth {
            verify_ok: true,
            update: Some(sample_credential("u1", 7)),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential",
                Some(r#"{"password":"new-secret"}"#.to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["user_code"], "u1");
        assert_eq!(body["password_hash"], "argon2id$v=19$m=...$...");
        assert_eq!(body["token_version"], 7);

        // The handler must read `user_code` from claims and pass the
        // raw `password` straight through to the auth usecase.
        let captured = mock.last_update_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.user_code, "u1");
        assert_eq!(captured.password.as_deref(), Some("new-secret"));
    }

    #[tokio::test]
    async fn update_with_empty_body_returns_unchanged_view() {
        // The apis trait permits a no-op update — body `{}` must
        // succeed and return the existing view. `user_code` is
        // derived from claims.
        let mock = MockAuth {
            verify_ok: true,
            update: Some(sample_credential("u1", 3)),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential",
                Some("{}".to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["token_version"], 3);

        let captured = mock.last_update_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.user_code, "u1");
        assert!(captured.password.is_none());
    }

    #[tokio::test]
    async fn update_maps_not_found_to_404() {
        let mock = MockAuth {
            verify_ok: true,
            update_err: Some(AuthApiError::NotFound),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential",
                Some(r#"{"password":"x"}"#.to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn update_without_authorization_returns_401() {
        let mock = MockAuth::default();
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential",
                Some(r#"{"password":"x"}"#.to_string()),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- register --------------------------------------------------

    fn register_body() -> String {
        serde_json::json!({
            "user_code": "u1",
            "user_name": "Alice",
            "domain_name": "example.com",
            "hostname": "host",
            "sid": "S-1-5",
            "password": "hunter2",
        })
        .to_string()
    }

    #[tokio::test]
    async fn register_returns_201_with_view_for_admin() {
        let mock = MockAuth {
            verify_ok: true,
            verify_role: Some(apis::user::Role::Admin),
            register: Some(sample_register_response()),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(register_body()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CREATED);
        assert_eq!(body["user_code"], "u1");
        assert_eq!(body["user_name"], "Alice");
        assert_eq!(body["role"], "general");
        assert_eq!(body["active"], false);
        assert_eq!(body["domain_name"], "example.com");

        let captured = mock.last_register_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.user_code, "u1");
        assert_eq!(captured.user_name, "Alice");
        assert_eq!(captured.domain_name, "example.com");
        assert_eq!(captured.hostname, "host");
        assert_eq!(captured.sid, "S-1-5");
        assert_eq!(captured.password, "hunter2");
    }

    #[tokio::test]
    async fn register_allows_root_role() {
        let mock = MockAuth {
            verify_ok: true,
            verify_role: Some(apis::user::Role::Root),
            register: Some(sample_register_response()),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(register_body()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::CREATED);
    }

    #[tokio::test]
    async fn register_returns_403_for_general_role() {
        let mock = MockAuth {
            verify_ok: true,
            verify_role: Some(apis::user::Role::General),
            register: Some(sample_register_response()),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(register_body()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");
        assert!(
            mock.last_register_args.lock().unwrap().is_none(),
            "general role must not call register_user"
        );
    }

    #[tokio::test]
    async fn register_maps_validation_to_400() {
        let mock = MockAuth {
            verify_ok: true,
            verify_role: Some(apis::user::Role::Admin),
            register_err: Some(AuthApiError::Validation("domain not allowed".into())),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(register_body()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
        assert_eq!(body["code"], "validation_failed");
    }

    #[tokio::test]
    async fn register_without_authorization_returns_401() {
        let mock = MockAuth::default();
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(register_body()),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
}
