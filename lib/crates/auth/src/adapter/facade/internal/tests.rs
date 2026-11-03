//! Unit tests for `AuthServiceImpl`.
//!
//! Wires the adapter on top of `MockUserCredentialsRepo` +
//! `MockDomainIdentityRepo` + `FakeUserService` so the public-facing
//! `AuthService` surface is exercised without PostgreSQL.

use std::time::Duration;

use apis::auth::{
    AuthApiError, AuthService, LoginWithDomainUserInfoRequest, LoginWithPasswordRequest,
    LogoutRequest, RefreshRequest, RegisterUserRequest, VerifyRequest,
};
use apis::user::Role as ApiRole;

use crate::domain::Role;
use crate::usecase::tests::{
    FakeUserService, MockDomainIdentityRepo, MockUserCredentialsRepo, hash_password,
};

use super::service::AuthServiceImpl;

fn make_service(
    creds: MockUserCredentialsRepo,
    ids: MockDomainIdentityRepo,
    users: FakeUserService,
) -> AuthServiceImpl<MockUserCredentialsRepo, MockDomainIdentityRepo> {
    let cfg = crate::usecase::AuthUsecaseConfig {
        credentials: creds,
        identities: ids,
        user_service: std::sync::Arc::new(users),
        cache: std::sync::Arc::new(crate::InMemoryTokenVersionCache::new()),
        signing_key: b"0123456789abcdef0123456789abcdef".to_vec(),
        access_ttl: Duration::from_secs(60),
        refresh_ttl: Duration::from_secs(3600),
        allow_domains: vec!["example.com".into()],
    };
    AuthServiceImpl::new(crate::usecase::AuthUsecase::new(cfg))
}

#[tokio::test]
async fn service_impl_can_be_constructed() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let _svc = make_service(creds, ids, users);
}

#[tokio::test]
async fn login_with_password_returns_token_pair_for_valid_credentials() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_with_password_returns_invalid_credentials_for_wrong_password() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "WRONG".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::InvalidCredentials));
}

#[tokio::test]
async fn login_with_password_returns_inactive_when_user_is_disabled() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, false);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::Inactive));
}

#[tokio::test]
async fn login_with_domain_user_info_returns_not_found_for_unmatched_triple() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_domain_user_info(LoginWithDomainUserInfoRequest {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::NotFound));
}

#[tokio::test]
async fn verify_returns_claims_for_freshly_minted_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 9);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Root, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let claims = svc
        .verify(VerifyRequest {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds");
    assert_eq!(claims.code, "u1");
    assert_eq!(claims.role, ApiRole::Root);
    assert_eq!(claims.token_version, 9);
}

#[tokio::test]
async fn refresh_returns_new_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let new = svc
        .refresh(RefreshRequest {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("refresh succeeds");
    assert!(!new.access_token.is_empty());
}

#[tokio::test]
async fn logout_returns_empty_response() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let ack = svc
        .logout(LogoutRequest {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("logout succeeds");
    assert_eq!(ack, apis::auth::LogoutResponse {});
}

#[tokio::test]
async fn find_user_credential_returns_view_for_known_code() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", "hash", 7);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);

    let view = svc
        .find_user_credential_by_code("u1")
        .await
        .expect("find succeeds");
    assert_eq!(view.user_code, "u1");
    assert_eq!(view.password_hash, "hash");
    assert_eq!(view.token_version, 7);
}

#[tokio::test]
async fn find_user_credential_returns_not_found_for_unknown_code() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);

    let err = svc.find_user_credential_by_code("ghost").await.unwrap_err();
    assert!(matches!(err, AuthApiError::NotFound));
}

#[tokio::test]
async fn create_user_credential_hashes_raw_password_before_persisting() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);

    let created = svc
        .create_user_credential(apis::auth::CreateUserCredentialRequest {
            user_code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("create succeeds");
    assert_eq!(created.token_version, 0);
    assert_ne!(
        created.password_hash, "hunter2",
        "raw password must not round-trip into the view"
    );

    let fetched = svc
        .find_user_credential_by_code("u1")
        .await
        .expect("find succeeds");
    assert_eq!(fetched.user_code, "u1");
    assert_eq!(fetched.password_hash, created.password_hash);
}

#[tokio::test]
async fn update_user_credential_hashes_raw_password() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", "old", 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);

    let view = svc
        .update_user_credential(apis::auth::UpdateUserCredentialRequest {
            user_code: "u1".into(),
            password: Some("new".into()),
        })
        .await
        .expect("update succeeds");
    assert_ne!(
        view.password_hash, "new",
        "raw password must not round-trip into the view"
    );
}

#[tokio::test]
async fn remove_user_credential_returns_empty_response() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", "hash", 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);

    let ack = svc
        .remove_user_credential("u1")
        .await
        .expect("remove succeeds");
    assert_eq!(ack, apis::auth::RemoveUserCredentialResponse {});

    let err = svc.find_user_credential_by_code("u1").await.unwrap_err();
    assert!(matches!(err, AuthApiError::NotFound));
}

#[tokio::test]
async fn register_user_returns_view_and_maps_disallowed_domain_to_validation_error() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);

    let view = svc
        .register_user(RegisterUserRequest {
            user_code: "u1".into(),
            user_name: "Alice".into(),
            domain_name: "example.com".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("registration succeeds");
    assert_eq!(view.user_code, "u1");
    assert_eq!(view.user_name, "Alice");
    assert_eq!(view.role, ApiRole::General);
    assert!(!view.active);
    assert_eq!(view.domain_name, "example.com");
    assert_eq!(view.hostname, "host");
    assert_eq!(view.sid, "S-1-5");

    let err = svc
        .register_user(RegisterUserRequest {
            user_code: "u2".into(),
            user_name: "Bob".into(),
            domain_name: "evil.test".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
            password: "hunter2".into(),
        })
        .await
        .expect_err("disallowed domain maps to validation");
    assert!(matches!(err, AuthApiError::Validation(_)));
}

#[tokio::test]
async fn auth_service_impl_is_object_safe() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);
    let _boxed: Box<dyn AuthService> = Box::new(svc);
}

#[tokio::test]
async fn auth_service_impl_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);
    assert_send_sync::<AuthServiceImpl<MockUserCredentialsRepo, MockDomainIdentityRepo>>();
    assert_send_sync::<Box<dyn AuthService>>();
    let _ = svc;
}
