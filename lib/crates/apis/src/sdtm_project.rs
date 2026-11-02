//! Outbound port for the per-project SDTM model tree.
//!
//! Mirrors the surface of `sdtm_project::usecase::SdtmProjectUsecase`
//! so adapters in any backend (in-memory, PostgreSQL, …) can adapt
//! their own types to the shared contract defined here. All supporting
//! DTOs (request shapes, view projections, enum re-declarations, and
//! [`SdtmProjectApiError`]) live alongside the trait so a single
//! `use apis::sdtm_project::*;` brings the whole contract into scope.
//!
//! The value objects below are **copies** of the `domain-model` ones
//! rather than shared types: the CDISC standard and a project's overlay
//! are allowed to drift independently, so the two features do not
//! share a type.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// SDTM domain category.
///
/// Mirrors `sdtm_project::DomainCategory`. The two enums are kept in
/// sync layer by layer — adapter implementations convert losslessly
/// via the matching variant constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

/// SDTM variable type.
///
/// Mirrors `sdtm_project::SdtmVariableType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

/// SDTM variable core.
///
/// Mirrors `sdtm_project::SdtmVariableCore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

/// SDTM variable role.
///
/// Mirrors `sdtm_project::SdtmRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

/// Error surface returned by every [`SdtmProjectService`] method.
///
/// Adapters translate backend-specific errors (e.g.
/// `sdtm_project::UsecaseError` / `sdtm_project::DomainError`) into
/// this type at the implementation boundary. The shape intentionally
/// combines validation, lookup, duplicate, and infrastructure concerns
/// into a single type so handlers can match exhaustively.
#[derive(Debug, Clone, Error)]
pub enum SdtmProjectApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("sdtm project version not found: {0}")]
    SdtmProjectVersionNotFound(i64),
    #[error("sdtm project domain not found: {0}")]
    SdtmProjectDomainNotFound(i64),
    #[error("sdtm project domain override not found: {0}")]
    SdtmProjectDomainOverrideNotFound(i64),
    #[error("sdtm project variable not found: {0}")]
    SdtmProjectVariableNotFound(i64),
    #[error("sdtm project variable override not found: {0}")]
    SdtmProjectVariableOverrideNotFound(i64),

    #[error("sdtm project version already exists: {project_code} / {version_name}")]
    DuplicateSdtmProjectVersion {
        project_code: String,
        version_name: String,
    },
    #[error("sdtm project domain already exists: version {version_id} / {domain_code}")]
    DuplicateSdtmProjectDomain {
        version_id: i64,
        domain_code: String,
    },
    #[error("sdtm project domain override already exists: domain {domain_id}")]
    DuplicateSdtmProjectDomainOverride { domain_id: i64 },
    #[error("sdtm project variable already exists: domain {domain_id} / {variable_code}")]
    DuplicateSdtmProjectVariable {
        domain_id: i64,
        variable_code: String,
    },
    #[error("sdtm project variable override already exists: variable {variable_id}")]
    DuplicateSdtmProjectVariableOverride { variable_id: i64 },

    #[error("referenced sdtm project version not found: {0}")]
    FkSdtmProjectVersionNotFound(i64),
    #[error("referenced sdtm project domain not found: {0}")]
    FkSdtmProjectDomainNotFound(i64),
    #[error("referenced sdtm project variable not found: {0}")]
    FkSdtmProjectVariableNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}

// ---- description DTOs ----

/// Localised description of a domain. Carried on
/// [`SdtmProjectDomainOverrideView`] and round-trips through the
/// backend as a single JSONB blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

/// Localised description of a variable. Carried on
/// [`SdtmProjectVariableOverrideView`] and round-trips through the
/// backend as a single JSONB blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

// ---- view projections ----

/// Safe projection of an `SdtmProjectVersion` aggregate — exactly what
/// adapters hand back to whatever consumes the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVersionView {
    pub id: i64,
    pub project_code: String,
    pub version_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `SdtmProjectDomain` aggregate. The nested
