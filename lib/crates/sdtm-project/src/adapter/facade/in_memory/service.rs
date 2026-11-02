//! `SdtmProjectService` adapter backed by in-memory repositories.
//!
//! Hosts `SdtmProjectServiceImpl<V, D, DO, Va, VO, P>` — the
//! implementation of `apis::sdtm_project::SdtmProjectService` that
//! adapts `sdtm_project::SdtmProjectUsecase` to the API contract.
//! Translation between `apis::sdtm_project::*` and
//! `sdtm_project::usecase::*` happens inline in each trait method.

use std::sync::Arc;

use async_trait::async_trait;

use apis::sdtm_project::{
    CreateSdtmProjectDomainOverrideRequest, CreateSdtmProjectDomainRequest,
    CreateSdtmProjectVariableOverrideRequest, CreateSdtmProjectVariableRequest,
    CreateSdtmProjectVersionRequest, DomainCategory as ApiCategory,
    SdtmDomainDescription as ApiDomainDescription,
    SdtmDomainDescriptionDetail as ApiDomainDescriptionDetail, SdtmProjectApiError,
    SdtmProjectDomainList, SdtmProjectDomainOverrideView, SdtmProjectDomainView,
    SdtmProjectService, SdtmProjectVariableList, SdtmProjectVariableOverrideView,
    SdtmProjectVariableView, SdtmProjectVersionList, SdtmProjectVersionView,
    SdtmRole as ApiSdtmRole, SdtmVariableCore as ApiSdtmVariableCore,
    SdtmVariableDescription as ApiVariableDescription,
    SdtmVariableDescriptionDetail as ApiVariableDescriptionDetail,
    SdtmVariableType as ApiSdtmVariableType, UpdateSdtmProjectDomainOverrideRequest,
    UpdateSdtmProjectDomainRequest, UpdateSdtmProjectVariableOverrideRequest,
    UpdateSdtmProjectVariableRequest, UpdateSdtmProjectVersionRequest,
};

use crate::domain::{
    DomainCategory, DomainError, ProjectLookup, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmProjectDomainOverrideRepository, SdtmProjectDomainRepository,
    SdtmProjectVariableOverrideRepository, SdtmProjectVariableRepository,
    SdtmProjectVersionRepository, SdtmRole, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableType,
};
use crate::usecase::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion,
    SdtmProjectDomainOverrideView as InternalDomainOverrideView,
    SdtmProjectDomainView as InternalDomainView, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
    SdtmProjectVariableOverrideView as InternalVariableOverrideView,
    SdtmProjectVariableView as InternalVariableView, SdtmProjectVersionView as InternalVersionView,
    UpdateSdtmProjectDomain, UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable,
    UpdateSdtmProjectVariableOverride, UpdateSdtmProjectVersion, UsecaseError,
};

/// Adapter that implements [`SdtmProjectService`] on top of a
/// [`SdtmProjectUsecase`].
///
/// Generic over the five persistence ports plus the project lookup so
/// the adapter can be exercised against in-memory fakes in tests and
/// against the PostgreSQL-backed repositories in production.
pub struct SdtmProjectServiceImpl<
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
> {
    usecase: SdtmProjectUsecase<V, D, DO, Va, VO, P>,
}

impl<V, D, DO, Va, VO, P> SdtmProjectServiceImpl<V, D, DO, Va, VO, P>
where
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
{
    /// Build a new `SdtmProjectServiceImpl` wrapping the supplied
    /// usecase.
    pub fn new(usecase: SdtmProjectUsecase<V, D, DO, Va, VO, P>) -> Self {
        Self { usecase }
    }

    /// Build a new `SdtmProjectServiceImpl` from the five repositories
    /// and the project lookup directly. Mirrors the
    /// [`SdtmProjectUsecase::new`] constructor shape so callers who
    /// already hold the repositories do not need to assemble a
    /// [`SdtmProjectUsecaseConfig`] first.
    pub fn from_repos(
        version_repo: V,
        domain_repo: D,
        domain_override_repo: DO,
        variable_repo: Va,
        variable_override_repo: VO,
        projects: Arc<P>,
    ) -> Self {
        Self::new(SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
            version_repo,
            domain_repo,
            domain_override_repo,
            variable_repo,
            variable_override_repo,
            projects,
        }))
    }
}

