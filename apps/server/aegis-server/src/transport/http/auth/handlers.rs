//! HTTP handlers for the auth flows.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from `dto`) into an apis DTO.
//! 2. Calls the corresponding [`AuthService`] method on `AppState`.
//! 3. Translates the apis response back into a wire DTO.
//!
//! `AuthApiError` is funnelled through [`ApiError::from`] so each
//! route returns `Result<Json<T>, ApiError>` and the error mapping
//! in `transport::http::error` does the rest.

use apis::auth::{
    LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, RefreshRequest,
};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto;
use crate::transport::http::error::ApiError;

/// `POST /api/auth/login` — exchange `(code, password)` for an
/// access + refresh token pair.
#[utoipa::path(
    post,
    path = "/login",
    tag = "auth",
    operation_id = "auth_login",
    request_body = dto::LoginRequest,
    responses(
        (status = 200, description = "Token pair minted", body = dto::TokenPairResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Invalid credentials", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "User is inactive", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository / signing failure", body = crate::transport::http::error::ErrorBody),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<dto::LoginRequest>,
) -> Result<Json<dto::TokenPairResponse>, ApiError> {
    let pair = state
        .auth
        .login_with_password(LoginWithPasswordRequest {
            code: req.code,
            password: req.password,
        })
        .await?;
    Ok(Json(dto::TokenPairResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    }))
}

/// `POST /api/auth/login-domain` — exchange `(code, domain_name,
/// hostname, sid)` for a token pair.
#[utoipa::path(
    post,
    path = "/login-domain",
    tag = "auth",
    operation_id = "auth_login_domain",
    request_body = dto::LoginDomainRequest,
    responses(
        (status = 200, description = "Token pair minted", body = dto::TokenPairResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "User is inactive", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository / signing failure", body = crate::transport::http::error::ErrorBody),
    ),
)]
pub async fn login_domain(
    State(state): State<AppState>,
    Json(req): Json<dto::LoginDomainRequest>,
) -> Result<Json<dto::TokenPairResponse>, ApiError> {
    let pair = state
        .auth
        .login_with_domain_user_info(LoginWithDomainUserInfoRequest {
            code: req.code,
            domain_name: req.domain_name,
            hostname: req.hostname,
            sid: req.sid,
        })
        .await?;
    Ok(Json(dto::TokenPairResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    }))
}

/// `POST /api/auth/refresh` — exchange a still-valid refresh token
/// for a fresh access token.
///
/// This route does **not** require a valid access token. The whole
/// point of the endpoint is to mint a new access token when the
/// current one has expired; rejecting expired access tokens here
/// would be self-defeating. The refresh token in the body is the
/// sole credential — `AuthService::refresh` validates it and
/// surfaces any failure as 401.
#[utoipa::path(
    post,
    path = "/refresh",
    tag = "auth",
    operation_id = "auth_refresh",
    request_body = dto::RefreshRequest,
    responses(
        (status = 200, description = "Fresh access token", body = dto::AccessTokenResponse),
        (status = 401, description = "Refresh token rejected", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository / signing failure", body = crate::transport::http::error::ErrorBody),
    ),
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<dto::RefreshRequest>,
) -> Result<Json<dto::AccessTokenResponse>, ApiError> {
    let response = state
        .auth
        .refresh(RefreshRequest {
            refresh_token: req.refresh_token,
        })
        .await?;
    Ok(Json(dto::AccessTokenResponse {
        access_token: response.access_token,
    }))
}

