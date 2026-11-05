use async_trait::async_trait;

use apis::project::{
    CreateProjectBranchRequest, CreateProjectRequest, ModelVersionData,
    ModelVersionView as ApiModelVersionView, ProjectApiError,
    ProjectBranchView as ApiProjectBranchView, ProjectConfigurationData,
    ProjectLanguage as ApiProjectLanguage, ProjectMemberData,
    ProjectMemberView as ApiProjectMemberView, ProjectService, ProjectView, TagData,
    UpdateProjectBranchRequest, UpdateProjectRequest, UserSummaryView as ApiUserSummaryView,
};

use crate::domain::{
    ModelVersion, ProjectBranchRepository, ProjectConfiguration, ProjectLanguage, ProjectMember,
    ProjectRepository, ProjectTag, TerminologyVersionData, UserService,
};
use crate::usecase::{
    CreateProject, CreateProjectBranch, ModelVersionView, ProjectBranchUsecase, ProjectBranchView,
    ProjectConfigurationView, ProjectUsecase, TerminologyVersionView, UpdateProject,
    UpdateProjectBranch, UserSummaryView as DomainUserSummaryView,
};

/// Facade adapting `ProjectUsecase<R, U>` and
/// `ProjectBranchUsecase<B>` to `apis::project::ProjectService`. The
/// construction is the same regardless of the underlying storage: the
/// generic `R / U / B` arguments stay concrete in the caller.
pub struct ProjectServiceImpl<R, U, B>
where
    R: ProjectRepository,
    U: UserService,
    B: ProjectBranchRepository,
{
    usecase: ProjectUsecase<R, U>,
    branch_usecase: ProjectBranchUsecase<B>,
}

impl<R, U, B> ProjectServiceImpl<R, U, B>
where
    R: ProjectRepository,
    U: UserService,
    B: ProjectBranchRepository,
{
    pub fn new(usecase: ProjectUsecase<R, U>, branch_usecase: ProjectBranchUsecase<B>) -> Self {
        Self {
            usecase,
            branch_usecase,
        }
    }
}

