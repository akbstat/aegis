//! `AuthClaims` request extractor.
//!
//! Pulls `Authorization: Bearer <token>` from the request, hands the
//! token to [`AuthService::verify`], and surfaces the resulting
//! [`apis::auth::AuthClaims`] to the handler. Failures (missing
//! header, malformed header, invalid token) all surface as
//! [`ApiError::Verification`], which renders as HTTP 401 with code
//! `token_verification_failed`.

use apis::auth::VerifyRequest;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::state::AppState;
use crate::transport::http::error::ApiError;

/// Authenticated identity recovered from the request's
/// `Authorization: Bearer <token>` header.
///
/// Use as a handler argument in any route that requires auth:
///
/// ```ignore
/// async fn whoami(claims: AuthClaims) -> Json<AuthClaimsResponse> {
///     Json(AuthClaimsResponse::from(claims.0))
/// }
/// ```
pub struct AuthClaims(pub apis::auth::AuthClaims);

impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(&parts.headers)?;
        let claims = state
            .auth
            .verify(VerifyRequest {
                access_token: token,
            })
            .await?;
        Ok(Self(claims))
    }
}

/// Accept only `Root` or `Admin` callers. Other roles — including
/// `General` — receive [`ApiError::Forbidden`], which renders as
/// `403 forbidden` with the message
/// `admin or root role required`.
///
/// Shared by every module that needs a write-role guard (project,
/// terminology, etc.) so the role policy lives in exactly one place.
pub(crate) fn require_admin_or_root(claims: &AuthClaims) -> Result<(), ApiError> {
    match claims.0.role {
        apis::user::Role::Root | apis::user::Role::Admin => Ok(()),
        apis::user::Role::General => Err(ApiError::Forbidden),
    }
}

/// Extract the bearer token from the `Authorization` header.
///
/// Returns `AuthApiError::Verification` (mapped to HTTP 401) for
/// any of: missing header, non-Bearer scheme, empty token.
fn bearer_token(headers: &axum::http::HeaderMap) -> Result<String, ApiError> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| {
            ApiError::from(apis::auth::AuthApiError::Verification(
                "missing Authorization header".into(),
            ))
        })?;
    let value = header.to_str().map_err(|_| {
        ApiError::from(apis::auth::AuthApiError::Verification(
            "non-ASCII Authorization header".into(),
        ))
    })?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .ok_or_else(|| {
            ApiError::from(apis::auth::AuthApiError::Verification(
                "Authorization header is not Bearer".into(),
            ))
        })?
        .trim();
    if token.is_empty() {
        return Err(ApiError::from(apis::auth::AuthApiError::Verification(
            "empty bearer token".into(),
        )));
    }
    Ok(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{HeaderMap, HeaderValue, Request, StatusCode as AxStatus};
    use axum::routing::get;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::state::test_support::{
        NullCrfService, NullDomainModelService, NullMissionService, NullSdtmProjectService,
        NullTerminologyService,
    };

    use apis::auth::{
        AuthApiError, AuthClaims as ApiAuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };
    use apis::user::Role;

    #[derive(Clone)]
    struct MockAuth;

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
        async fn verify(&self, req: VerifyRequest) -> Result<ApiAuthClaims, AuthApiError> {
            // Pretend token-shape: "good:<code>:<role>:<ver>".
            let parts: Vec<&str> = req.access_token.split(':').collect();
            if parts.first() != Some(&"good") || parts.len() != 4 {
                return Err(AuthApiError::Verification("bad token".into()));
            }
            let role = match parts[2] {
                "root" => Role::Root,
                "admin" => Role::Admin,
                _ => Role::General,
            };
            Ok(ApiAuthClaims {
                code: parts[1].to_string(),
                role,
                token_version: parts[3].parse().unwrap_or(0),
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

    fn test_state() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
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

    fn app(state: AppState) -> Router {
        async fn whoami(claims: AuthClaims) -> String {
            format!(
                "{}:{}:{}",
                claims.0.code, claims.0.role as u8, claims.0.token_version
            )
        }
        Router::new()
            .route("/whoami", get(whoami))
            .with_state(state)
    }

    // ---- bearer_token helper -------------------------------------------

    #[test]
    fn bearer_token_picks_up_valid_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer abc.def.ghi"),
        );
        let t = bearer_token(&headers).unwrap();
        assert_eq!(t, "abc.def.ghi");
    }

    #[test]
    fn bearer_token_lowercase_scheme_accepted() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("bearer xyz"),
        );
        let t = bearer_token(&headers).unwrap();
        assert_eq!(t, "xyz");
    }

    #[test]
    fn bearer_token_missing_header_rejected() {
        let headers = HeaderMap::new();
        let err = bearer_token(&headers).unwrap_err();
        assert_eq!(err.code(), "token_verification_failed");
    }

    #[test]
    fn bearer_token_non_bearer_scheme_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Basic dXNlcjpwYXNz"),
        );
        let err = bearer_token(&headers).unwrap_err();
        assert_eq!(err.code(), "token_verification_failed");
    }

    #[test]
    fn bearer_token_empty_after_scheme_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer "),
        );
        let err = bearer_token(&headers).unwrap_err();
        assert_eq!(err.code(), "token_verification_failed");
    }

    // ---- FromRequestParts integration ---------------------------------

    #[tokio::test]
    async fn extractor_returns_claims_for_valid_token() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .uri("/whoami")
                    .header("Authorization", "Bearer good:u1:admin:7")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert_eq!(&body[..], b"u1:1:7");
    }

    #[tokio::test]
    async fn extractor_returns_401_for_missing_header() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .uri("/whoami")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), 256)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "token_verification_failed");
    }

    #[tokio::test]
    async fn extractor_returns_401_for_bad_token() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .uri("/whoami")
                    .header("Authorization", "Bearer definitely-not-good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }
}
