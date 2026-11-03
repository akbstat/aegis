//! `DomainModelService` adapter over the repository ports.
//!
//! Hosts `DomainModelServiceImpl<V, D, Va>` — the implementation
//! of `apis::domain_model::DomainModelService` that adapts
//! `domain_model::DomainModelUsecase` to the API contract.
//! Translation between `apis::domain_model::*` and
//! `domain_model::usecase::*` happens inline in each trait
//! method.

use async_trait::async_trait;

use apis::domain_model::{
    CreateSdtmDomainRequest, CreateSdtmVariableRequest, CreateSdtmVersionRequest,
    DomainCategory as ApiCategory, DomainModelApiError, DomainModelService,
    SdtmDomainDescription as ApiSdtmDomainDescription,
    SdtmDomainDescriptionDetail as ApiSdtmDomainDescriptionDetail, SdtmDomainList, SdtmDomainView,
    SdtmRole as ApiSdtmRole, SdtmVariableControlled as ApiSdtmVariableControlled,
    SdtmVariableCore as ApiSdtmVariableCore, SdtmVariableDescription as ApiSdtmVariableDescription,
    SdtmVariableDescriptionDetail as ApiSdtmVariableDescriptionDetail, SdtmVariableList,
    SdtmVariableType as ApiSdtmVariableType, SdtmVariableView, SdtmVersionList, SdtmVersionView,
    UpdateSdtmDomainRequest, UpdateSdtmVariableRequest, UpdateSdtmVersionRequest,
};

use crate::domain::{
    DomainCategory, DomainError, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmDomainRepository, SdtmRole, SdtmVariableControlled, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableRepository,
    SdtmVariableType, SdtmVersionRepository,
};
use crate::usecase::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion, DomainModelUsecase,
    DomainModelUsecaseConfig, SdtmDomainView as InternalSdtmDomainView,
    SdtmVariableView as InternalSdtmVariableView, SdtmVersionView as InternalSdtmVersionView,
    UpdateSdtmDomain, UpdateSdtmVariable, UpdateSdtmVersion, UsecaseError,
};

/// Adapter that implements [`DomainModelService`] on top of a
/// [`DomainModelUsecase`].
///
/// Generic over the three persistence ports so the adapter can be
/// exercised against in-memory fakes in tests and against the
/// PostgreSQL-backed
/// [`SdtmVersionRepoPg`](crate::SdtmVersionRepoPg) /
/// [`SdtmDomainRepoPg`](crate::SdtmDomainRepoPg) /
/// [`SdtmVariableRepoPg`](crate::SdtmVariableRepoPg) in production.
pub struct DomainModelServiceImpl<
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
> {
    usecase: DomainModelUsecase<V, D, Va>,
}

