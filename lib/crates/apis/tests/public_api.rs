//! Public-API compile test for the `apis` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and
//! the in-crate type names so a regression in `user.rs` is caught
//! at `cargo test -p apis` time.

use apis::user::{CreateUserRequest, Role, UpdateUserRequest, UserApiError, UserService, UserView};

/// Every public type in `apis::user` is nameable from the test.
#[test]
fn public_types_are_nameable() {
    fn assert_role(_: Role) {}
    fn assert_view(_: UserView) {}
    fn assert_create(_: CreateUserRequest) {}
    fn assert_update(_: UpdateUserRequest) {}
    fn assert_err(_: UserApiError) {}

    // `Role` is constructible from its variants.
    assert_role(Role::General);
    // `UserView` is constructible field-by-field.
    assert_view(UserView {
        id: 1,
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        active: true,
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    });
    // `CreateUserRequest` has no `password` field — this is the
    // shape adapters receive from outside the backend.
    assert_create(CreateUserRequest {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        active: true,
    });
    assert_update(UpdateUserRequest {
        id: 1,
        ..Default::default()
    });

    // Touch the error type to keep it from being dead-code-eliminated
    // by the test build's analysis.
    let _: UserApiError = UserApiError::NotFound;
    let _ = assert_err;
}

/// Minimal in-test implementation used to lock the trait's
/// signature, object-safety, and `Send + Sync` bounds. Each method
/// returns `todo!()` because the test only exercises the type
/// system — never the runtime behavior.
struct FakeUserService;

#[async_trait::async_trait]
impl UserService for FakeUserService {
    async fn create(&self, _req: CreateUserRequest) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn get_by_id(&self, _id: i32) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn get_by_code(&self, _code: &str) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        todo!()
    }
    async fn update(&self, _req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        todo!()
    }
}

/// `UserService` is object-safe: it can be held behind a `Box<dyn …>`.
#[test]
fn user_service_is_object_safe() {
    let _boxed: Box<dyn UserService> = Box::new(FakeUserService);
}

/// `UserService` requires `Send + Sync`, so a `Box<dyn UserService>`
/// is itself `Send + Sync` and can be shared state in an async server.
#[test]
fn user_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn UserService>>();
    assert_send_sync::<&FakeUserService>();
}

// -- apis::auth ---------------------------------------------------------

use apis::auth::{
    AuthApiError, AuthClaims, AuthService, CreateUserCredentialRequest,
    LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
    RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
    RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
    VerifyRequest,
};