/// `domain_override` carries the domain's one override, when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainView {
    pub id: i64,
    pub version_id: i64,
    pub domain_code: String,
    pub key_variables: Vec<String>,
    pub domain_override: Option<SdtmProjectDomainOverrideView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `SdtmProjectDomainOverride` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainOverrideView {
    pub id: i64,
    pub domain_id: i64,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `SdtmProjectVariable` aggregate. The nested
/// `variable_override` carries the variable's one override, when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableView {
    pub id: i64,
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
    pub variable_override: Option<SdtmProjectVariableOverrideView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `SdtmProjectVariableOverride` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableOverrideView {
    pub id: i64,
    pub variable_id: i64,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- list wrappers ----

/// Wire-friendly wrapper around `Vec<SdtmProjectVersionView>`. List
/// endpoints return this so the response shape can grow (e.g. add
/// pagination cursors) without breaking every consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVersionList {
    pub versions: Vec<SdtmProjectVersionView>,
}

/// Wire-friendly wrapper around `Vec<SdtmProjectDomainView>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainList {
    pub domains: Vec<SdtmProjectDomainView>,
}

/// Wire-friendly wrapper around `Vec<SdtmProjectVariableView>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableList {
    pub variables: Vec<SdtmProjectVariableView>,
}

// ---- request DTOs ----

/// Input DTO for [`SdtmProjectService::create_version`].
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVersionRequest {
    pub project_code: String,
    pub version_name: String,
}

/// Input DTO for [`SdtmProjectService::update_version`]. Only
/// `version_name` is mutable — `project_code` is immutable so a
/// version cannot be moved between projects.
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmProjectVersionRequest {
    pub id: i64,
    pub version_name: Option<String>,
}

/// Input DTO for [`SdtmProjectService::create_domain`].
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectDomainRequest {
    pub version_id: i64,
    pub domain_code: String,
    pub key_variables: Vec<String>,
}

/// Input DTO for [`SdtmProjectService::update_domain`]. `key_variables`
/// preserves the caller's order; `Some(vec![])` clears the column.
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmProjectDomainRequest {
    pub id: i64,
    pub domain_code: Option<String>,
    pub key_variables: Option<Vec<String>>,
}

