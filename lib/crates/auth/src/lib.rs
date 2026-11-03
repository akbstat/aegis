//! # auth crate
//!
//! Workspace library that implements the `apis::auth::AuthService` port.
//! Three DDD layers (`domain`, `usecase`, `adapter`) plus an
//! `Arc<dyn TokenVersionCache>` cache plumbed through the usecase.
//! Public consumers should `use auth::*;` (see the re-exports below)
//! rather than reach into the sub-modules.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::cache::in_memory::token_version::InMemoryTokenVersionCache;
pub use adapter::facade::internal::AuthServiceImpl;
pub use adapter::persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};
pub use adapter::service::user::UserServiceImpl;
pub use domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, TokenVersionCache,
    UserCredentials, UserCredentialsRepository, UserService, UserSummary,
};
pub use usecase::{
    AccessTokenView, AuthClaimsView, AuthUsecase, AuthUsecaseConfig, CreateUserCredential,
    FindUserCredential, LoginWithDomainUserInfo, LoginWithPassword, Logout, LogoutAck,
    RefreshAccessToken, RegisterUser, RegisteredUserView, RemoveUserCredential,
    RemoveUserCredentialAck, TokenPairView, UpdateUserCredential, UsecaseError, UserCredentialView,
    VerifyAccessToken,
};