// ---- enum mappers (api <-> domain) ----

fn to_internal_category(c: ApiCategory) -> DomainCategory {
    match c {
        ApiCategory::SpecialPurpose => DomainCategory::SpecialPurpose,
        ApiCategory::Interventions => DomainCategory::Interventions,
        ApiCategory::Events => DomainCategory::Events,
        ApiCategory::Findings => DomainCategory::Findings,
        ApiCategory::TrialDesign => DomainCategory::TrialDesign,
        ApiCategory::Relationships => DomainCategory::Relationships,
        ApiCategory::StudyReference => DomainCategory::StudyReference,
    }
}

fn from_internal_category(c: DomainCategory) -> ApiCategory {
    match c {
        DomainCategory::SpecialPurpose => ApiCategory::SpecialPurpose,
        DomainCategory::Interventions => ApiCategory::Interventions,
        DomainCategory::Events => ApiCategory::Events,
        DomainCategory::Findings => ApiCategory::Findings,
        DomainCategory::TrialDesign => ApiCategory::TrialDesign,
        DomainCategory::Relationships => ApiCategory::Relationships,
        DomainCategory::StudyReference => ApiCategory::StudyReference,
    }
}

fn to_internal_variable_type(t: ApiSdtmVariableType) -> SdtmVariableType {
    match t {
        ApiSdtmVariableType::Numeric => SdtmVariableType::Numeric,
        ApiSdtmVariableType::Character => SdtmVariableType::Character,
    }
}

fn from_internal_variable_type(t: SdtmVariableType) -> ApiSdtmVariableType {
    match t {
        SdtmVariableType::Numeric => ApiSdtmVariableType::Numeric,
        SdtmVariableType::Character => ApiSdtmVariableType::Character,
    }
}

fn to_internal_variable_core(c: ApiSdtmVariableCore) -> SdtmVariableCore {
    match c {
        ApiSdtmVariableCore::Req => SdtmVariableCore::Req,
        ApiSdtmVariableCore::Exp => SdtmVariableCore::Exp,
        ApiSdtmVariableCore::Perm => SdtmVariableCore::Perm,
        ApiSdtmVariableCore::Supp => SdtmVariableCore::Supp,
    }
}

fn from_internal_variable_core(c: SdtmVariableCore) -> ApiSdtmVariableCore {
    match c {
        SdtmVariableCore::Req => ApiSdtmVariableCore::Req,
        SdtmVariableCore::Exp => ApiSdtmVariableCore::Exp,
        SdtmVariableCore::Perm => ApiSdtmVariableCore::Perm,
        SdtmVariableCore::Supp => ApiSdtmVariableCore::Supp,
    }
}

fn to_internal_role(r: ApiSdtmRole) -> SdtmRole {
    match r {
        ApiSdtmRole::Identifier => SdtmRole::Identifier,
        ApiSdtmRole::Topic => SdtmRole::Topic,
        ApiSdtmRole::Timing => SdtmRole::Timing,
        ApiSdtmRole::RecordQualifier => SdtmRole::RecordQualifier,
        ApiSdtmRole::SynonymQualifier => SdtmRole::SynonymQualifier,
        ApiSdtmRole::VariableQualifier => SdtmRole::VariableQualifier,
        ApiSdtmRole::GroupingQualifier => SdtmRole::GroupingQualifier,
        ApiSdtmRole::Rule => SdtmRole::Rule,
    }
}

fn from_internal_role(r: SdtmRole) -> ApiSdtmRole {
    match r {
        SdtmRole::Identifier => ApiSdtmRole::Identifier,
        SdtmRole::Topic => ApiSdtmRole::Topic,
        SdtmRole::Timing => ApiSdtmRole::Timing,
        SdtmRole::RecordQualifier => ApiSdtmRole::RecordQualifier,
        SdtmRole::SynonymQualifier => ApiSdtmRole::SynonymQualifier,
        SdtmRole::VariableQualifier => ApiSdtmRole::VariableQualifier,
        SdtmRole::GroupingQualifier => ApiSdtmRole::GroupingQualifier,
        SdtmRole::Rule => ApiSdtmRole::Rule,
    }
}