/// Every public type in `apis::auth` is nameable from the test.
#[test]
fn auth_public_types_are_nameable() {
    fn assert_pair(_: TokenPair) {}
    fn assert_claims(_: AuthClaims) {}
    fn assert_login_pw(_: LoginWithPasswordRequest) {}
    fn assert_login_domain(_: LoginWithDomainUserInfoRequest) {}
    fn assert_logout_req(_: LogoutRequest) {}
    fn assert_logout_res(_: LogoutResponse) {}
    fn assert_verify_req(_: VerifyRequest) {}
    fn assert_refresh_req(_: RefreshRequest) {}
    fn assert_refresh_res(_: RefreshResponse) {}
    fn assert_create_cred(_: CreateUserCredentialRequest) {}
    fn assert_update_cred(_: UpdateUserCredentialRequest) {}
    fn assert_cred_view(_: UserCredentialView) {}
    fn assert_remove_cred_res(_: RemoveUserCredentialResponse) {}
    fn assert_register_req(_: RegisterUserRequest) {}
    fn assert_register_res(_: RegisterUserResponse) {}
    fn assert_err(_: AuthApiError) {}

    // `TokenPair` is constructible field-by-field.
    assert_pair(TokenPair {
        access_token: "a".into(),
        refresh_token: "r".into(),
    });
    // `AuthClaims` is constructible field-by-field; `role` reuses
    // `apis::user::Role`.
    assert_claims(AuthClaims {
        code: "u1".into(),
        role: apis::user::Role::General,
        token_version: 0,
    });
    // Every request DTO owns its string field — that is the shape
    // adapters receive from outside the backend.
    assert_login_pw(LoginWithPasswordRequest {
        code: "u1".into(),
        password: "p".into(),
    });
    assert_login_domain(LoginWithDomainUserInfoRequest {
        code: "u1".into(),
        domain_name: "d".into(),
        hostname: "h".into(),
        sid: "s".into(),
    });
    assert_logout_req(LogoutRequest {
        refresh_token: "r".into(),
    });
    assert_verify_req(VerifyRequest {
        access_token: "a".into(),
    });
    assert_refresh_req(RefreshRequest {
        refresh_token: "r".into(),
    });
    // Every response DTO is constructible field-by-field.
    assert_logout_res(LogoutResponse {});
    assert_refresh_res(RefreshResponse {
        access_token: "a".into(),
    });
    assert_create_cred(CreateUserCredentialRequest {
        user_code: "u1".into(),
        password: "p".into(),
    });
    assert_update_cred(UpdateUserCredentialRequest {
        user_code: "u1".into(),
        ..Default::default()
    });
    assert_cred_view(UserCredentialView {
        user_code: "u1".into(),
        password_hash: "h".into(),
        token_version: 0,
    });
    assert_remove_cred_res(RemoveUserCredentialResponse {});
    assert_register_req(RegisterUserRequest {
        user_code: "u1".into(),
        user_name: "Alice".into(),
        domain_name: "example.com".into(),
        hostname: "host".into(),
        sid: "S-1-5".into(),
        password: "p".into(),
    });
    assert_register_res(RegisterUserResponse {
        user_code: "u1".into(),
        user_name: "Alice".into(),
        role: apis::user::Role::General,
        active: false,
        domain_name: "example.com".into(),
        hostname: "host".into(),
        sid: "S-1-5".into(),
    });

    // Touch every variant of the error type to keep it from being
    // dead-code-eliminated by the test build's analysis.
    let _: AuthApiError = AuthApiError::Validation("".into());
    let _: AuthApiError = AuthApiError::NotFound;
    let _: AuthApiError = AuthApiError::Inactive;
    let _: AuthApiError = AuthApiError::InvalidCredentials;
    let _: AuthApiError = AuthApiError::Signing("".into());
    let _: AuthApiError = AuthApiError::Verification("".into());
    let _: AuthApiError = AuthApiError::Repository("".into());
    let _: AuthApiError = AuthApiError::DuplicateCode("".into());
    let _ = assert_err;
}

/// Minimal in-test implementation used to lock the trait's signature,
/// object-safety, and `Send + Sync` bounds. Each method returns
/// `todo!()` because the test only exercises the type system — never
/// the runtime behavior.
struct FakeAuthService;