/// `POST /api/auth/logout` — invalidate the session identified by
/// `refresh_token`. Always returns `200 OK` with `{}` on success.
/// Requires a valid access token in `Authorization: Bearer <token>`.
#[utoipa::path(
    post,
    path = "/logout",
    tag = "auth",
    operation_id = "auth_logout",
    request_body = dto::LogoutRequest,
    responses(
        (status = 200, description = "Logged out", body = dto::LogoutResponse),
        (status = 401, description = "Missing / invalid access token, or refresh token rejected", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn logout(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::LogoutRequest>,
) -> Result<(StatusCode, Json<dto::LogoutResponse>), ApiError> {
    let _ = claims;
    state
        .auth
        .logout(LogoutRequest {
            refresh_token: req.refresh_token,
        })
        .await?;
    Ok((StatusCode::OK, Json(dto::LogoutResponse {})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use axum::routing::post;
    use std::sync::Arc;
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

    /// Mock `AuthService` whose login / refresh / logout methods
    /// return a preconfigured value. Each method stores the success
    /// variant and the failure variant separately. `verify` is
    /// exercised by the `AuthClaims` extractor on refresh / logout
    /// and returns either a successful claims value or a
    /// preconfigured error.
    #[derive(Clone, Default)]
    struct MockAuth {
        login_with_password: Option<TokenPair>,
        login_with_password_err: Option<AuthApiError>,
        login_with_domain_user_info: Option<TokenPair>,
        login_with_domain_user_info_err: Option<AuthApiError>,
        refresh: Option<RefreshResponse>,
        refresh_err: Option<AuthApiError>,
        logout_ok: bool,
        logout_err: Option<AuthApiError>,
        verify_ok: bool,
        verify_err: Option<AuthApiError>,
    }

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _req: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            if let Some(err) = self.login_with_password_err.clone() {
                return Err(err);
            }
            Ok(self
                .login_with_password
                .clone()
                .expect("login_with_password result configured"))
        }
        async fn login_with_domain_user_info(
            &self,
            _req: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> {
            if let Some(err) = self.login_with_domain_user_info_err.clone() {
                return Err(err);
            }
            Ok(self
                .login_with_domain_user_info
                .clone()
                .expect("login_with_domain_user_info result configured"))
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
            if let Some(err) = self.refresh_err.clone() {
                return Err(err);
            }
            Ok(self.refresh.clone().expect("refresh result configured"))
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
            if let Some(err) = self.logout_err.clone() {
                return Err(err);
            }
            assert!(self.logout_ok, "logout_ok must be set");
            Ok(LogoutResponse::default())
        }
        async fn register_user(
            &self,
            _req: RegisterUserRequest,
        ) -> Result<RegisterUserResponse, AuthApiError> {
            unimplemented!("not exercised by this handler")
        }
    }

    /// `UserService` is part of `AppState` but the auth handlers
    /// never use it. The trait is non-trivial (real impls need a
    /// `PgPool`) so this stub lets the handler tests avoid wiring
    /// any user-service surface.
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

    fn router(state: AppState) -> Router {
        Router::new()
            .route("/api/auth/login", post(login))
            .route("/api/auth/login-domain", post(login_domain))
            .route("/api/auth/refresh", post(refresh))
            .route("/api/auth/logout", post(logout))
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

    // ---- login ----------------------------------------------------

    #[tokio::test]
    async fn login_returns_token_pair_on_success() {
        let mock = MockAuth {
            login_with_password: Some(TokenPair {
                access_token: "ACCESS".into(),
                refresh_token: "REFRESH".into(),
            }),
            ..Default::default()
        };
        let app = router(test_state(mock));
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
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["access_token"], "ACCESS");
        assert_eq!(body["refresh_token"], "REFRESH");
    }

    #[tokio::test]
    async fn login_maps_invalid_credentials_to_401() {
        let mock = MockAuth {
            login_with_password_err: Some(AuthApiError::InvalidCredentials),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"u1","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "invalid_credentials");
    }

    // ---- login_domain --------------------------------------------

    #[tokio::test]
    async fn login_domain_returns_token_pair() {
        let mock = MockAuth {
            login_with_domain_user_info: Some(TokenPair {
                access_token: "ACCESS".into(),
                refresh_token: "REFRESH".into(),
            }),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login-domain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"code":"u1","domain_name":"D","hostname":"H","sid":"S"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["access_token"], "ACCESS");
    }

    #[tokio::test]
    async fn login_domain_maps_not_found_to_404() {
        let mock = MockAuth {
            login_with_domain_user_info_err: Some(AuthApiError::NotFound),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login-domain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"code":"u1","domain_name":"D","hostname":"H","sid":"S"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    // ---- refresh --------------------------------------------------

    #[tokio::test]
    async fn refresh_returns_access_token() {
        let mock = MockAuth {
            refresh: Some(RefreshResponse {
                access_token: "NEW".into(),
            }),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/refresh")
                    .header("authorization", "Bearer good-access")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"refresh_token":"r"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["access_token"], "NEW");
    }

    #[tokio::test]
    async fn refresh_with_expired_access_token_still_succeeds() {
        // The whole point of the refresh endpoint: an expired
        // access token must not be a barrier. The refresh token in
        // the body is the real credential. We configure the mock
        // to fail `verify` (mimicking an expired access token)
        // but still let `refresh` succeed — the handler must
        // never consult `verify` on this route.
        let mock = MockAuth {
            verify_err: Some(AuthApiError::Verification("expired".into())),
            refresh: Some(RefreshResponse {
                access_token: "NEW".into(),
            }),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/refresh")
                    .header("authorization", "Bearer expired-access")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"refresh_token":"r"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["access_token"], "NEW");
    }

    #[tokio::test]
    async fn refresh_without_authorization_header_succeeds() {
        // The refresh endpoint does not require an access token
        // at all. The body carries the refresh token that
        // authenticates the request.
        let mock = MockAuth {
            refresh: Some(RefreshResponse {
                access_token: "NEW".into(),
            }),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/refresh")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"refresh_token":"r"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["access_token"], "NEW");
    }

    #[tokio::test]
    async fn refresh_maps_refresh_token_failure_to_401() {
        // The refresh token itself is rejected by the auth usecase —
        // 401 with `token_verification_failed`, reached via the
        // handler body. The presence or absence of an access token
        // in the request has no effect on this route.
        let mock = MockAuth {
            refresh_err: Some(AuthApiError::Verification("refresh expired".into())),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/refresh")
                    .header("authorization", "Bearer good-access")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"refresh_token":"expired-refresh"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- logout ---------------------------------------------------

    #[tokio::test]
    async fn logout_returns_empty_object() {
        let mock = MockAuth {
            verify_ok: true,
            logout_ok: true,
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("authorization", "Bearer good-access")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"refresh_token":"r"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body, serde_json::json!({}));
    }

    #[tokio::test]
    async fn logout_maps_repository_to_500() {
        let mock = MockAuth {
            verify_ok: true,
            logout_err: Some(AuthApiError::Repository("oops".into())),
            ..Default::default()
        };
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("authorization", "Bearer good-access")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"refresh_token":"r"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "repository_error");
    }

    #[tokio::test]
    async fn logout_without_authorization_returns_401() {
        // No Authorization header — AuthClaims rejects the request
        // before the handler body runs.
        let mock = MockAuth::default();
        let app = router(test_state(mock));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"refresh_token":"r"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
}
