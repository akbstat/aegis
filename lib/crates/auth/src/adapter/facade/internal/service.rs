use async_trait::async_trait;

use apis::auth::{
    AuthApiError, AuthClaims, AuthService, CreateUserCredentialRequest,
    LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
    RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
    RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
    VerifyRequest,
};
use apis::user::Role as ApiRole;

use crate::domain::{DomainError, DomainIdentityRepository, UserCredentialsRepository};
use crate::usecase::{
    AccessTokenView, AuthClaimsView, AuthUsecase, CreateUserCredential, FindUserCredential,
    LoginWithDomainUserInfo, LoginWithPassword, Logout as LogoutCmd, RefreshAccessToken,
    RegisterUser, RegisteredUserView, RemoveUserCredential as RemoveUserCredentialCmd,
    RemoveUserCredentialAck, TokenPairView, UpdateUserCredential, UsecaseError,
    UserCredentialView as UserCredentialViewUsecase, VerifyAccessToken,
};

pub struct AuthServiceImpl<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    usecase: AuthUsecase<R, D>,
}

impl<R: UserCredentialsRepository, D: DomainIdentityRepository> AuthServiceImpl<R, D> {
    pub fn new(usecase: AuthUsecase<R, D>) -> Self {
        Self { usecase }
    }
}

fn to_api_role(r: crate::domain::Role) -> ApiRole {
    match r {
        crate::domain::Role::Root => ApiRole::Root,
        crate::domain::Role::Admin => ApiRole::Admin,
        crate::domain::Role::General => ApiRole::General,
    }
}

/// Project a usecase `UserCredentialView` into the apis view. The two
/// shapes are identical; the indirection keeps the apis DTO distinct
/// from the usecase's view.
fn credential_view_to_api(v: UserCredentialViewUsecase) -> UserCredentialView {
    UserCredentialView {
        user_code: v.code,
        password_hash: v.password_hash,
        token_version: v.token_version,
    }
}

fn map_error(err: UsecaseError) -> AuthApiError {
    match err {
        UsecaseError::Validation(d) => AuthApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => AuthApiError::NotFound,
            DomainError::Inactive => AuthApiError::Inactive,
            DomainError::InvalidCredentials => AuthApiError::InvalidCredentials,
            DomainError::DuplicateCode(code) => AuthApiError::DuplicateCode(code),
            DomainError::Repository(msg) => AuthApiError::Repository(msg),
            DomainError::DomainNotAllowed(msg) => AuthApiError::Validation(msg),
            DomainError::EmptyCode
            | DomainError::EmptyPasswordHash
            | DomainError::InvalidRole(_) => AuthApiError::Repository(d.to_string()),
        },
        UsecaseError::Verification(msg) => AuthApiError::Verification(msg),
    }
}

#[async_trait]
impl<R: UserCredentialsRepository, D: DomainIdentityRepository> AuthService
    for AuthServiceImpl<R, D>
{
    async fn login_with_password(
        &self,
        req: LoginWithPasswordRequest,
    ) -> Result<TokenPair, AuthApiError> {
        let view: TokenPairView = self
            .usecase
            .login_with_password(LoginWithPassword {
                code: req.code,
                password: req.password,
            })
            .await
            .map_err(map_error)?;
        Ok(TokenPair {
            access_token: view.access_token,
            refresh_token: view.refresh_token,
        })
    }

    async fn login_with_domain_user_info(
        &self,
        req: LoginWithDomainUserInfoRequest,
    ) -> Result<TokenPair, AuthApiError> {
        let view: TokenPairView = self
            .usecase
            .login_with_domain_user_info(LoginWithDomainUserInfo {
                code: req.code,
                domain_name: req.domain_name,
                hostname: req.hostname,
                sid: req.sid,
            })
            .await
            .map_err(map_error)?;
        Ok(TokenPair {
            access_token: view.access_token,
            refresh_token: view.refresh_token,
        })
    }

    async fn logout(&self, req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
        self.usecase
            .logout(LogoutCmd {
                refresh_token: req.refresh_token,
            })
            .await
            .map_err(map_error)?;
        Ok(LogoutResponse {})
    }

    async fn verify(&self, req: VerifyRequest) -> Result<AuthClaims, AuthApiError> {
        let view: AuthClaimsView = self
            .usecase
            .verify(VerifyAccessToken {
                access_token: req.access_token,
            })
            .await
            .map_err(map_error)?;
        Ok(AuthClaims {
            code: view.code,
            role: to_api_role(view.role),
            token_version: view.token_version,
        })
    }

    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
        let view: AccessTokenView = self
            .usecase
            .refresh(RefreshAccessToken {
                refresh_token: req.refresh_token,
            })
            .await
            .map_err(map_error)?;
        Ok(RefreshResponse {
            access_token: view.access_token,
        })
    }

    async fn find_user_credential_by_code(
        &self,
        code: &str,
    ) -> Result<UserCredentialView, AuthApiError> {
        let view = self
            .usecase
            .find_user_credential(FindUserCredential {
                code: code.to_string(),
            })
            .await
            .map_err(map_error)?;
        Ok(credential_view_to_api(view))
    }

    async fn create_user_credential(
        &self,
        req: CreateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError> {
        let view = self
            .usecase
            .create_user_credential(CreateUserCredential {
                code: req.user_code,
                password: req.password,
            })
            .await
            .map_err(map_error)?;
        Ok(credential_view_to_api(view))
    }

    async fn update_user_credential(
        &self,
        req: UpdateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError> {
        let view = self
            .usecase
            .update_user_credential(UpdateUserCredential {
                code: req.user_code,
                password: req.password,
            })
            .await
            .map_err(map_error)?;
        Ok(credential_view_to_api(view))
    }

    async fn remove_user_credential(
        &self,
        code: &str,
    ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
        let _ack: RemoveUserCredentialAck = self
            .usecase
            .remove_user_credential(RemoveUserCredentialCmd {
                code: code.to_string(),
            })
            .await
            .map_err(map_error)?;
        Ok(RemoveUserCredentialResponse {})
    }

    async fn register_user(
        &self,
        req: RegisterUserRequest,
    ) -> Result<RegisterUserResponse, AuthApiError> {
        let view: RegisteredUserView = self
            .usecase
            .register_user(RegisterUser {
                user_code: req.user_code,
                user_name: req.user_name,
                domain_name: req.domain_name,
                hostname: req.hostname,
                sid: req.sid,
                password: req.password,
            })
            .await
            .map_err(map_error)?;
        Ok(RegisterUserResponse {
            user_code: view.user_code,
            user_name: view.user_name,
            role: to_api_role(view.role),
            active: view.active,
            domain_name: view.domain_name,
            hostname: view.hostname,
            sid: view.sid,
        })
    }
}
