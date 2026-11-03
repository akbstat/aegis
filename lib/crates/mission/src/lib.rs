//! `mission` workspace crate.
//!
//! Hosts the `Mission`, `Assignee`, and `MissionIssue`
//! aggregates, their ports, the PostgreSQL-backed persistence
//! adapters, the cross-crate `ProjectLookup` / `UserLookup`
//! adapters, the usecase layers that orchestrate them with
//! project-leader authorization, and the facade that
//! adapts both usecases to `apis::mission::MissionService`.
//!
//! Layered architecture:
//!
//! ```text
//! mission crate
//! └── adapter
//!     ├── facade                  (MissionServiceImpl<M, A, P, U, I>)
//!     ├── persistence             (MissionRepoPg, AssigneeRepoPg, IssueRepoPg)
//!     └── service                 (ProjectLookupImpl, UserLookupImpl)
//! usecase
//!     ├── MissionUsecase<M, A, P, U>
//!     └── MissionIssueUsecase<M, A, P, I>
//! domain
//!     └── Mission, Assignee, MissionIssue, IssueComment,
//!         MissionKind, MissionRole, IssueState,
//!         MissionRepository, AssigneeRepository,
//!         MissionIssueRepository,
//!         ProjectLookup, UserLookup,
//!         DomainError
//! ```

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::internal::service::MissionServiceImpl;
pub use adapter::persistence::postgres::{AssigneeRepo, IssueRepo, MissionRepo};
pub use adapter::service::project::ProjectLookupImpl;
pub use adapter::service::user::UserLookupImpl;
pub use domain::{
    Assignee, AssigneeRepository, DomainError, IssueComment, IssueState, Mission, MissionIssue,
    MissionIssueNew, MissionIssueRepository, MissionKind, MissionRepository, MissionRole,
    ProjectLookup, UserLookup,
};
pub use usecase::{
    CreateIssue, IssueCommentView, IssueView, MissionIssueUsecase, MissionIssueUsecaseConfig,
    MissionUsecase, MissionUsecaseConfig, UsecaseError,
};