// ---- description mappers ----

fn domain_descriptions_from_api(v: Vec<ApiDomainDescription>) -> Vec<SdtmDomainDescription> {
    v.into_iter()
        .map(|d| SdtmDomainDescription {
            lang: d.lang,
            details: SdtmDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        })
        .collect()
}

fn domain_descriptions_to_api(v: Vec<SdtmDomainDescription>) -> Vec<ApiDomainDescription> {
    v.into_iter()
        .map(|d| ApiDomainDescription {
            lang: d.lang,
            details: ApiDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        })
        .collect()
}

fn variable_descriptions_from_api(v: Vec<ApiVariableDescription>) -> Vec<SdtmVariableDescription> {
    v.into_iter()
        .map(|d| SdtmVariableDescription {
            lang: d.lang,
            details: SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        })
        .collect()
}

fn variable_descriptions_to_api(v: Vec<SdtmVariableDescription>) -> Vec<ApiVariableDescription> {
    v.into_iter()
        .map(|d| ApiVariableDescription {
            lang: d.lang,
            details: ApiVariableDescriptionDetail {
                label: d.details.label,
            },
        })
        .collect()
}

// ---- view mappers ----

fn version_view_from_internal(view: InternalVersionView) -> SdtmProjectVersionView {
    SdtmProjectVersionView {
        id: view.id,
        project_code: view.project_code,
        version_name: view.version_name,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn domain_view_from_internal(view: InternalDomainView) -> SdtmProjectDomainView {
    SdtmProjectDomainView {
        id: view.id,
        version_id: view.version_id,
        domain_code: view.domain_code,
        key_variables: view.key_variables,
        domain_override: view.domain_override.map(domain_override_view_from_internal),
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn domain_override_view_from_internal(
    view: InternalDomainOverrideView,
) -> SdtmProjectDomainOverrideView {
    SdtmProjectDomainOverrideView {
        id: view.id,
        domain_id: view.domain_id,
        category: from_internal_category(view.category),
        descriptions: domain_descriptions_to_api(view.descriptions),
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn variable_view_from_internal(view: InternalVariableView) -> SdtmProjectVariableView {
    SdtmProjectVariableView {
        id: view.id,
        domain_id: view.domain_id,
        variable_code: view.variable_code,
        variable_sequence: view.variable_sequence,
        variable_override: view
            .variable_override
            .map(variable_override_view_from_internal),
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn variable_override_view_from_internal(
    view: InternalVariableOverrideView,
) -> SdtmProjectVariableOverrideView {
    SdtmProjectVariableOverrideView {
        id: view.id,
        variable_id: view.variable_id,
        variable_type: from_internal_variable_type(view.variable_type),
        variable_core: from_internal_variable_core(view.variable_core),
        variable_role: view.variable_role.map(from_internal_role),
        descriptions: variable_descriptions_to_api(view.descriptions),
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

// ---- error mapper ----

/// Translate a [`UsecaseError`] into the API's [`SdtmProjectApiError`].
///
/// `UsecaseError::Validation` only ever wraps the validation-only
/// `DomainError` variants. The validation-only variants can still
/// reach the `Repository` branch when a stored row carries a value the
/// schema's CHECK constraint did not catch, so that branch maps them
/// to `Repository` rather than panicking.
impl From<UsecaseError> for SdtmProjectApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => SdtmProjectApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => SdtmProjectApiError::NotFound,
                DomainError::ProjectNotFound(code) => SdtmProjectApiError::ProjectNotFound(code),
                DomainError::SdtmProjectVersionNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectVersionNotFound(id)
                }
                DomainError::SdtmProjectDomainNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectDomainNotFound(id)
                }
                DomainError::SdtmProjectDomainOverrideNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectDomainOverrideNotFound(id)
                }
                DomainError::SdtmProjectVariableNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectVariableNotFound(id)
                }
                DomainError::SdtmProjectVariableOverrideNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectVariableOverrideNotFound(id)
                }
                DomainError::DuplicateSdtmProjectVersion {
                    project_code,
                    version_name,
                } => SdtmProjectApiError::DuplicateSdtmProjectVersion {
                    project_code,
                    version_name,
                },
                DomainError::DuplicateSdtmProjectDomain {
                    version_id,
                    domain_code,
                } => SdtmProjectApiError::DuplicateSdtmProjectDomain {
                    version_id,
                    domain_code,
                },
                DomainError::DuplicateSdtmProjectDomainOverride { domain_id } => {
                    SdtmProjectApiError::DuplicateSdtmProjectDomainOverride { domain_id }
                }
                DomainError::DuplicateSdtmProjectVariable {
                    domain_id,
                    variable_code,
                } => SdtmProjectApiError::DuplicateSdtmProjectVariable {
                    domain_id,
                    variable_code,
                },
                DomainError::DuplicateSdtmProjectVariableOverride { variable_id } => {
                    SdtmProjectApiError::DuplicateSdtmProjectVariableOverride { variable_id }
                }
                DomainError::FkSdtmProjectVersionNotFound(id) => {
                    SdtmProjectApiError::FkSdtmProjectVersionNotFound(id)
                }
                DomainError::FkSdtmProjectDomainNotFound(id) => {
                    SdtmProjectApiError::FkSdtmProjectDomainNotFound(id)
                }
                DomainError::FkSdtmProjectVariableNotFound(id) => {
                    SdtmProjectApiError::FkSdtmProjectVariableNotFound(id)
                }
                DomainError::EmptyProjectCode
                | DomainError::EmptyVersionName
                | DomainError::EmptyDomainCode
                | DomainError::EmptyVariableCode
                | DomainError::EmptyKeyVariable
                | DomainError::InvalidDomainCategory(_)
                | DomainError::InvalidVariableType(_)
                | DomainError::InvalidVariableCore(_)
                | DomainError::InvalidVariableRole(_) => {
                    SdtmProjectApiError::Repository(domain.to_string())
                }
                // A repository-level failure is already an
                // infrastructure error: pass the message straight
                // through.
                DomainError::Repository(msg) => SdtmProjectApiError::Repository(msg),
            },
        }
    }
}

#[async_trait]
impl<V, D, DO, Va, VO, P> SdtmProjectService for SdtmProjectServiceImpl<V, D, DO, Va, VO, P>
where
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
{
    // ---- SdtmProjectVersion ----

    async fn create_version(
        &self,
        req: CreateSdtmProjectVersionRequest,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectVersion {
            project_code: req.project_code,
            version_name: req.version_name,
        };
        let view = self.usecase.create_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn get_version_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError> {
        let view = self.usecase.get_version_by_id(id).await?;
        Ok(version_view_from_internal(view))
    }

    async fn list_versions_by_project(
        &self,
        project_code: &str,
    ) -> Result<SdtmProjectVersionList, SdtmProjectApiError> {
        let views = self.usecase.list_versions_by_project(project_code).await?;
        Ok(SdtmProjectVersionList {
            versions: views.into_iter().map(version_view_from_internal).collect(),
        })
    }

    async fn update_version(
        &self,
        req: UpdateSdtmProjectVersionRequest,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectVersion {
            id: req.id,
            version_name: req.version_name,
        };
        let view = self.usecase.update_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn delete_version(&self, id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_version(id).await?;
        Ok(())
    }

    // ---- SdtmProjectDomain ----

    async fn create_domain(
        &self,
        req: CreateSdtmProjectDomainRequest,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectDomain {
            version_id: req.version_id,
            domain_code: req.domain_code,
            key_variables: req.key_variables,
        };
        let view = self.usecase.create_domain(cmd).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn get_domain_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError> {
        let view = self.usecase.get_domain_by_id(id).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<SdtmProjectDomainList, SdtmProjectApiError> {
        let views = self.usecase.list_domains_by_version(version_id).await?;
        Ok(SdtmProjectDomainList {
            domains: views.into_iter().map(domain_view_from_internal).collect(),
        })
    }

    async fn update_domain(
        &self,
        req: UpdateSdtmProjectDomainRequest,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectDomain {
            id: req.id,
            domain_code: req.domain_code,
            key_variables: req.key_variables,
        };
        let view = self.usecase.update_domain(cmd).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn delete_domain(&self, id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_domain(id).await?;
        Ok(())
    }

    // ---- SdtmProjectDomainOverride ----

    async fn create_domain_override(
        &self,
        req: CreateSdtmProjectDomainOverrideRequest,
    ) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectDomainOverride {
            domain_id: req.domain_id,
            category: to_internal_category(req.category),
            descriptions: domain_descriptions_from_api(req.descriptions),
        };
        let view = self.usecase.create_domain_override(cmd).await?;
        Ok(domain_override_view_from_internal(view))
    }

    async fn get_domain_override(
        &self,
        domain_id: i64,
    ) -> Result<Option<SdtmProjectDomainOverrideView>, SdtmProjectApiError> {
        let view = self.usecase.get_domain_override(domain_id).await?;
        Ok(view.map(domain_override_view_from_internal))
    }

    async fn update_domain_override(
        &self,
        req: UpdateSdtmProjectDomainOverrideRequest,
    ) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectDomainOverride {
            domain_id: req.domain_id,
            category: req.category.map(to_internal_category),
            descriptions: req.descriptions.map(domain_descriptions_from_api),
        };
        let view = self.usecase.update_domain_override(cmd).await?;
        Ok(domain_override_view_from_internal(view))
    }

    async fn delete_domain_override(&self, domain_id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_domain_override(domain_id).await?;
        Ok(())
    }

    // ---- SdtmProjectVariable ----

    async fn create_variable(
        &self,
        req: CreateSdtmProjectVariableRequest,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectVariable {
            domain_id: req.domain_id,
            variable_code: req.variable_code,
            variable_sequence: req.variable_sequence,
        };
        let view = self.usecase.create_variable(cmd).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn get_variable_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError> {
        let view = self.usecase.get_variable_by_id(id).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<SdtmProjectVariableList, SdtmProjectApiError> {
        let views = self.usecase.list_variables_by_domain(domain_id).await?;
        Ok(SdtmProjectVariableList {
            variables: views.into_iter().map(variable_view_from_internal).collect(),
        })
    }

    async fn update_variable(
        &self,
        req: UpdateSdtmProjectVariableRequest,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectVariable {
            id: req.id,
            variable_code: req.variable_code,
            variable_sequence: req.variable_sequence,
        };
        let view = self.usecase.update_variable(cmd).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn delete_variable(&self, id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_variable(id).await?;
        Ok(())
    }

    // ---- SdtmProjectVariableOverride ----

    async fn create_variable_override(
        &self,
        req: CreateSdtmProjectVariableOverrideRequest,
    ) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectVariableOverride {
            variable_id: req.variable_id,
            variable_type: to_internal_variable_type(req.variable_type),
            variable_core: to_internal_variable_core(req.variable_core),
            variable_role: req.variable_role.map(to_internal_role),
            descriptions: variable_descriptions_from_api(req.descriptions),
        };
        let view = self.usecase.create_variable_override(cmd).await?;
        Ok(variable_override_view_from_internal(view))
    }

    async fn get_variable_override(
        &self,
        variable_id: i64,
    ) -> Result<Option<SdtmProjectVariableOverrideView>, SdtmProjectApiError> {
        let view = self.usecase.get_variable_override(variable_id).await?;
        Ok(view.map(variable_override_view_from_internal))
    }

    async fn update_variable_override(
        &self,
        req: UpdateSdtmProjectVariableOverrideRequest,
    ) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectVariableOverride {
            variable_id: req.variable_id,
            variable_type: req.variable_type.map(to_internal_variable_type),
            variable_core: req.variable_core.map(to_internal_variable_core),
            variable_role: req.variable_role.map(|opt| opt.map(to_internal_role)),
            descriptions: req.descriptions.map(variable_descriptions_from_api),
        };
        let view = self.usecase.update_variable_override(cmd).await?;
        Ok(variable_override_view_from_internal(view))
    }

    async fn delete_variable_override(&self, variable_id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_variable_override(variable_id).await?;
        Ok(())
    }
}
