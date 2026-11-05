//! # project crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for the `Project` aggregate (with a `ProjectConfiguration { language,
//! tags }` JSONB payload) and an async `ProjectUsecase` that
//! orchestrates them and adapts to the `apis::project::ProjectService`
//! port.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::internal::ProjectServiceImpl;
pub use adapter::persistence::postgres::ProjectBranchRepo;
pub use adapter::persistence::postgres::ProjectRepo;
pub use adapter::service::user::UserServiceImpl;
pub use domain::{
    DomainError, ModelVersion, Project, ProjectBranch, ProjectBranchNew, ProjectBranchRepository,
    ProjectBranchUpdate, ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectNew,
    ProjectRepository, ProjectTag, ProjectUpdate, RoleType, TeamType, UserService, UserSummary,
};
pub use usecase::{
    CreateProject, CreateProjectBranch, ProjectBranchUsecase, ProjectBranchView, ProjectMemberView,
    ProjectUsecase, ProjectUsecaseConfig, ProjectView, TagView, UpdateProject, UpdateProjectBranch,
    UsecaseError, UserSummaryView,
};
