use async_trait::async_trait;

use super::error::DomainError;
use super::structure::{Structure, StructureFilter, StructureNew};

/// Outbound port for persistence of `Structure` aggregates.
///
/// `find_by_id` reports a missing row as `DomainError::StructureNotFound`
/// rather than the generic `NotFound`, so the wire contract can tell a
/// client that the structure it asked for is gone.
#[async_trait]
pub trait StructureRepository: Send + Sync {
    async fn create(&self, input: StructureNew) -> Result<Structure, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<Structure, DomainError>;
    /// Every structure matching the filter, ordered by `id ASC`.
    async fn list(&self, filter: StructureFilter) -> Result<Vec<Structure>, DomainError>;
    async fn rename(&self, id: i64, name: &str) -> Result<Structure, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    /// How many project-level structures the given project already has.
    async fn count_project_structures(&self, project_code: &str) -> Result<i64, DomainError>;
    /// Whether any project-level structure still points at this template.
    async fn is_template_referenced(&self, template_id: i64) -> Result<bool, DomainError>;
}
