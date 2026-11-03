//! `TerminologyService` adapter over the repository ports.
//!
//! Hosts `TerminologyServiceImpl<V, L, I>` — the implementation of
//! `apis::terminology::TerminologyService` that adapts
//! `terminology::TerminologyUsecase` to the API contract.
//! Translation between `apis::terminology::*` and
//! `terminology::usecase::*` happens inline in each trait method.

use async_trait::async_trait;

use apis::terminology::TerminologyKind as ApiKind;
use apis::terminology::{
    BatchCreateCodeItemsRequest, BatchCreateCodeItemsResponse as ApiBatchResp,
    CodeItemListQuery as ApiCodeItemListQuery, CodeItemView,
    CodeListListQuery as ApiCodeListListQuery, CodeListView, CreateCodeItemRequest,
    CreateCodeListRequest, CreateTerminologyVersionRequest, Page as ApiPage, TerminologyApiError,
    TerminologyService, TerminologyVersionView, UpdateCodeItemRequest, UpdateCodeListRequest,
    UpdateTerminologyVersionRequest,
};

use crate::domain::{
    CodeItemListQuery, CodeItemRepository, CodeListListQuery, CodeListRepository, DomainError,
    TerminologyKind, TerminologyVersionRepository,
};
use crate::usecase::{
    BatchCreateCodeItems, CodeItemView as InternalCodeItemView,
    CodeListView as InternalCodeListView, CreateCodeItem, CreateCodeList, CreateTerminologyVersion,
    TerminologyUsecase, TerminologyUsecaseConfig,
    TerminologyVersionView as InternalTerminologyVersionView, UpdateCodeItem, UpdateCodeList,
    UpdateTerminologyVersion, UsecaseError,
};

/// Adapter that implements [`TerminologyService`] on top of a
/// [`TerminologyUsecase`].
///
/// Generic over the three persistence ports so the adapter can be
/// exercised against in-memory fakes in tests and against the
/// PostgreSQL-backed [`TerminologyVersionRepo`](crate::TerminologyVersionRepo) /
/// [`CodeListRepo`](crate::CodeListRepo) /
/// [`CodeItemRepo`](crate::CodeItemRepo) in production.
pub struct TerminologyServiceImpl<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    usecase: TerminologyUsecase<V, L, I>,
}

impl<V, L, I> TerminologyServiceImpl<V, L, I>
where
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
{
    /// Build a new `TerminologyServiceImpl` wrapping the supplied usecase.
    pub fn new(usecase: TerminologyUsecase<V, L, I>) -> Self {
        Self { usecase }
    }

    /// Build a new `TerminologyServiceImpl` from the three
    /// repositories directly. Mirrors the
    /// [`TerminologyUsecase::new`] constructor shape so callers who
    /// already hold the three repos do not need to assemble a
    /// [`TerminologyUsecaseConfig`] first.
    pub fn from_repos(version_repo: V, code_list_repo: L, code_item_repo: I) -> Self {
        Self::new(TerminologyUsecase::new(TerminologyUsecaseConfig {
            version_repo,
            code_list_repo,
            code_item_repo,
        }))
    }
}

/// Map the API's `TerminologyKind` into the domain's
/// `TerminologyKind`. The two enums share the same variants; the
/// match is exhaustive and the compiler enforces it on either side.
fn to_internal_kind(k: ApiKind) -> TerminologyKind {
    match k {
        ApiKind::Sdtm => TerminologyKind::Sdtm,
        ApiKind::Adam => TerminologyKind::Adam,
    }
}

/// Inverse of [`to_internal_kind`].
fn from_internal_kind(k: TerminologyKind) -> ApiKind {
    match k {
        TerminologyKind::Sdtm => ApiKind::Sdtm,
        TerminologyKind::Adam => ApiKind::Adam,
    }
}

