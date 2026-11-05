//! PostgreSQL-backed implementation of `ProjectRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the user crate.
//! `ProjectRepo::create` / `update` open a transaction so the project
//! row, the `project_members` rows, and the JSONB `tags` payload land
//! atomically.
//!
//! `row` is `pub(crate)` and is NOT re-exported at the crate root.

pub(crate) mod branch_repo;
pub(crate) mod project_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use branch_repo::ProjectBranchRepo;
pub use project_repo::ProjectRepo;