#[async_trait::async_trait]
impl AuthService for FakeAuthService {
    async fn login_with_password(
        &self,
        _req: LoginWithPasswordRequest,
    ) -> Result<TokenPair, AuthApiError> {
        todo!()
    }
    async fn login_with_domain_user_info(
        &self,
        _req: LoginWithDomainUserInfoRequest,
    ) -> Result<TokenPair, AuthApiError> {
        todo!()
    }
    async fn logout(&self, _req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
        todo!()
    }
    async fn verify(&self, _req: VerifyRequest) -> Result<AuthClaims, AuthApiError> {
        todo!()
    }
    async fn refresh(&self, _req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
        todo!()
    }
    async fn find_user_credential_by_code(
        &self,
        _code: &str,
    ) -> Result<UserCredentialView, AuthApiError> {
        todo!()
    }
    async fn create_user_credential(
        &self,
        _req: CreateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError> {
        todo!()
    }
    async fn update_user_credential(
        &self,
        _req: UpdateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError> {
        todo!()
    }
    async fn remove_user_credential(
        &self,
        _code: &str,
    ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
        todo!()
    }
    async fn register_user(
        &self,
        _req: RegisterUserRequest,
    ) -> Result<RegisterUserResponse, AuthApiError> {
        todo!()
    }
}

/// `AuthService` is object-safe: it can be held behind a `Box<dyn …>`.
#[test]
fn auth_service_is_object_safe() {
    let _boxed: Box<dyn AuthService> = Box::new(FakeAuthService);
}

/// `AuthService` requires `Send + Sync`, so a `Box<dyn AuthService>`
/// is itself `Send + Sync` and can be shared state in an async server.
#[test]
fn auth_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn AuthService>>();
    assert_send_sync::<&FakeAuthService>();
}

// ---- sdtm_project --------------------------------------------------------

use apis::sdtm_project::{
    CreateSdtmProjectDomainOverrideRequest, CreateSdtmProjectDomainRequest,
    CreateSdtmProjectVariableOverrideRequest, CreateSdtmProjectVariableRequest,
    CreateSdtmProjectVersionRequest, DomainCategory, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmProjectApiError, SdtmProjectDomainList,
    SdtmProjectDomainOverrideView, SdtmProjectDomainView, SdtmProjectService,
    SdtmProjectVariableList, SdtmProjectVariableOverrideView, SdtmProjectVariableView,
    SdtmProjectVersionList, SdtmProjectVersionView, SdtmRole, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableType,
    UpdateSdtmProjectDomainOverrideRequest, UpdateSdtmProjectDomainRequest,
    UpdateSdtmProjectVariableOverrideRequest, UpdateSdtmProjectVariableRequest,
    UpdateSdtmProjectVersionRequest,
};

/// Every public type in `apis::sdtm_project` is nameable from the test.
#[test]
fn sdtm_project_public_types_are_nameable() {
    fn assert_category(_: DomainCategory) {}
    fn assert_var_type(_: SdtmVariableType) {}
    fn assert_var_core(_: SdtmVariableCore) {}
    fn assert_role(_: SdtmRole) {}
    fn assert_domain_desc(_: SdtmDomainDescription) {}
    fn assert_domain_desc_detail(_: SdtmDomainDescriptionDetail) {}
    fn assert_var_desc(_: SdtmVariableDescription) {}
    fn assert_var_desc_detail(_: SdtmVariableDescriptionDetail) {}
    fn assert_version_view(_: SdtmProjectVersionView) {}
    fn assert_domain_view(_: SdtmProjectDomainView) {}
    fn assert_domain_override_view(_: SdtmProjectDomainOverrideView) {}
    fn assert_variable_view(_: SdtmProjectVariableView) {}
    fn assert_variable_override_view(_: SdtmProjectVariableOverrideView) {}
    fn assert_version_list(_: SdtmProjectVersionList) {}
    fn assert_domain_list(_: SdtmProjectDomainList) {}
    fn assert_variable_list(_: SdtmProjectVariableList) {}
    fn assert_err(_: SdtmProjectApiError) {}
    fn assert_create_version(_: CreateSdtmProjectVersionRequest) {}
    fn assert_update_version(_: UpdateSdtmProjectVersionRequest) {}
    fn assert_create_domain(_: CreateSdtmProjectDomainRequest) {}
    fn assert_update_domain(_: UpdateSdtmProjectDomainRequest) {}
    fn assert_create_domain_override(_: CreateSdtmProjectDomainOverrideRequest) {}
    fn assert_update_domain_override(_: UpdateSdtmProjectDomainOverrideRequest) {}
    fn assert_create_variable(_: CreateSdtmProjectVariableRequest) {}
    fn assert_update_variable(_: UpdateSdtmProjectVariableRequest) {}
    fn assert_create_variable_override(_: CreateSdtmProjectVariableOverrideRequest) {}
    fn assert_update_variable_override(_: UpdateSdtmProjectVariableOverrideRequest) {}

    assert_category(DomainCategory::Events);
    assert_var_type(SdtmVariableType::Character);
    assert_var_core(SdtmVariableCore::Req);
    assert_role(SdtmRole::Topic);
    assert_domain_desc(SdtmDomainDescription {
        lang: "en".into(),
        details: SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure: "One record per AE".into(),
        },
    });
    assert_domain_desc_detail(SdtmDomainDescriptionDetail {
        description: "Adverse events".into(),
        structure: "One record per AE".into(),
    });
    assert_var_desc(SdtmVariableDescription {
        lang: "en".into(),
        details: SdtmVariableDescriptionDetail {
            label: "Adverse Event Term".into(),
        },
    });
    assert_var_desc_detail(SdtmVariableDescriptionDetail {
        label: "Adverse Event Term".into(),
    });