/// Input DTO for [`SdtmProjectService::create_domain_override`].
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectDomainOverrideRequest {
    pub domain_id: i64,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// Input DTO for [`SdtmProjectService::update_domain_override`]. Keyed
/// on the parent `domain_id` — a 1:1 sub-resource's natural key.
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmProjectDomainOverrideRequest {
    pub domain_id: i64,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

/// Input DTO for [`SdtmProjectService::create_variable`].
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVariableRequest {
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
}

/// Input DTO for [`SdtmProjectService::update_variable`].
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmProjectVariableRequest {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_sequence: Option<i64>,
}

/// Input DTO for [`SdtmProjectService::create_variable_override`].
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVariableOverrideRequest {
    pub variable_id: i64,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// Input DTO for [`SdtmProjectService::update_variable_override`].
/// `variable_role` uses `Option<Option<SdtmRole>>` so the caller can
/// distinguish "don't change" (outer `None`) from "clear the field"
/// (outer `Some(None)`).
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmProjectVariableOverrideRequest {
    pub variable_id: i64,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

/// Outbound port for the per-project SDTM model tree.
///
/// `Send + Sync` so a `Box<dyn SdtmProjectService>` can be shared
/// state in an async server (axum, tarpc, etc.). Object-safe: no
/// generic methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `sdtm_project::SdtmProjectUsecase`) into this contract, translating
/// between backend-specific DTOs / errors and the `apis` types defined
/// above.
#[async_trait]
pub trait SdtmProjectService: Send + Sync {
    // ---- SdtmProjectVersion ----

    /// Create a new version. Returns
    /// `DuplicateSdtmProjectVersion { .. }` if a version with that
    /// `(project_code, version_name)` pair already exists, or
    /// `ProjectNotFound` if the project does not exist.
    async fn create_version(
        &self,
        req: CreateSdtmProjectVersionRequest,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError>;

    /// Look up a version by its surrogate id. Returns
    /// `SdtmProjectVersionNotFound` if no such version exists.
    async fn get_version_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError>;

    /// List every version belonging to the given project. An unknown
    /// project code yields an empty list.
    async fn list_versions_by_project(
        &self,
        project_code: &str,
    ) -> Result<SdtmProjectVersionList, SdtmProjectApiError>;

    /// Apply the optional fields on `req` to the version identified by
    /// `req.id`. Returns `SdtmProjectVersionNotFound` if no such
    /// version exists.
    async fn update_version(
        &self,
        req: UpdateSdtmProjectVersionRequest,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError>;

    /// Hard delete the version identified by `id`. Cascades to its
    /// domains, overrides, variables and variable overrides at the
    /// backend.
    async fn delete_version(&self, id: i64) -> Result<(), SdtmProjectApiError>;

    // ---- SdtmProjectDomain ----

    /// Create a new `(version_id, domain_code)` domain. Returns
    /// `DuplicateSdtmProjectDomain { .. }` if a domain with that pair
    /// already exists, or `FkSdtmProjectVersionNotFound` if the parent
    /// version is missing.
    async fn create_domain(
        &self,
        req: CreateSdtmProjectDomainRequest,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError>;

    /// Look up a domain by its surrogate id. Returns
    /// `SdtmProjectDomainNotFound` if no such domain exists.
    async fn get_domain_by_id(&self, id: i64)
    -> Result<SdtmProjectDomainView, SdtmProjectApiError>;

    /// List every domain attached to the given version, ordered by
    /// id ASC.
    async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<SdtmProjectDomainList, SdtmProjectApiError>;

    /// Apply the optional fields on `req` to the domain identified by
    /// `req.id`. Returns `SdtmProjectDomainNotFound` if no such domain
    /// exists.
    async fn update_domain(
        &self,
        req: UpdateSdtmProjectDomainRequest,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError>;

    /// Hard delete the domain identified by `id`. Cascades to its
    /// override and its variables at the backend.
    async fn delete_domain(&self, id: i64) -> Result<(), SdtmProjectApiError>;

    // ---- SdtmProjectDomainOverride ----

    /// Create the domain's one override. Returns
    /// `DuplicateSdtmProjectDomainOverride { .. }` if the domain
    /// already holds one.
    async fn create_domain_override(
        &self,
        req: CreateSdtmProjectDomainOverrideRequest,
    ) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError>;

    /// Fetch the domain's override.
    ///
    /// The override is **optional**: a domain without one is a normal
    /// state, not a failure, so a missing override is `None` rather
    /// than an error.
    async fn get_domain_override(
        &self,
        domain_id: i64,
    ) -> Result<Option<SdtmProjectDomainOverrideView>, SdtmProjectApiError>;

    /// Apply the optional fields on `req` to the override of the
    /// domain identified by `req.domain_id`.
    async fn update_domain_override(
        &self,
        req: UpdateSdtmProjectDomainOverrideRequest,
    ) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError>;

    /// Delete the domain's override.
    async fn delete_domain_override(&self, domain_id: i64) -> Result<(), SdtmProjectApiError>;

    // ---- SdtmProjectVariable ----

    /// Create a new `(domain_id, variable_code)` variable. Returns
    /// `DuplicateSdtmProjectVariable { .. }` if a variable with that
    /// pair already exists, or `FkSdtmProjectDomainNotFound` if the
    /// parent domain is missing.
    async fn create_variable(
        &self,
        req: CreateSdtmProjectVariableRequest,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError>;

    /// Look up a variable by its surrogate id. Returns
    /// `SdtmProjectVariableNotFound` if no such variable exists.
    async fn get_variable_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError>;

    /// List every variable attached to the given domain, ordered by
    /// `variable_sequence ASC, id ASC`.
    async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<SdtmProjectVariableList, SdtmProjectApiError>;

    /// Apply the optional fields on `req` to the variable identified
    /// by `req.id`.
    async fn update_variable(
        &self,
        req: UpdateSdtmProjectVariableRequest,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError>;

    /// Hard delete the variable identified by `id`.
    async fn delete_variable(&self, id: i64) -> Result<(), SdtmProjectApiError>;

    // ---- SdtmProjectVariableOverride ----

    /// Create the variable's one override. Returns
    /// `DuplicateSdtmProjectVariableOverride { .. }` if the variable
    /// already holds one.
    async fn create_variable_override(
        &self,
        req: CreateSdtmProjectVariableOverrideRequest,
    ) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError>;

    /// Fetch the variable's override.
    ///
    /// The override is **optional**: a variable without one is a
    /// normal state, not a failure, so a missing override is `None`
    /// rather than an error.
    async fn get_variable_override(
        &self,
        variable_id: i64,
    ) -> Result<Option<SdtmProjectVariableOverrideView>, SdtmProjectApiError>;

    /// Apply the optional fields on `req` to the override of the
    /// variable identified by `req.variable_id`.
    async fn update_variable_override(
        &self,
        req: UpdateSdtmProjectVariableOverrideRequest,
    ) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError>;

    /// Delete the variable's override.
    async fn delete_variable_override(&self, variable_id: i64) -> Result<(), SdtmProjectApiError>;
}