impl<V, D, Va> DomainModelServiceImpl<V, D, Va>
where
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
{
    /// Build a new `DomainModelServiceImpl` wrapping the supplied
    /// usecase.
    pub fn new(usecase: DomainModelUsecase<V, D, Va>) -> Self {
        Self { usecase }
    }

    /// Build a new `DomainModelServiceImpl` from the three
    /// repositories directly. Mirrors the
    /// [`DomainModelUsecase::new`] constructor shape so callers
    /// who already hold the three repos do not need to assemble a
    /// [`DomainModelUsecaseConfig`] first.
    pub fn from_repos(version_repo: V, domain_repo: D, variable_repo: Va) -> Self {
        Self::new(DomainModelUsecase::new(DomainModelUsecaseConfig {
            version_repo,
            domain_repo,
            variable_repo,
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

fn domain_descriptions_from_api(v: Vec<ApiSdtmDomainDescription>) -> Vec<SdtmDomainDescription> {
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

fn domain_descriptions_to_api(v: Vec<SdtmDomainDescription>) -> Vec<ApiSdtmDomainDescription> {
    v.into_iter()
        .map(|d| ApiSdtmDomainDescription {
            lang: d.lang,
            details: ApiSdtmDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        })
        .collect()
}

fn variable_descriptions_from_api(
    v: Vec<ApiSdtmVariableDescription>,
) -> Vec<SdtmVariableDescription> {
    v.into_iter()
        .map(|d| SdtmVariableDescription {
            lang: d.lang,
            details: SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        })
        .collect()
}

fn variable_descriptions_to_api(
    v: Vec<SdtmVariableDescription>,
) -> Vec<ApiSdtmVariableDescription> {
    v.into_iter()
        .map(|d| ApiSdtmVariableDescription {
            lang: d.lang,
            details: ApiSdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        })
        .collect()
}

fn variable_controlled_from_api(v: ApiSdtmVariableControlled) -> SdtmVariableControlled {
    SdtmVariableControlled {
        code: v.code,
        format: v.format,
    }
}

fn variable_controlled_to_api(v: SdtmVariableControlled) -> ApiSdtmVariableControlled {
    ApiSdtmVariableControlled {
        code: v.code,
        format: v.format,
    }
}

// ---- view mappers ----

fn version_view_from_internal(view: InternalSdtmVersionView) -> SdtmVersionView {
    SdtmVersionView {
        id: view.id,
        name: view.name,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn domain_view_from_internal(view: InternalSdtmDomainView) -> SdtmDomainView {
    SdtmDomainView {
        id: view.id,
        version_id: view.version_id,
        name: view.name,
        category: from_internal_category(view.category),
        descriptions: domain_descriptions_to_api(view.descriptions),
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn variable_view_from_internal(view: InternalSdtmVariableView) -> SdtmVariableView {
    SdtmVariableView {
        id: view.id,
        domain_id: view.domain_id,
        name: view.name,
        variable_controlled: view.variable_controlled.map(variable_controlled_to_api),
        variable_type: from_internal_variable_type(view.variable_type),
        variable_core: from_internal_variable_core(view.variable_core),
        variable_role: view.variable_role.map(from_internal_role),
        variable_sequence: view.variable_sequence,
        descriptions: variable_descriptions_to_api(view.descriptions),
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

// ---- error mapper ----

/// Translate a [`UsecaseError`] into the API's
/// [`DomainModelApiError`].
///
/// `UsecaseError::Validation` only ever wraps the validation-only
/// `DomainError` variants; the `unreachable!` arm in the
/// `Repository` branch documents that fact and would fire if a
/// future change ever broke the invariant.
impl From<UsecaseError> for DomainModelApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => DomainModelApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => DomainModelApiError::NotFound,
                DomainError::SdtmVersionNotFound(id) => {
                    DomainModelApiError::SdtmVersionNotFound(id)
                }
                DomainError::SdtmDomainNotFound(id) => DomainModelApiError::SdtmDomainNotFound(id),
                DomainError::SdtmVariableNotFound(id) => {
                    DomainModelApiError::SdtmVariableNotFound(id)
                }
                DomainError::DuplicateSdtmVersion { name } => {
                    DomainModelApiError::DuplicateSdtmVersion { name }
                }
                DomainError::DuplicateSdtmDomain { version_id, name } => {
                    DomainModelApiError::DuplicateSdtmDomain { version_id, name }
                }
                DomainError::DuplicateSdtmVariable { domain_id, name } => {
                    DomainModelApiError::DuplicateSdtmVariable { domain_id, name }
                }
                DomainError::FkSdtmVersionNotFound(id) => {
                    DomainModelApiError::FkSdtmVersionNotFound(id)
                }
                DomainError::FkSdtmDomainNotFound(id) => {
                    DomainModelApiError::FkSdtmDomainNotFound(id)
                }
                DomainError::Repository(msg) => DomainModelApiError::Repository(msg),
                DomainError::EmptyName
                | DomainError::InvalidDomainCategory(_)
                | DomainError::InvalidVariableType(_)
                | DomainError::InvalidVariableCore(_)
                | DomainError::InvalidVariableRole(_) => unreachable!(
                    "domain validation errors are only produced as UsecaseError::Validation"
                ),
            },
        }
    }
}

#[async_trait]
impl<V, D, Va> DomainModelService for DomainModelServiceImpl<V, D, Va>
where
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
{
    // ---- SdtmVersion ----

    async fn create_version(
        &self,
        req: CreateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError> {
        let cmd = CreateSdtmVersion { name: req.name };
        let view = self.usecase.create_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn list_versions(&self) -> Result<SdtmVersionList, DomainModelApiError> {
        let views = self.usecase.list_versions().await?;
        Ok(SdtmVersionList {
            versions: views.into_iter().map(version_view_from_internal).collect(),
        })
    }

    async fn update_version(
        &self,
        req: UpdateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError> {
        let cmd = UpdateSdtmVersion {
            id: req.id,
            name: req.name,
        };
        let view = self.usecase.update_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn delete_version(&self, id: i64) -> Result<(), DomainModelApiError> {
        self.usecase.delete_version(id).await?;
        Ok(())
    }

    // ---- SdtmDomain ----

    async fn create_domain(
        &self,
        req: CreateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError> {
        let cmd = CreateSdtmDomain {
            version_id: req.version_id,
            name: req.name,
            category: to_internal_category(req.category),
            descriptions: domain_descriptions_from_api(req.descriptions),
        };
        let view = self.usecase.create_domain(cmd).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn get_domain_by_id(&self, id: i64) -> Result<SdtmDomainView, DomainModelApiError> {
        let view = self.usecase.get_domain_by_id(id).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<SdtmDomainList, DomainModelApiError> {
        let views = self.usecase.list_domains_by_version(version_id).await?;
        Ok(SdtmDomainList {
            domains: views.into_iter().map(domain_view_from_internal).collect(),
        })
    }

    async fn update_domain(
        &self,
        req: UpdateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError> {
        let cmd = UpdateSdtmDomain {
            id: req.id,
            name: req.name,
            category: req.category.map(to_internal_category),
            descriptions: req.descriptions.map(domain_descriptions_from_api),
        };
        let view = self.usecase.update_domain(cmd).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn delete_domain(&self, id: i64) -> Result<(), DomainModelApiError> {
        self.usecase.delete_domain(id).await?;
        Ok(())
    }

    // ---- SdtmVariable ----

    async fn create_variable(
        &self,
        req: CreateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError> {
        let cmd = CreateSdtmVariable {
            domain_id: req.domain_id,
            name: req.name,
            variable_controlled: req.variable_controlled.map(variable_controlled_from_api),
            variable_type: to_internal_variable_type(req.variable_type),
            variable_core: to_internal_variable_core(req.variable_core),
            variable_role: req.variable_role.map(to_internal_role),
            variable_sequence: req.variable_sequence,
            descriptions: variable_descriptions_from_api(req.descriptions),
        };
        let view = self.usecase.create_variable(cmd).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn get_variable_by_id(&self, id: i64) -> Result<SdtmVariableView, DomainModelApiError> {
        let view = self.usecase.get_variable_by_id(id).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<SdtmVariableList, DomainModelApiError> {
        let views = self.usecase.list_variables_by_domain(domain_id).await?;
        Ok(SdtmVariableList {
            variables: views.into_iter().map(variable_view_from_internal).collect(),
        })
    }

    async fn update_variable(
        &self,
        req: UpdateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError> {
        let cmd = UpdateSdtmVariable {
            id: req.id,
            name: req.name,
            variable_controlled: req
                .variable_controlled
                .map(|inner| inner.map(variable_controlled_from_api)),
            variable_type: req.variable_type.map(to_internal_variable_type),
            variable_core: req.variable_core.map(to_internal_variable_core),
            variable_role: req.variable_role.map(|opt| opt.map(to_internal_role)),
            variable_sequence: req.variable_sequence,
            descriptions: req.descriptions.map(variable_descriptions_from_api),
        };
        let view = self.usecase.update_variable(cmd).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn delete_variable(&self, id: i64) -> Result<(), DomainModelApiError> {
        self.usecase.delete_variable(id).await?;
        Ok(())
    }
}
