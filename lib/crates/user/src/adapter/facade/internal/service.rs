//! `UserService` adapter over the `UserRepository` port.
//!
//! Hosts `UserServiceImpl<R>` — the implementation of
//! `apis::user::UserService` that adapts `user::UserUsecase` to the
//! API contract. Behaviour is exercised by `tests`, which wires the
//! adapter on top of an in-memory `UserRepository` so no live
//! PostgreSQL connection is required.

use async_trait::async_trait;

use apis::user::Role as ApiRole;
use apis::user::{CreateUserRequest, UpdateUserRequest, UserApiError, UserService, UserView};

use crate::domain::{DomainError, Role, UserRepository};
use crate::usecase::{CreateUser, UpdateUser, UsecaseError, UserUsecase};

/// Adapter that implements [`UserService`] on top of a
/// [`UserUsecase`].
///
/// Generic over the persistence port (`R: UserRepository`) so the
/// adapter can be exercised against in-memory fakes in tests and
/// against the PostgreSQL-backed [`UserRepo`](crate::UserRepo) in
/// production. Translation between `apis::user::*` and
/// `user::usecase::*` happens inline in each trait method.
pub struct UserServiceImpl<R: UserRepository> {
    usecase: UserUsecase<R>,
}

impl<R: UserRepository> UserServiceImpl<R> {
    /// Build a new `UserServiceImpl` wrapping the supplied usecase.
    pub fn new(usecase: UserUsecase<R>) -> Self {
        Self { usecase }
    }
}

/// Map the API's `Role` into the domain's `Role`. The two enums
/// share the same three variants; the match is exhaustive and the
/// compiler enforces it on either side.
fn to_internal_role(r: ApiRole) -> Role {
    match r {
        ApiRole::Root => Role::Root,
        ApiRole::Admin => Role::Admin,
        ApiRole::General => Role::General,
    }
}

/// Inverse of [`to_internal_role`].
fn from_internal_role(r: Role) -> ApiRole {
    match r {
        Role::Root => ApiRole::Root,
        Role::Admin => ApiRole::Admin,
        Role::General => ApiRole::General,
    }
}

/// Project the usecase-layer `UserView` into the API-layer
/// `UserView`. Field-for-field because the two structs are kept
/// identical by design.
fn user_view_from_internal(view: crate::usecase::UserView) -> UserView {
    UserView {
        id: view.id,
        code: view.code,
        name: view.name,
        role: from_internal_role(view.role),
        active: view.active,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

/// Translate a [`UsecaseError`] into the API's [`UserApiError`].
///
/// `UsecaseError::Validation` only ever wraps the validation-only
/// `DomainError` variants; the `unreachable!` arm in the
/// `Repository` branch documents that fact and would fire if a
/// future change ever broke the invariant.
impl From<UsecaseError> for UserApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => UserApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => UserApiError::NotFound,
                DomainError::DuplicateCode(code) => UserApiError::DuplicateCode(code),
                DomainError::Repository(msg) => UserApiError::Repository(msg),
                DomainError::EmptyCode | DomainError::EmptyName | DomainError::InvalidRole(_) => {
                    unreachable!(
                        "domain validation errors are only produced as UsecaseError::Validation"
                    )
                }
            },
        }
    }
}

#[async_trait]
impl<R: UserRepository> UserService for UserServiceImpl<R> {
    async fn create(&self, req: CreateUserRequest) -> Result<UserView, UserApiError> {
        let cmd = CreateUser {
            code: req.code,
            name: req.name,
            role: to_internal_role(req.role),
            active: req.active,
        };
        let view = self.usecase.create(cmd).await?;
        Ok(user_view_from_internal(view))
    }

    async fn get_by_id(&self, id: i32) -> Result<UserView, UserApiError> {
        let view = self.usecase.get_by_id(id).await?;
        Ok(user_view_from_internal(view))
    }

    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError> {
        let view = self.usecase.get_by_code(code).await?;
        Ok(user_view_from_internal(view))
    }

    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        let views = self.usecase.list().await?;
        Ok(views.into_iter().map(user_view_from_internal).collect())
    }

    async fn update(&self, req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        let cmd = UpdateUser {
            id: req.id,
            code: req.code,
            name: req.name,
            role: req.role.map(to_internal_role),
            active: req.active,
        };
        let view = self.usecase.update(cmd).await?;
        Ok(user_view_from_internal(view))
    }
}