/// Project the usecase-layer `TerminologyVersionView` into the
/// API-layer view. Field-for-field because the two structs are
/// kept identical by design.
fn version_view_from_internal(view: InternalTerminologyVersionView) -> TerminologyVersionView {
    TerminologyVersionView {
        id: view.id,
        kind: from_internal_kind(view.kind),
        name: view.name,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

/// See [`version_view_from_internal`].
fn code_list_view_from_internal(view: InternalCodeListView) -> CodeListView {
    CodeListView {
        id: view.id,
        version_id: view.version_id,
        code: view.code,
        extensible: view.extensible,
        name: view.name,
        submission_value: view.submission_value,
        synonym: view.synonym,
        definition: view.definition,
        nci_preferred_term: view.nci_preferred_term,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

/// See [`version_view_from_internal`].
fn code_item_view_from_internal(view: InternalCodeItemView) -> CodeItemView {
    CodeItemView {
        id: view.id,
        codelist_id: view.codelist_id,
        version_id: view.version_id,
        code: view.code,
        submission_value: view.submission_value,
        synonym: view.synonym,
        definition: view.definition,
        nci_preferred_term: view.nci_preferred_term,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

/// Translate a [`UsecaseError`] into the API's [`TerminologyApiError`].
///
/// `UsecaseError::Validation` only ever wraps the validation-only
/// `DomainError` variants; the `unreachable!` arm in the
/// `Repository` branch documents that fact and would fire if a
/// future change ever broke the invariant.
impl From<UsecaseError> for TerminologyApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => TerminologyApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound
                | DomainError::VersionNotFound(_)
                | DomainError::CodeListNotFound(_)
                | DomainError::CodeItemNotFound(_) => TerminologyApiError::NotFound,
                DomainError::DuplicateVersion { kind, name } => {
                    TerminologyApiError::DuplicateVersion {
                        kind: from_internal_kind(kind),
                        name,
                    }
                }
                DomainError::DuplicateCodeList { version_id, code } => {
                    TerminologyApiError::DuplicateCodeList { version_id, code }
                }
                DomainError::DuplicateCodeItem { codelist_id, code } => {
                    TerminologyApiError::DuplicateCodeItem { codelist_id, code }
                }
                DomainError::Repository(msg) => TerminologyApiError::Repository(msg),
                DomainError::EmptyCode
                | DomainError::EmptyName
                | DomainError::InvalidFragment
                | DomainError::InvalidKind(_)
                | DomainError::FkVersionNotFound(_)
                | DomainError::FkCodeListNotFound(_)
                | DomainError::EmptyCodeAtPosition(_) => unreachable!(
                    "domain validation / FK errors are only produced as UsecaseError::Validation"
                ),
            },
        }
    }
}

#[async_trait]
impl<V, L, I> TerminologyService for TerminologyServiceImpl<V, L, I>
where
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
{
    // ---- TerminologyVersion ----

    async fn create_version(
        &self,
        req: CreateTerminologyVersionRequest,
    ) -> Result<TerminologyVersionView, TerminologyApiError> {
        let cmd = CreateTerminologyVersion {
            kind: to_internal_kind(req.kind),
            name: req.name,
        };
        let view = self.usecase.create_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn get_version_by_id(
        &self,
        id: i64,
    ) -> Result<TerminologyVersionView, TerminologyApiError> {
        let view = self
            .usecase
            .get_version_by_id(id)
            .await
            .map_err(TerminologyApiError::from)?;
        Ok(version_view_from_internal(view))
    }

    async fn list_versions(&self) -> Result<Vec<TerminologyVersionView>, TerminologyApiError> {
        let views = self.usecase.list_versions().await?;
        Ok(views.into_iter().map(version_view_from_internal).collect())
    }

    async fn update_version(
        &self,
        req: UpdateTerminologyVersionRequest,
    ) -> Result<TerminologyVersionView, TerminologyApiError> {
        let cmd = UpdateTerminologyVersion {
            id: req.id,
            kind: req.kind.map(to_internal_kind),
            name: req.name,
        };
        let view = self.usecase.update_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn delete_version(&self, id: i64) -> Result<(), TerminologyApiError> {
        self.usecase.delete_version(id).await?;
        Ok(())
    }

    // ---- CodeList ----

    async fn create_code_list(
        &self,
        req: CreateCodeListRequest,
    ) -> Result<CodeListView, TerminologyApiError> {
        let cmd = CreateCodeList {
            version_id: req.version_id,
            code: req.code,
            extensible: req.extensible,
            name: req.name,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        };
        let view = self.usecase.create_code_list(cmd).await?;
        Ok(code_list_view_from_internal(view))
    }

    async fn get_code_list_by_id(&self, id: i64) -> Result<CodeListView, TerminologyApiError> {
        let view = self
            .usecase
            .get_code_list_by_id(id)
            .await
            .map_err(TerminologyApiError::from)?;
        Ok(code_list_view_from_internal(view))
    }

    async fn list_code_lists(
        &self,
        query: ApiCodeListListQuery,
    ) -> Result<ApiPage<CodeListView>, TerminologyApiError> {
        let internal_q = CodeListListQuery {
            version_id: query.version_id,
            fragment: query.fragment,
            offset: query.offset,
            limit: query.limit,
        };
        let page = self.usecase.list_code_lists(internal_q).await?;
        Ok(ApiPage {
            items: page
                .items
                .into_iter()
                .map(code_list_view_from_internal)
                .collect(),
            next_offset: page.next_offset,
        })
    }

    async fn update_code_list(
        &self,
        req: UpdateCodeListRequest,
    ) -> Result<CodeListView, TerminologyApiError> {
        let cmd = UpdateCodeList {
            id: req.id,
            code: req.code,
            extensible: req.extensible,
            name: req.name,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        };
        let view = self.usecase.update_code_list(cmd).await?;
        Ok(code_list_view_from_internal(view))
    }

    async fn delete_code_list(&self, id: i64) -> Result<(), TerminologyApiError> {
        self.usecase.delete_code_list(id).await?;
        Ok(())
    }

    // ---- CodeItem ----

    async fn create_code_item(
        &self,
        req: CreateCodeItemRequest,
    ) -> Result<CodeItemView, TerminologyApiError> {
        let cmd = CreateCodeItem {
            codelist_id: req.codelist_id,
            version_id: req.version_id,
            code: req.code,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        };
        let view = self.usecase.create_code_item(cmd).await?;
        Ok(code_item_view_from_internal(view))
    }

    async fn list_code_items(
        &self,
        query: ApiCodeItemListQuery,
    ) -> Result<ApiPage<CodeItemView>, TerminologyApiError> {
        let internal_q = CodeItemListQuery {
            version_id: query.version_id,
            codelist_id: query.codelist_id,
            fragment: query.fragment,
            offset: query.offset,
            limit: query.limit,
        };
        let page = self.usecase.list_code_items(internal_q).await?;
        Ok(ApiPage {
            items: page
                .items
                .into_iter()
                .map(code_item_view_from_internal)
                .collect(),
            next_offset: page.next_offset,
        })
    }

    async fn list_code_items_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItemView>, TerminologyApiError> {
        let views = self
            .usecase
            .list_code_items_by_version_and_code(version_id, code)
            .await?;
        Ok(views
            .into_iter()
            .map(code_item_view_from_internal)
            .collect())
    }

    async fn update_code_item(
        &self,
        req: UpdateCodeItemRequest,
    ) -> Result<CodeItemView, TerminologyApiError> {
        let cmd = UpdateCodeItem {
            id: req.id,
            code: req.code,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        };
        let view = self.usecase.update_code_item(cmd).await?;
        Ok(code_item_view_from_internal(view))
    }

    async fn delete_code_item(&self, id: i64) -> Result<(), TerminologyApiError> {
        self.usecase.delete_code_item(id).await?;
        Ok(())
    }

    async fn batch_create_code_items(
        &self,
        req: BatchCreateCodeItemsRequest,
    ) -> Result<ApiBatchResp, TerminologyApiError> {
        let cmd = BatchCreateCodeItems {
            codelist_id: req.codelist_id,
            version_id: req.version_id,
            items: req
                .items
                .into_iter()
                .map(|e| CreateCodeItem {
                    codelist_id: req.codelist_id,
                    version_id: req.version_id,
                    code: e.code,
                    submission_value: e.submission_value,
                    synonym: e.synonym,
                    definition: e.definition,
                    nci_preferred_term: e.nci_preferred_term,
                })
                .collect(),
        };
        let resp = self
            .usecase
            .batch_create_code_items(cmd)
            .await
            .map_err(TerminologyApiError::from)?;
        Ok(ApiBatchResp {
            count: resp.count,
            codelist_id: resp.codelist_id,
            version_id: resp.version_id,
        })
    }
}