    let ts = chrono::DateTime::from_timestamp(0, 0).unwrap();
    assert_version_view(SdtmProjectVersionView {
        id: 1,
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
        created_at: ts,
        updated_at: ts,
    });
    assert_domain_view(SdtmProjectDomainView {
        id: 2,
        version_id: 1,
        domain_code: "AE".into(),
        key_variables: vec!["AETERM".into()],
        domain_override: None,
        created_at: ts,
        updated_at: ts,
    });
    assert_domain_override_view(SdtmProjectDomainOverrideView {
        id: 3,
        domain_id: 2,
        category: DomainCategory::Events,
        descriptions: Vec::new(),
        created_at: ts,
        updated_at: ts,
    });
    assert_variable_view(SdtmProjectVariableView {
        id: 4,
        domain_id: 2,
        variable_code: "AETERM".into(),
        variable_sequence: 11,
        variable_override: None,
        created_at: ts,
        updated_at: ts,
    });
    assert_variable_override_view(SdtmProjectVariableOverrideView {
        id: 5,
        variable_id: 4,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        descriptions: Vec::new(),
        created_at: ts,
        updated_at: ts,
    });
    assert_version_list(SdtmProjectVersionList {
        versions: Vec::new(),
    });
    assert_domain_list(SdtmProjectDomainList {
        domains: Vec::new(),
    });
    assert_variable_list(SdtmProjectVariableList {
        variables: Vec::new(),
    });

    assert_create_version(CreateSdtmProjectVersionRequest {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    });
    assert_update_version(UpdateSdtmProjectVersionRequest {
        id: 1,
        version_name: None,
    });
    assert_create_domain(CreateSdtmProjectDomainRequest {
        version_id: 1,
        domain_code: "AE".into(),
        key_variables: vec!["AETERM".into()],
    });
    assert_update_domain(UpdateSdtmProjectDomainRequest {
        id: 1,
        domain_code: None,
        key_variables: None,
    });
    assert_create_domain_override(CreateSdtmProjectDomainOverrideRequest {
        domain_id: 1,
        category: DomainCategory::Events,
        descriptions: Vec::new(),
    });
    assert_update_domain_override(UpdateSdtmProjectDomainOverrideRequest {
        domain_id: 1,
        category: None,
        descriptions: None,
    });
    assert_create_variable(CreateSdtmProjectVariableRequest {
        domain_id: 1,
        variable_code: "AETERM".into(),
        variable_sequence: 11,
    });
    assert_update_variable(UpdateSdtmProjectVariableRequest {
        id: 1,
        variable_code: None,
        variable_sequence: None,
    });
    assert_create_variable_override(CreateSdtmProjectVariableOverrideRequest {
        variable_id: 1,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        descriptions: Vec::new(),
    });
    assert_update_variable_override(UpdateSdtmProjectVariableOverrideRequest {
        variable_id: 1,
        variable_type: None,
        variable_core: None,
        variable_role: None,
        descriptions: None,
    });

    // Touch the error type to keep it from being dead-code-eliminated
    // by the test build's analysis.
    assert_err(SdtmProjectApiError::NotFound);
}

/// `SdtmProjectService` is object-safe: it can be held behind a
/// `Box<dyn …>`.
#[test]
fn sdtm_project_service_is_object_safe() {
    let _boxed: Option<Box<dyn SdtmProjectService>> = None;
}

