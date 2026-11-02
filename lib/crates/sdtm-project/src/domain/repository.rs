use async_trait::async_trait;

use super::error::DomainError;
use super::sdtm_project_domain::{
    SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainUpdate,
};
use super::sdtm_project_domain_override::{
    SdtmProjectDomainOverride, SdtmProjectDomainOverrideNew, SdtmProjectDomainOverrideUpdate,
};
use super::sdtm_project_variable::{
    SdtmProjectVariable, SdtmProjectVariableNew, SdtmProjectVariableUpdate,
};
use super::sdtm_project_variable_override::{
    SdtmProjectVariableOverride, SdtmProjectVariableOverrideNew, SdtmProjectVariableOverrideUpdate,
};
use super::sdtm_project_version::{
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionUpdate,
};

/// Outbound port for persistence of `SdtmProjectVersion` aggregates.
/// The only list path is scoped to a project (no bare `list()`).
#[async_trait]
pub trait SdtmProjectVersionRepository: Send + Sync {
    async fn create(&self, input: SdtmProjectVersionNew)
    -> Result<SdtmProjectVersion, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVersion, DomainError>;
    async fn list_by_project(
        &self,
        project_code: &str,
    ) -> Result<Vec<SdtmProjectVersion>, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectVersionUpdate,
    ) -> Result<SdtmProjectVersion, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmProjectDomain` aggregates.
/// The only list path is scoped to a version (no bare `list()`).
///
/// The read methods hydrate the domain's `domain_override` from
/// `sdtm_project_domain_overrides`; the write methods do not touch
/// that table — the override repository owns it.
#[async_trait]
pub trait SdtmProjectDomainRepository: Send + Sync {
    async fn create(&self, input: SdtmProjectDomainNew) -> Result<SdtmProjectDomain, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectDomain, DomainError>;
    async fn list_by_version(&self, version_id: i64)
    -> Result<Vec<SdtmProjectDomain>, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectDomainUpdate,
    ) -> Result<SdtmProjectDomain, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmProjectDomainOverride`
/// aggregates. This is the **only** writer of
/// `sdtm_project_domain_overrides`.
///
/// Every method is keyed on the parent `domain_id` — a 1:1
/// sub-resource's natural key — rather than on the override's own
/// surrogate id.
#[async_trait]
pub trait SdtmProjectDomainOverrideRepository: Send + Sync {
    async fn create(
        &self,
        input: SdtmProjectDomainOverrideNew,
    ) -> Result<SdtmProjectDomainOverride, DomainError>;
    async fn find_by_domain_id(
        &self,
        domain_id: i64,
    ) -> Result<SdtmProjectDomainOverride, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectDomainOverrideUpdate,
    ) -> Result<SdtmProjectDomainOverride, DomainError>;
    async fn delete(&self, domain_id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmProjectVariable` aggregates.
/// The only list path is scoped to a domain (no bare `list()`).
///
/// The read methods hydrate the variable's `variable_override` from
/// `sdtm_project_variable_overrides`; the write methods do not touch
/// that table — the override repository owns it.
#[async_trait]
pub trait SdtmProjectVariableRepository: Send + Sync {
    async fn create(
        &self,
        input: SdtmProjectVariableNew,
    ) -> Result<SdtmProjectVariable, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVariable, DomainError>;
    async fn list_by_domain(&self, domain_id: i64)
    -> Result<Vec<SdtmProjectVariable>, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectVariableUpdate,
    ) -> Result<SdtmProjectVariable, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmProjectVariableOverride`
/// aggregates. This is the **only** writer of
/// `sdtm_project_variable_overrides`.
///
/// Every method is keyed on the parent `variable_id` — a 1:1
/// sub-resource's natural key — rather than on the override's own
/// surrogate id.
#[async_trait]
pub trait SdtmProjectVariableOverrideRepository: Send + Sync {
    async fn create(
        &self,
        input: SdtmProjectVariableOverrideNew,
    ) -> Result<SdtmProjectVariableOverride, DomainError>;
    async fn find_by_variable_id(
        &self,
        variable_id: i64,
    ) -> Result<SdtmProjectVariableOverride, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectVariableOverrideUpdate,
    ) -> Result<SdtmProjectVariableOverride, DomainError>;
    async fn delete(&self, variable_id: i64) -> Result<(), DomainError>;
}