#[async_trait]
impl<R, U, B> ProjectService for ProjectServiceImpl<R, U, B>
where
    R: ProjectRepository + 'static,
    U: UserService + 'static,
    B: ProjectBranchRepository + 'static,
{
    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .create_project(CreateProject {
                code: req.code,
                description: req.description,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
                configuration: req.configurations.map(configuration_data_to_domain),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .get_project_by_id(id)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .get_project_by_code(code)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError> {
        let views = self.usecase.list_projects().await.map_err(map_error)?;
        Ok(views.into_iter().map(Into::into).collect())
    }

    async fn update_project(
        &self,
        req: UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .update_project(UpdateProject {
                id: req.id,
                code: req.code,
                description: req.description,
                active: req.active,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
                configuration: req.configurations.map(configuration_data_to_domain),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn create_project_branch(
        &self,
        req: CreateProjectBranchRequest,
    ) -> Result<ApiProjectBranchView, ProjectApiError> {
        let view = self
            .branch_usecase
            .create_branch(CreateProjectBranch {
                project_id: req.project_id,
                branch: req.branch,
                active: req.active,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_branch_by_id(
        &self,
        id: i32,
    ) -> Result<ApiProjectBranchView, ProjectApiError> {
        let view = self
            .branch_usecase
            .get_branch_by_id(id)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn list_project_branches(
        &self,
        project_id: i32,
    ) -> Result<Vec<ApiProjectBranchView>, ProjectApiError> {
        let views = self
            .branch_usecase
            .list_branches(project_id)
            .await
            .map_err(map_error)?;
        Ok(views.into_iter().map(Into::into).collect())
    }

    async fn update_project_branch(
        &self,
        req: UpdateProjectBranchRequest,
    ) -> Result<ApiProjectBranchView, ProjectApiError> {
        let view = self
            .branch_usecase
            .update_branch(UpdateProjectBranch {
                id: req.id,
                project_id: req.project_id,
                branch: req.branch,
                active: req.active,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }
}

fn member_data_to_domain(d: ProjectMemberData) -> ProjectMember {
    ProjectMember::for_repository(d.leaders, d.workers)
}

/// Bridge for the request-side `ProjectConfigurationData` so the
/// apis port doesn't need to reach into the domain types. The
/// usecase / domain layer re-validates the inner tags via
/// `ProjectTag::new`; if the wire payload violated the non-empty
/// contract, that re-validation surfaces as
/// `UsecaseError::Validation(EmptyTagKey | EmptyTagValue | EmptySdtmigName)`.
fn configuration_data_to_domain(d: ProjectConfigurationData) -> ProjectConfiguration {
    ProjectConfiguration::for_repository(
        d.language.map(api_language_to_domain),
        tag_data_vec_to_domain(d.tags),
        d.sdtmig.map(model_version_data_to_domain),
        d.sdtm_terminology.map(terminology_version_data_to_domain),
    )
}

fn terminology_version_data_to_domain(
    d: apis::project::TerminologyVersionData,
) -> TerminologyVersionData {
    TerminologyVersionData::for_repository(d.version_id, d.version_name)
}

fn model_version_data_to_domain(d: ModelVersionData) -> ModelVersion {
    ModelVersion::for_repository(d.version_id, d.version_name)
}

fn api_language_to_domain(l: ApiProjectLanguage) -> ProjectLanguage {
    match l {
        ApiProjectLanguage::English => ProjectLanguage::English,
        ApiProjectLanguage::SimplifiedChinese => ProjectLanguage::SimplifiedChinese,
    }
}

fn tag_data_vec_to_domain(tags: Vec<TagData>) -> Vec<ProjectTag> {
    tags.into_iter()
        .map(|t| ProjectTag::for_repository(t.key, t.value))
        .collect()
}

fn map_error(err: crate::usecase::UsecaseError) -> ProjectApiError {
    use crate::domain::DomainError;
    use crate::usecase::UsecaseError;
    match err {
        UsecaseError::Validation(d) => ProjectApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => ProjectApiError::NotFound,
            DomainError::UserNotFound(code) => ProjectApiError::UserNotFound(code),
            DomainError::DuplicateCode(code) => ProjectApiError::DuplicateCode(code),
            DomainError::DuplicateBranch(name) => ProjectApiError::DuplicateBranch(name),
            DomainError::BranchNotFound(id) => ProjectApiError::BranchNotFound(id),
            DomainError::BranchProjectNotFound(id) => ProjectApiError::BranchProjectNotFound(id),
            other => ProjectApiError::Repository(other.to_string()),
        },
    }
}

// ---- From impls: domain usecase views -> apis views ----

impl From<crate::usecase::ProjectView> for ProjectView {
    fn from(v: crate::usecase::ProjectView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            description: v.description,
            members: v.members.into(),
            unblind_members: v.unblind_members.into(),
            configurations: v.configurations.into(),
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<ProjectBranchView> for ApiProjectBranchView {
    fn from(v: ProjectBranchView) -> Self {
        Self {
            id: v.id,
            project_id: v.project_id,
            branch: v.branch,
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<ProjectConfigurationView> for apis::project::ProjectConfigurationView {
    fn from(v: ProjectConfigurationView) -> Self {
        Self {
            language: v.language.map(domain_language_to_api),
            tags: v.tags.into_iter().map(Into::into).collect(),
            sdtmig: v.sdtmig.map(model_version_view_to_api),
            sdtm_terminology: v.sdtm_terminology.map(terminology_version_view_to_api),
        }
    }
}

fn terminology_version_view_to_api(
    v: TerminologyVersionView,
) -> apis::project::TerminologyVersionView {
    apis::project::TerminologyVersionView {
        version_id: v.version_id,
        version_name: v.version_name,
    }
}

fn model_version_view_to_api(v: ModelVersionView) -> ApiModelVersionView {
    ApiModelVersionView {
        version_id: v.version_id,
        version_name: v.version_name,
    }
}

fn domain_language_to_api(l: ProjectLanguage) -> ApiProjectLanguage {
    match l {
        ProjectLanguage::English => ApiProjectLanguage::English,
        ProjectLanguage::SimplifiedChinese => ApiProjectLanguage::SimplifiedChinese,
    }
}

impl From<crate::usecase::TagView> for apis::project::TagView {
    fn from(v: crate::usecase::TagView) -> Self {
        Self {
            key: v.key,
            value: v.value,
        }
    }
}

impl From<crate::usecase::ProjectMemberView> for ApiProjectMemberView {
    fn from(v: crate::usecase::ProjectMemberView) -> Self {
        Self {
            leaders: v.leaders.into_iter().map(Into::into).collect(),
            workers: v.workers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DomainUserSummaryView> for ApiUserSummaryView {
    fn from(v: DomainUserSummaryView) -> Self {
        Self {
            code: v.code,
            name: v.name,
        }
    }
}