/// `SdtmProjectService` requires `Send + Sync`, so a
/// `Box<dyn SdtmProjectService>` is itself `Send + Sync` and can be
/// shared state in an async server.
#[test]
fn sdtm_project_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn SdtmProjectService>>();
}

// ---- domain_model --------------------------------------------------------

use apis::domain_model::{
    CreateSdtmVariableRequest, SdtmVariableControlled, SdtmVariableView, UpdateSdtmVariableRequest,
};

/// Every public type in `apis::domain_model` is nameable from the test.
#[test]
fn domain_model_public_types_are_nameable() {
    fn assert_controlled(_: SdtmVariableControlled) {}
    fn assert_view(_: SdtmVariableView) {}
    fn assert_create(_: CreateSdtmVariableRequest) {}
    fn assert_update(_: UpdateSdtmVariableRequest) {}

    assert_controlled(SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    });
    assert_view(SdtmVariableView {
        id: 1,
        domain_id: 2,
        name: "AESEV".into(),
        variable_controlled: Some(SdtmVariableControlled {
            code: Some(vec!["C66731".into()]),
            format: Some(vec!["Codelist".into()]),
        }),
        variable_type: apis::domain_model::SdtmVariableType::Character,
        variable_core: apis::domain_model::SdtmVariableCore::Req,
        variable_role: Some(apis::domain_model::SdtmRole::Topic),
        variable_sequence: 11,
        descriptions: vec![],
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    });
    assert_create(CreateSdtmVariableRequest {
        domain_id: 2,
        name: "AESEV".into(),
        variable_controlled: None,
        variable_type: apis::domain_model::SdtmVariableType::Character,
        variable_core: apis::domain_model::SdtmVariableCore::Req,
        variable_role: None,
        variable_sequence: 11,
        descriptions: vec![],
    });
    assert_update(UpdateSdtmVariableRequest::default());
}

/// The wire shape of the controlled-terminology value omits the lists
/// that are not set, so a client sees `{"code": [...]}` rather than
/// `{"code": [...], "format": null}`.
#[test]
fn domain_model_variable_controlled_serialises_only_the_lists_that_are_set() {
    let vc = SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    };
    let json = serde_json::to_value(&vc).unwrap();
    assert_eq!(json, serde_json::json!({ "code": ["C66731"] }));
}

// ---- project ----------------------------------------------------------

use apis::project::{
    CreateProjectBranchRequest, ProjectApiError, ProjectBranchView, ProjectService,
    UpdateProjectBranchRequest,
};

/// Every new branch type in `apis::project` is nameable from the test.
#[test]
fn project_branch_public_types_are_nameable() {
    fn assert_view(_: ProjectBranchView) {}
    fn assert_create(_: CreateProjectBranchRequest) {}
    fn assert_update(_: UpdateProjectBranchRequest) {}
    fn assert_err(_: ProjectApiError) {}

    assert_view(ProjectBranchView {
        id: 1,
        project_id: 2,
        branch: "2026Q1".into(),
        active: true,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    });
    assert_create(CreateProjectBranchRequest {
        project_id: 2,
        branch: "2026Q1".into(),
        active: false,
    });
    assert_update(UpdateProjectBranchRequest {
        id: 1,
        project_id: 2,
        branch: Some("2026Q2".into()),
        active: Some(true),
    });
    assert_err(ProjectApiError::DuplicateBranch("2026Q1".into()));
    assert_err(ProjectApiError::BranchNotFound(7));
}

/// The branch methods do not break the trait's object safety: it can
/// still be held behind a `Box<dyn …>`.
#[test]
fn project_service_with_branch_methods_is_object_safe() {
    let _boxed: Option<Box<dyn ProjectService>> = None;
}

/// The branch methods do not break the trait's `Send + Sync` bound.
#[test]
fn project_service_with_branch_methods_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn ProjectService>>();
}
