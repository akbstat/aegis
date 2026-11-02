//! Shared state injected into every handler via `axum::extract::State`.

use std::sync::Arc;

/// Shared state injected into every handler.
///
/// Cloned per worker task (axum's `State<T>: Clone` requires it);
/// all services are `Arc`, so the clone is cheap.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
    pub project: Arc<dyn apis::project::ProjectService>,
    pub terminology: Arc<dyn apis::terminology::TerminologyService>,
    pub domain_model: Arc<dyn apis::domain_model::DomainModelService>,
    pub mission: Arc<dyn apis::mission::MissionService>,
    pub crf: Arc<dyn apis::crf::CrfService>,
    pub sdtm_project: Arc<dyn apis::sdtm_project::SdtmProjectService>,
    pub structures: Arc<dyn apis::structures::StructureService>,
    pub log_ingestor: Arc<logging_utils::LogIngestor>,
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Shared test doubles used by every per-module `tests` block.
    //!
    //! Each `tests` block that builds an `AppState` literally needs a
    //! stand-in implementation for every service field. To avoid
    //! copy-pasting 100-line `#[async_trait] impl` blocks across six
    //! test modules, the common null doubles live here.

    use std::sync::Arc;

    use async_trait::async_trait;

    /// A `StructureService` for tests that need an `AppState` but
    /// never touch the structures endpoints. Every method is
    /// `unimplemented!()`, like the other null doubles in this module.
    #[derive(Clone)]
    pub(crate) struct NullStructureService;

    #[async_trait]
    impl apis::structures::StructureService for NullStructureService {
        async fn create_structure(
            &self,
            _req: apis::structures::CreateStructureRequest,
        ) -> Result<apis::structures::StructureView, apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn get_structure(
            &self,
            _id: i64,
        ) -> Result<apis::structures::StructureView, apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn get_structure_meta(
            &self,
            _id: i64,
        ) -> Result<apis::structures::StructureMetaView, apis::structures::StructureApiError>
        {
            unimplemented!()
        }
        async fn list_structures(
            &self,
            _req: apis::structures::ListStructuresRequest,
        ) -> Result<apis::structures::StructureList, apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn rename_structure(
            &self,
            _req: apis::structures::RenameStructureRequest,
        ) -> Result<apis::structures::StructureView, apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn delete_structure(
            &self,
            _id: i64,
        ) -> Result<(), apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn add_node(
            &self,
            _req: apis::structures::AddNodeRequest,
        ) -> Result<apis::structures::NodeView, apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn update_node(
            &self,
            _req: apis::structures::UpdateNodeRequest,
        ) -> Result<apis::structures::NodeView, apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn delete_node(
            &self,
            _req: apis::structures::DeleteNodeRequest,
        ) -> Result<(), apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn list_own_nodes(
            &self,
            _structure_id: i64,
        ) -> Result<apis::structures::OwnNodeList, apis::structures::StructureApiError> {
            unimplemented!()
        }
        async fn build_structure(
            &self,
            _id: i64,
        ) -> Result<apis::structures::StructureTreeView, apis::structures::StructureApiError>
        {
            unimplemented!()
        }
    }

    /// A `SdtmProjectService` for tests that need an `AppState` but
    /// never touch the sdtm-project endpoints. Every method is
    /// `unimplemented!()`, like the other null doubles in this module.
    #[derive(Clone)]
    pub(crate) struct NullSdtmProjectService;

    #[async_trait]
    impl apis::sdtm_project::SdtmProjectService for NullSdtmProjectService {
        async fn create_version(
            &self,
            _req: apis::sdtm_project::CreateSdtmProjectVersionRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVersionView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn get_version_by_id(
            &self,
            _id: i64,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVersionView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn list_versions_by_project(
            &self,
            _project_code: &str,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVersionList,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn update_version(
            &self,
            _req: apis::sdtm_project::UpdateSdtmProjectVersionRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVersionView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn delete_version(
            &self,
            _id: i64,
        ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
            unimplemented!()
        }
        async fn create_domain(
            &self,
            _req: apis::sdtm_project::CreateSdtmProjectDomainRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectDomainView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn get_domain_by_id(
            &self,
            _id: i64,
        ) -> Result<
            apis::sdtm_project::SdtmProjectDomainView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn list_domains_by_version(
            &self,
            _version_id: i64,
        ) -> Result<
            apis::sdtm_project::SdtmProjectDomainList,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn update_domain(
            &self,
            _req: apis::sdtm_project::UpdateSdtmProjectDomainRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectDomainView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn delete_domain(
            &self,
            _id: i64,
        ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
            unimplemented!()
        }
        async fn create_domain_override(
            &self,
            _req: apis::sdtm_project::CreateSdtmProjectDomainOverrideRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectDomainOverrideView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn get_domain_override(
            &self,
            _domain_id: i64,
        ) -> Result<
            Option<apis::sdtm_project::SdtmProjectDomainOverrideView>,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn update_domain_override(
            &self,
            _req: apis::sdtm_project::UpdateSdtmProjectDomainOverrideRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectDomainOverrideView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn delete_domain_override(
            &self,
            _domain_id: i64,
        ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
            unimplemented!()
        }
        async fn create_variable(
            &self,
            _req: apis::sdtm_project::CreateSdtmProjectVariableRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVariableView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn get_variable_by_id(
            &self,
            _id: i64,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVariableView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn list_variables_by_domain(
            &self,
            _domain_id: i64,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVariableList,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn update_variable(
            &self,
            _req: apis::sdtm_project::UpdateSdtmProjectVariableRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVariableView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn delete_variable(
            &self,
            _id: i64,
        ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
            unimplemented!()
        }
        async fn create_variable_override(
            &self,
            _req: apis::sdtm_project::CreateSdtmProjectVariableOverrideRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVariableOverrideView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn get_variable_override(
            &self,
            _variable_id: i64,
        ) -> Result<
            Option<apis::sdtm_project::SdtmProjectVariableOverrideView>,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn update_variable_override(
            &self,
            _req: apis::sdtm_project::UpdateSdtmProjectVariableOverrideRequest,
        ) -> Result<
            apis::sdtm_project::SdtmProjectVariableOverrideView,
            apis::sdtm_project::SdtmProjectApiError,
        > {
            unimplemented!()
        }
        async fn delete_variable_override(
            &self,
            _variable_id: i64,
        ) -> Result<(), apis::sdtm_project::SdtmProjectApiError> {
            unimplemented!()
        }
    }

    /// A `LogIngestor` for tests that need an `AppState` but never
    /// touch the `log_ingest` endpoint. The internal tempdir is
    /// scoped to the returned `Arc`'s lifetime — when the test drops
    /// `AppState`, `LogIngestor::shutdown` runs (with its deadline)
    /// and the tempdir cleanup is best-effort. The dir does not need
    /// to be writable for tests that never call `submit`.
    pub(crate) fn unused_log_ingestor() -> Arc<logging_utils::LogIngestor> {
        use logging_utils::{LogIngestor, LogIngestorConfig};
        let tmp = tempfile::tempdir().expect("tempdir");
        let cfg = LogIngestorConfig::builder()
            .log_dir(tmp.path().to_path_buf())
            .file_name_prefix("unused.log".into())
            .build();
        Arc::new(LogIngestor::new(cfg).expect("ingestor init"))
    }

    /// Null terminology service for tests that don't exercise the
    /// terminology surface. Every method panics with
    /// `unimplemented!()` — the corresponding handler is never reached
    /// because the test either uses a different route or short-
    /// circuits on auth / role checks before the usecase is invoked.
    #[derive(Clone)]
    pub(crate) struct NullTerminologyService;

    #[async_trait]
    impl apis::terminology::TerminologyService for NullTerminologyService {
        async fn create_version(
            &self,
            _req: apis::terminology::CreateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_versions(
            &self,
        ) -> Result<
            Vec<apis::terminology::TerminologyVersionView>,
            apis::terminology::TerminologyApiError,
        > {
            unimplemented!()
        }
        async fn get_version_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn update_version(
            &self,
            _req: apis::terminology::UpdateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn delete_version(
            &self,
            _id: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn create_code_list(
            &self,
            _req: apis::terminology::CreateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_lists(
            &self,
            _query: apis::terminology::CodeListListQuery,
        ) -> Result<
            apis::terminology::Page<apis::terminology::CodeListView>,
            apis::terminology::TerminologyApiError,
        > {
            unimplemented!()
        }
        async fn get_code_list_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn update_code_list(
            &self,
            _req: apis::terminology::UpdateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn delete_code_list(
            &self,
            _id: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn create_code_item(
            &self,
            _req: apis::terminology::CreateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_items(
            &self,
            _query: apis::terminology::CodeItemListQuery,
        ) -> Result<
            apis::terminology::Page<apis::terminology::CodeItemView>,
            apis::terminology::TerminologyApiError,
        > {
            unimplemented!()
        }
        async fn list_code_items_by_version_and_code(
            &self,
            _version_id: i64,
            _code: &str,
        ) -> Result<Vec<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn update_code_item(
            &self,
            _req: apis::terminology::UpdateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn delete_code_item(
            &self,
            _id: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn batch_create_code_items(
            &self,
            _req: apis::terminology::BatchCreateCodeItemsRequest,
        ) -> Result<
            apis::terminology::BatchCreateCodeItemsResponse,
            apis::terminology::TerminologyApiError,
        > {
            unimplemented!()
        }
    }

    /// Null domain-model service for tests that don't exercise the
    /// SDTM domain-model surface. Every method panics with
    /// `unimplemented!()` — the corresponding handler is never reached
    /// because the test either uses a different route or short-
    /// circuits on auth / role checks before the usecase is invoked.
    #[derive(Clone)]
    pub(crate) struct NullDomainModelService;

    #[async_trait]
    impl apis::domain_model::DomainModelService for NullDomainModelService {
        async fn create_version(
            &self,
            _req: apis::domain_model::CreateSdtmVersionRequest,
        ) -> Result<apis::domain_model::SdtmVersionView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn list_versions(
            &self,
        ) -> Result<apis::domain_model::SdtmVersionList, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn update_version(
            &self,
            _req: apis::domain_model::UpdateSdtmVersionRequest,
        ) -> Result<apis::domain_model::SdtmVersionView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn delete_version(
            &self,
            _id: i64,
        ) -> Result<(), apis::domain_model::DomainModelApiError> {
            unimplemented!()
        }
        async fn create_domain(
            &self,
            _req: apis::domain_model::CreateSdtmDomainRequest,
        ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn get_domain_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn list_domains_by_version(
            &self,
            _version_id: i64,
        ) -> Result<apis::domain_model::SdtmDomainList, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn update_domain(
            &self,
            _req: apis::domain_model::UpdateSdtmDomainRequest,
        ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn delete_domain(
            &self,
            _id: i64,
        ) -> Result<(), apis::domain_model::DomainModelApiError> {
            unimplemented!()
        }
        async fn create_variable(
            &self,
            _req: apis::domain_model::CreateSdtmVariableRequest,
        ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn get_variable_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn list_variables_by_domain(
            &self,
            _domain_id: i64,
        ) -> Result<apis::domain_model::SdtmVariableList, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn update_variable(
            &self,
            _req: apis::domain_model::UpdateSdtmVariableRequest,
        ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn delete_variable(
            &self,
            _id: i64,
        ) -> Result<(), apis::domain_model::DomainModelApiError> {
            unimplemented!()
        }
    }

    /// Null mission service for tests that don't exercise the
    /// mission surface. Every method panics with `unimplemented!()`
    /// — the corresponding handler is never reached because the
    /// test either uses a different route or short-circuits on auth
    /// / role checks before the usecase is invoked.
    #[derive(Clone)]
    pub(crate) struct NullMissionService;

    #[async_trait]
    impl apis::mission::MissionService for NullMissionService {
        async fn create_mission(
            &self,
            _actor: &apis::mission::Actor,
            _req: apis::mission::CreateMissionRequest,
        ) -> Result<apis::mission::MissionView, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn get_mission_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::mission::MissionView, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn list_missions_by_project(
            &self,
            _req: apis::mission::ListMissionsByProjectRequest,
        ) -> Result<Vec<apis::mission::MissionView>, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn list_missions_by_user(
            &self,
            _req: apis::mission::ListMissionsByUserRequest,
        ) -> Result<Vec<apis::mission::MissionView>, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn delete_mission(
            &self,
            _actor: &apis::mission::Actor,
            _id: i64,
        ) -> Result<(), apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn add_assignee(
            &self,
            _actor: &apis::mission::Actor,
            _mission_id: i64,
            _data: apis::mission::AssigneeData,
        ) -> Result<apis::mission::AssigneeView, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn remove_assignee(
            &self,
            _actor: &apis::mission::Actor,
            _mission_id: i64,
            _assignee_id: i64,
        ) -> Result<(), apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn list_issues_by_mission(
            &self,
            _req: apis::mission::ListIssuesByMissionRequest,
        ) -> Result<Vec<apis::mission::IssueView>, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn create_issue(
            &self,
            _actor: &apis::mission::Actor,
            _req: apis::mission::CreateIssueRequest,
        ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn close_issue(
            &self,
            _actor: &apis::mission::Actor,
            _req: apis::mission::CloseIssueRequest,
            _issue_id: i64,
        ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn reopen_issue(
            &self,
            _actor: &apis::mission::Actor,
            _req: apis::mission::ReopenIssueRequest,
            _issue_id: i64,
        ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn update_issue_description(
            &self,
            _actor: &apis::mission::Actor,
            _issue_id: i64,
            _req: apis::mission::UpdateIssueDescriptionRequest,
        ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
            unimplemented!()
        }
        async fn append_comment(
            &self,
            _actor: &apis::mission::Actor,
            _issue_id: i64,
            _req: apis::mission::AppendCommentRequest,
        ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
            unimplemented!()
        }
    }

    /// Null CRF service for tests that don't exercise the
    /// Case Report Form surface. Every method panics with
    /// `unimplemented!()` — the corresponding handler is never
    /// reached because the test either uses a different route or
    /// short-circuits on auth / role checks before the usecase is
    /// invoked.
    #[derive(Clone)]
    pub(crate) struct NullCrfService;

    #[async_trait]
    impl apis::crf::CrfService for NullCrfService {
        async fn create_version(
            &self,
            _req: apis::crf::CreateCrfVersionRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_version_by_id(
            &self,
            _req: apis::crf::GetCrfVersionByIdRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_versions_by_project(
            &self,
            _req: apis::crf::ListCrfVersionsByProjectRequest,
        ) -> Result<Vec<apis::crf::CrfVersionView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_version(
            &self,
            _req: apis::crf::UpdateCrfVersionRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_version(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_form(
            &self,
            _req: apis::crf::CreateCrfFormRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_form_by_id(
            &self,
            _req: apis::crf::GetCrfFormByIdRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_form_detail(
            &self,
            _req: apis::crf::GetCrfFormDetailRequest,
        ) -> Result<apis::crf::CrfFormDetailView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_forms_by_version(
            &self,
            _req: apis::crf::ListCrfFormsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfFormView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_form(
            &self,
            _req: apis::crf::UpdateCrfFormRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_form(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn set_approved(
            &self,
            _req: apis::crf::SetCrfApprovedRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn bulk_create_form(
            &self,
            _req: apis::crf::BulkCreateCrfFormRequest,
        ) -> Result<apis::crf::BulkCreateCrfFormResult, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_item(
            &self,
            _req: apis::crf::CreateCrfItemRequest,
        ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_item_by_id(
            &self,
            _req: apis::crf::GetCrfItemByIdRequest,
        ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_items_by_form(
            &self,
            _req: apis::crf::ListCrfItemsByFormRequest,
        ) -> Result<Vec<apis::crf::CrfItemView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_item(
            &self,
            _req: apis::crf::UpdateCrfItemRequest,
        ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_item(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_option(
            &self,
            _req: apis::crf::CreateCrfOptionRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_option_by_id(
            &self,
            _req: apis::crf::GetCrfOptionByIdRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_options_by_item(
            &self,
            _req: apis::crf::ListCrfOptionsByItemRequest,
        ) -> Result<Vec<apis::crf::CrfOptionView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_option(
            &self,
            _req: apis::crf::UpdateCrfOptionRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_option(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_unit(
            &self,
            _req: apis::crf::CreateCrfUnitRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_unit_by_id(
            &self,
            _req: apis::crf::GetCrfUnitByIdRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_units_by_item(
            &self,
            _req: apis::crf::ListCrfUnitsByItemRequest,
        ) -> Result<Vec<apis::crf::CrfUnitView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_unit(
            &self,
            _req: apis::crf::UpdateCrfUnitRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_unit(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_domain_annotation(
            &self,
            _req: apis::crf::CreateDomainAnnotationRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_domain_annotation_by_id(
            &self,
            _req: apis::crf::GetDomainAnnotationByIdRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_domain_annotations_by_form(
            &self,
            _req: apis::crf::ListDomainAnnotationsByFormRequest,
        ) -> Result<Vec<apis::crf::DomainAnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_domain_annotation(
            &self,
            _req: apis::crf::UpdateDomainAnnotationRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_domain_annotation(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_annotation(
            &self,
            _req: apis::crf::CreateAnnotationRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_annotation_by_id(
            &self,
            _req: apis::crf::GetAnnotationByIdRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_form(
            &self,
            _req: apis::crf::ListAnnotationsByFormRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_item(
            &self,
            _req: apis::crf::ListAnnotationsByItemRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_option(
            &self,
            _req: apis::crf::ListAnnotationsByOptionRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_unit(
            &self,
            _req: apis::crf::ListAnnotationsByUnitRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_annotation(
            &self,
            _req: apis::crf::UpdateAnnotationRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_annotation(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_forms_by_version(
            &self,
            _req: apis::crf::SearchCrfFormsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfFormView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_items_by_version(
            &self,
            _req: apis::crf::SearchCrfItemsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfItemView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_options_by_version(
            &self,
            _req: apis::crf::SearchCrfOptionsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfOptionView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_units_by_version(
            &self,
            _req: apis::crf::SearchCrfUnitsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfUnitView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_domain_annotations_by_version(
            &self,
            _req: apis::crf::SearchDomainAnnotationsByVersionRequest,
        ) -> Result<Vec<apis::crf::DomainAnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_annotations_by_version(
            &self,
            _req: apis::crf::SearchAnnotationsByVersionRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
    }

    /// Null auth service for tests that need an `AppState` but don't
    /// exercise the auth surface. [`verify`](apis::auth::AuthService::verify)
    /// accepts any non-empty bearer token and returns a fixed
    /// `AuthClaims` so bearer-auth gates fire for tests that need a
    /// logged-in identity. Every other method panics with
    /// `unimplemented!()`.
    #[derive(Clone)]
    pub(crate) struct NullAuthService;

    #[async_trait]
    impl apis::auth::AuthService for NullAuthService {
        async fn login_with_password(
            &self,
            _req: apis::auth::LoginWithPasswordRequest,
        ) -> Result<apis::auth::TokenPair, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn login_with_domain_user_info(
            &self,
            _req: apis::auth::LoginWithDomainUserInfoRequest,
        ) -> Result<apis::auth::TokenPair, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn verify(
            &self,
            req: apis::auth::VerifyRequest,
        ) -> Result<apis::auth::AuthClaims, apis::auth::AuthApiError> {
            if req.access_token.is_empty() {
                return Err(apis::auth::AuthApiError::Verification("empty token".into()));
            }
            Ok(apis::auth::AuthClaims {
                code: "test-user".into(),
                role: apis::user::Role::General,
                token_version: 0,
            })
        }
        async fn refresh(
            &self,
            _req: apis::auth::RefreshRequest,
        ) -> Result<apis::auth::RefreshResponse, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn find_user_credential_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::auth::UserCredentialView, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn create_user_credential(
            &self,
            _req: apis::auth::CreateUserCredentialRequest,
        ) -> Result<apis::auth::UserCredentialView, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn update_user_credential(
            &self,
            _req: apis::auth::UpdateUserCredentialRequest,
        ) -> Result<apis::auth::UserCredentialView, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn remove_user_credential(
            &self,
            _code: &str,
        ) -> Result<apis::auth::RemoveUserCredentialResponse, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn logout(
            &self,
            _req: apis::auth::LogoutRequest,
        ) -> Result<apis::auth::LogoutResponse, apis::auth::AuthApiError> {
            unimplemented!()
        }
        async fn register_user(
            &self,
            _req: apis::auth::RegisterUserRequest,
        ) -> Result<apis::auth::RegisterUserResponse, apis::auth::AuthApiError> {
            unimplemented!()
        }
    }

    /// Null user service for tests that need an `AppState` but never
    /// exercise the user surface. Every method panics with
    /// `unimplemented!()`.
    #[derive(Clone)]
    pub(crate) struct NullUserService;

    #[async_trait]
    impl apis::user::UserService for NullUserService {
        async fn create(
            &self,
            _req: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn get_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn get_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn update(
            &self,
            _req: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
    }

    /// Null project service for tests that need an `AppState` but
    /// never exercise the project surface. Every method panics with
    /// `unimplemented!()`.
    #[derive(Clone)]
    pub(crate) struct NullProjectService;

    #[async_trait]
    impl apis::project::ProjectService for NullProjectService {
        async fn create_project(
            &self,
            _req: apis::project::CreateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn list_projects(
            &self,
        ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn update_project(
            &self,
            _req: apis::project::UpdateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
    }
}
