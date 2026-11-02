use async_trait::async_trait;

use super::error::DomainError;

/// Narrow cross-crate port. Returns `Ok(())` if the project exists,
/// `Err(DomainError::ProjectNotFound)` if not, or
/// `Err(DomainError::Repository(msg))` for any other failure.
///
/// Existence-only on purpose: the usecase only needs an existence
/// check; carrying the full project view across the port would couple
/// this crate to the project crate's DTOs. Mirrors the minimal-surface
/// decision behind `sdtm_project::domain::ProjectLookup`.
#[async_trait]
pub trait ProjectLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
}
