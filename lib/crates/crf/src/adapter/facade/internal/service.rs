use std::sync::Arc;

use async_trait::async_trait;

use apis::crf::{
    AnnotationOwner as ApiAnnotationOwner, AnnotationView as ApiAnnotationView,
    CreateAnnotationRequest, CreateCrfFormRequest, CreateCrfItemRequest, CreateCrfOptionRequest,
    CreateCrfUnitRequest, CreateCrfVersionRequest, CreateDomainAnnotationRequest, CrfApiError,
    CrfFormDetailView as ApiCrfFormDetailView, CrfFormView as ApiCrfFormView,
    CrfItemDetailView as ApiCrfItemDetailView, CrfItemView as ApiCrfItemView,
    CrfOptionDetailView as ApiCrfOptionDetailView, CrfOptionView as ApiCrfOptionView, CrfService,
    CrfUnitDetailView as ApiCrfUnitDetailView, CrfUnitView as ApiCrfUnitView,
    CrfVersionView as ApiCrfVersionView, DomainAnnotationView as ApiDomainAnnotationView,
    GetAnnotationByIdRequest, GetCrfFormByIdRequest, GetCrfFormDetailRequest,
    GetCrfItemByIdRequest, GetCrfOptionByIdRequest, GetCrfUnitByIdRequest,
    GetCrfVersionByIdRequest, GetDomainAnnotationByIdRequest, ListAnnotationsByFormRequest,
    ListAnnotationsByItemRequest, ListAnnotationsByOptionRequest, ListAnnotationsByUnitRequest,
    ListCrfFormsByVersionRequest, ListCrfItemsByFormRequest, ListCrfOptionsByItemRequest,
    ListCrfUnitsByItemRequest, ListCrfVersionsByProjectRequest, ListDomainAnnotationsByFormRequest,
    SearchAnnotationsByVersionRequest, SearchCrfFormsByVersionRequest,
    SearchCrfItemsByVersionRequest, SearchCrfOptionsByVersionRequest,
    SearchCrfUnitsByVersionRequest, SearchDomainAnnotationsByVersionRequest,
    UpdateAnnotationRequest, UpdateCrfFormRequest, UpdateCrfItemRequest, UpdateCrfOptionRequest,
    UpdateCrfUnitRequest, UpdateCrfVersionRequest, UpdateDomainAnnotationRequest,
};

use crate::domain::{
    AnnotationOwner, AnnotationRepository, CrfBulkFormRepository, CrfFormRepository,
    CrfItemRepository, CrfOptionRepository, CrfUnitRepository, CrfVersionRepository,
    DomainAnnotationRepository, DomainError, ProjectLookup,
};
use crate::usecase::{CrfUsecase, CrfUsecaseConfig, UsecaseError};

pub struct CrfServiceImpl<
    V: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: AnnotationRepository,
    P: ProjectLookup,
    B: CrfBulkFormRepository,
> {
    usecase: CrfUsecase<V, F, I, O, U, Da, A, P, B>,
}

impl<V, F, I, O, U, Da, A, P, B> CrfServiceImpl<V, F, I, O, U, Da, A, P, B>
where
    V: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: AnnotationRepository,
    P: ProjectLookup,
    B: CrfBulkFormRepository,
{
    pub fn from_usecase(usecase: CrfUsecase<V, F, I, O, U, Da, A, P, B>) -> Self {
        Self { usecase }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_repos(
        version_repo: V,
        form_repo: F,
        item_repo: I,
        option_repo: O,
        unit_repo: U,
        domain_annotation_repo: Da,
        annotation_repo: A,
        projects: Arc<P>,
        bulk_form_repo: Arc<B>,
    ) -> Self {
        Self::from_usecase(CrfUsecase::new(CrfUsecaseConfig {
            version_repo,
            form_repo,
            item_repo,
            option_repo,
            unit_repo,
            domain_annotation_repo,
            annotation_repo,
            projects,
            bulk_form_repo,
        }))
    }
}

#[async_trait]
impl<V, F, I, O, U, Da, A, P, B> CrfService for CrfServiceImpl<V, F, I, O, U, Da, A, P, B>
where
    V: CrfVersionRepository + 'static,
    F: CrfFormRepository + 'static,
    I: CrfItemRepository + 'static,
    O: CrfOptionRepository + 'static,
    U: CrfUnitRepository + 'static,
    Da: DomainAnnotationRepository + 'static,
    A: AnnotationRepository + 'static,
    P: ProjectLookup + 'static,
    B: CrfBulkFormRepository + 'static,
{
    // ---- CrfVersion ----

    async fn create_version(
        &self,
        req: CreateCrfVersionRequest,
    ) -> Result<ApiCrfVersionView, CrfApiError> {
        self.usecase
            .create_version(crate::usecase::CreateCrfVersion {
                project_code: req.project_code,
                name: req.name,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn get_version_by_id(
        &self,
        req: GetCrfVersionByIdRequest,
    ) -> Result<ApiCrfVersionView, CrfApiError> {
        self.usecase
            .get_version_by_id(req.id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn list_versions_by_project(
        &self,
        req: ListCrfVersionsByProjectRequest,
    ) -> Result<Vec<ApiCrfVersionView>, CrfApiError> {
        self.usecase
            .list_versions_by_project(&req.project_code)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn update_version(
        &self,
        req: UpdateCrfVersionRequest,
    ) -> Result<ApiCrfVersionView, CrfApiError> {
        self.usecase
            .update_version(crate::usecase::UpdateCrfVersion {
                id: req.id,
                name: req.name,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn delete_version(&self, id: i64) -> Result<(), CrfApiError> {
        self.usecase.delete_version(id).await.map_err(map_error)
    }

    // ---- CrfForm ----

    async fn create_form(&self, req: CreateCrfFormRequest) -> Result<ApiCrfFormView, CrfApiError> {
        self.usecase
            .create_form(crate::usecase::CreateCrfForm {
                version_id: req.version_id,
                code: req.code,
                name: req.name,
                order: req.order,
                not_submitted: req.not_submitted,
                approved: req.approved,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn bulk_create_form(
        &self,
        req: apis::crf::BulkCreateCrfFormRequest,
    ) -> Result<apis::crf::BulkCreateCrfFormResult, CrfApiError> {
        self.usecase
            .create_bulk_form(crate::usecase::CreateCrfBulkForm {
                form: crate::usecase::CreateCrfForm {
                    version_id: req.form.version_id,
                    code: req.form.code,
                    name: req.form.name,
                    order: req.form.order,
                    not_submitted: req.form.not_submitted,
                    approved: req.form.approved,
                },
                items: req
                    .items
                    .into_iter()
                    .map(|bi| crate::usecase::CreateCrfBulkItem {
                        item: crate::usecase::CreateCrfItem {
                            form_id: 0,
                            code: bi.item.code,
                            name: bi.item.name,
                            kind: crate::domain::CrfItemKind::from(bi.item.kind),
                            order: bi.item.order,
                            not_submitted: bi.item.not_submitted,
                        },
                        options: bi
                            .options
                            .into_iter()
                            .map(|o| crate::usecase::CreateCrfOption {
                                item_id: 0,
                                value: o.value,
                                not_submitted: o.not_submitted,
                            })
                            .collect(),
                        units: bi
                            .units
                            .into_iter()
                            .map(|u| crate::usecase::CreateCrfUnit {
                                item_id: 0,
                                value: u.value,
                                not_submitted: u.not_submitted,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .await
            .map(|r| apis::crf::BulkCreateCrfFormResult {
                form: r.form.into(),
                items: r.items.into_iter().map(Into::into).collect(),
            })
            .map_err(map_error)
    }

    async fn get_form_by_id(
        &self,
        req: GetCrfFormByIdRequest,
    ) -> Result<ApiCrfFormView, CrfApiError> {
        self.usecase
            .get_form_by_id(req.id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn get_form_detail(
        &self,
        req: GetCrfFormDetailRequest,
    ) -> Result<ApiCrfFormDetailView, CrfApiError> {
        self.usecase
            .get_form_detail(req.form_id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn list_forms_by_version(
        &self,
        req: ListCrfFormsByVersionRequest,
    ) -> Result<Vec<ApiCrfFormView>, CrfApiError> {
        self.usecase
            .list_forms_by_version(req.version_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn update_form(&self, req: UpdateCrfFormRequest) -> Result<ApiCrfFormView, CrfApiError> {
        self.usecase
            .update_form(crate::usecase::UpdateCrfForm {
                id: req.id,
                code: req.code,
                name: req.name,
                order: req.order,
                not_submitted: req.not_submitted,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn delete_form(&self, id: i64) -> Result<(), CrfApiError> {
        self.usecase.delete_form(id).await.map_err(map_error)
    }

    async fn set_approved(
        &self,
        req: apis::crf::SetCrfApprovedRequest,
    ) -> Result<ApiCrfFormView, CrfApiError> {
        self.usecase
            .set_approved(crate::usecase::SetCrfApproved {
                id: req.id,
                approved: req.approved,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    // ---- CrfItem ----

    async fn create_item(&self, req: CreateCrfItemRequest) -> Result<ApiCrfItemView, CrfApiError> {
        self.usecase
            .create_item(crate::usecase::CreateCrfItem {
                form_id: req.form_id,
                code: req.code,
                name: req.name,
                kind: req.kind.into(),
                order: req.order,
                not_submitted: req.not_submitted,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn get_item_by_id(
        &self,
        req: GetCrfItemByIdRequest,
    ) -> Result<ApiCrfItemView, CrfApiError> {
        self.usecase
            .get_item_by_id(req.id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn list_items_by_form(
        &self,
        req: ListCrfItemsByFormRequest,
    ) -> Result<Vec<ApiCrfItemView>, CrfApiError> {
        self.usecase
            .list_items_by_form(req.form_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn update_item(&self, req: UpdateCrfItemRequest) -> Result<ApiCrfItemView, CrfApiError> {
        self.usecase
            .update_item(crate::usecase::UpdateCrfItem {
                id: req.id,
                code: req.code,
                name: req.name,
                kind: req.kind.map(Into::into),
                order: req.order,
                not_submitted: req.not_submitted,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn delete_item(&self, id: i64) -> Result<(), CrfApiError> {
        self.usecase.delete_item(id).await.map_err(map_error)
    }

    // ---- CrfOption ----

    async fn create_option(
        &self,
        req: CreateCrfOptionRequest,
    ) -> Result<ApiCrfOptionView, CrfApiError> {
        self.usecase
            .create_option(crate::usecase::CreateCrfOption {
                item_id: req.item_id,
                value: req.value,
                not_submitted: req.not_submitted,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn get_option_by_id(
        &self,
        req: GetCrfOptionByIdRequest,
    ) -> Result<ApiCrfOptionView, CrfApiError> {
        self.usecase
            .get_option_by_id(req.id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn list_options_by_item(
        &self,
        req: ListCrfOptionsByItemRequest,
    ) -> Result<Vec<ApiCrfOptionView>, CrfApiError> {
        self.usecase
            .list_options_by_item(req.item_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn update_option(
        &self,
        req: UpdateCrfOptionRequest,
    ) -> Result<ApiCrfOptionView, CrfApiError> {
        self.usecase
            .update_option(crate::usecase::UpdateCrfOption {
                id: req.id,
                value: req.value,
                not_submitted: req.not_submitted,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn delete_option(&self, id: i64) -> Result<(), CrfApiError> {
        self.usecase.delete_option(id).await.map_err(map_error)
    }

    // ---- CrfUnit ----

    async fn create_unit(&self, req: CreateCrfUnitRequest) -> Result<ApiCrfUnitView, CrfApiError> {
        self.usecase
            .create_unit(crate::usecase::CreateCrfUnit {
                item_id: req.item_id,
                value: req.value,
                not_submitted: req.not_submitted,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn get_unit_by_id(
        &self,
        req: GetCrfUnitByIdRequest,
    ) -> Result<ApiCrfUnitView, CrfApiError> {
        self.usecase
            .get_unit_by_id(req.id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn list_units_by_item(
        &self,
        req: ListCrfUnitsByItemRequest,
    ) -> Result<Vec<ApiCrfUnitView>, CrfApiError> {
        self.usecase
            .list_units_by_item(req.item_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn update_unit(&self, req: UpdateCrfUnitRequest) -> Result<ApiCrfUnitView, CrfApiError> {
        self.usecase
            .update_unit(crate::usecase::UpdateCrfUnit {
                id: req.id,
                value: req.value,
                not_submitted: req.not_submitted,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn delete_unit(&self, id: i64) -> Result<(), CrfApiError> {
        self.usecase.delete_unit(id).await.map_err(map_error)
    }

    // ---- DomainAnnotation ----

    async fn create_domain_annotation(
        &self,
        req: CreateDomainAnnotationRequest,
    ) -> Result<ApiDomainAnnotationView, CrfApiError> {
        self.usecase
            .create_domain_annotation(crate::usecase::CreateDomainAnnotation {
                form_id: req.form_id,
                name: req.name,
                description: req.description,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn get_domain_annotation_by_id(
        &self,
        req: GetDomainAnnotationByIdRequest,
    ) -> Result<ApiDomainAnnotationView, CrfApiError> {
        self.usecase
            .get_domain_annotation_by_id(req.id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn list_domain_annotations_by_form(
        &self,
        req: ListDomainAnnotationsByFormRequest,
    ) -> Result<Vec<ApiDomainAnnotationView>, CrfApiError> {
        self.usecase
            .list_domain_annotations_by_form(req.form_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn update_domain_annotation(
        &self,
        req: UpdateDomainAnnotationRequest,
    ) -> Result<ApiDomainAnnotationView, CrfApiError> {
        self.usecase
            .update_domain_annotation(crate::usecase::UpdateDomainAnnotation {
                id: req.id,
                name: req.name,
                description: req.description,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn delete_domain_annotation(&self, id: i64) -> Result<(), CrfApiError> {
        self.usecase
            .delete_domain_annotation(id)
            .await
            .map_err(map_error)
    }

    // ---- Annotation ----

    async fn create_annotation(
        &self,
        req: CreateAnnotationRequest,
    ) -> Result<ApiAnnotationView, CrfApiError> {
        self.usecase
            .create_annotation(crate::usecase::CreateAnnotation {
                domain_annotation_id: req.domain_annotation_id,
                content: req.content,
                assign: req.assign,
                owner: annotation_owner_from_api(req.owner),
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn get_annotation_by_id(
        &self,
        req: GetAnnotationByIdRequest,
    ) -> Result<ApiAnnotationView, CrfApiError> {
        self.usecase
            .get_annotation_by_id(req.id)
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn list_annotations_by_form(
        &self,
        req: ListAnnotationsByFormRequest,
    ) -> Result<Vec<ApiAnnotationView>, CrfApiError> {
        self.usecase
            .list_annotations_by_form(req.form_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn list_annotations_by_item(
        &self,
        req: ListAnnotationsByItemRequest,
    ) -> Result<Vec<ApiAnnotationView>, CrfApiError> {
        self.usecase
            .list_annotations_by_item(req.item_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn list_annotations_by_option(
        &self,
        req: ListAnnotationsByOptionRequest,
    ) -> Result<Vec<ApiAnnotationView>, CrfApiError> {
        self.usecase
            .list_annotations_by_option(req.option_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn list_annotations_by_unit(
        &self,
        req: ListAnnotationsByUnitRequest,
    ) -> Result<Vec<ApiAnnotationView>, CrfApiError> {
        self.usecase
            .list_annotations_by_unit(req.unit_id)
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn update_annotation(
        &self,
        req: UpdateAnnotationRequest,
    ) -> Result<ApiAnnotationView, CrfApiError> {
        self.usecase
            .update_annotation(crate::usecase::UpdateAnnotation {
                id: req.id,
                content: req.content,
                assign: req.assign,
            })
            .await
            .map(Into::into)
            .map_err(map_error)
    }

    async fn delete_annotation(&self, id: i64) -> Result<(), CrfApiError> {
        self.usecase.delete_annotation(id).await.map_err(map_error)
    }

    // ---- Search ----

    async fn search_forms_by_version(
        &self,
        req: SearchCrfFormsByVersionRequest,
    ) -> Result<Vec<ApiCrfFormView>, CrfApiError> {
        self.usecase
            .search_forms_by_version(crate::usecase::SearchCrfFormsByVersion {
                version_id: req.version_id,
                fragment: req.fragment,
            })
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn search_items_by_version(
        &self,
        req: SearchCrfItemsByVersionRequest,
    ) -> Result<Vec<ApiCrfItemView>, CrfApiError> {
        self.usecase
            .search_items_by_version(crate::usecase::SearchCrfItemsByVersion {
                version_id: req.version_id,
                fragment: req.fragment,
            })
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn search_options_by_version(
        &self,
        req: SearchCrfOptionsByVersionRequest,
    ) -> Result<Vec<ApiCrfOptionView>, CrfApiError> {
        self.usecase
            .search_options_by_version(crate::usecase::SearchCrfOptionsByVersion {
                version_id: req.version_id,
                fragment: req.fragment,
            })
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn search_units_by_version(
        &self,
        req: SearchCrfUnitsByVersionRequest,
    ) -> Result<Vec<ApiCrfUnitView>, CrfApiError> {
        self.usecase
            .search_units_by_version(crate::usecase::SearchCrfUnitsByVersion {
                version_id: req.version_id,
                fragment: req.fragment,
            })
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn search_domain_annotations_by_version(
        &self,
        req: SearchDomainAnnotationsByVersionRequest,
    ) -> Result<Vec<ApiDomainAnnotationView>, CrfApiError> {
        self.usecase
            .search_domain_annotations_by_version(
                crate::usecase::SearchDomainAnnotationsByVersion {
                    version_id: req.version_id,
                    fragment: req.fragment,
                },
            )
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }

    async fn search_annotations_by_version(
        &self,
        req: SearchAnnotationsByVersionRequest,
    ) -> Result<Vec<ApiAnnotationView>, CrfApiError> {
        self.usecase
            .search_annotations_by_version(crate::usecase::SearchAnnotationsByVersion {
                version_id: req.version_id,
                fragment: req.fragment,
            })
            .await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_error)
    }
}

// ---- From<View> impls for Api conversions ----

impl From<crate::usecase::CrfVersionView> for ApiCrfVersionView {
    fn from(v: crate::usecase::CrfVersionView) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<crate::usecase::CrfFormView> for ApiCrfFormView {
    fn from(f: crate::usecase::CrfFormView) -> Self {
        Self {
            id: f.id,
            version_id: f.version_id,
            code: f.code,
            name: f.name,
            order: f.order,
            not_submitted: f.not_submitted,
            approved: f.approved,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}

impl From<crate::usecase::CrfItemView> for ApiCrfItemView {
    fn from(i: crate::usecase::CrfItemView) -> Self {
        Self {
            id: i.id,
            form_id: i.form_id,
            code: i.code,
            name: i.name,
            kind: i.kind.into(),
            order: i.order,
            not_submitted: i.not_submitted,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

impl From<crate::usecase::CrfOptionView> for ApiCrfOptionView {
    fn from(o: crate::usecase::CrfOptionView) -> Self {
        Self {
            id: o.id,
            item_id: o.item_id,
            value: o.value,
            not_submitted: o.not_submitted,
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}

impl From<crate::usecase::CrfUnitView> for ApiCrfUnitView {
    fn from(u: crate::usecase::CrfUnitView) -> Self {
        Self {
            id: u.id,
            item_id: u.item_id,
            value: u.value,
            not_submitted: u.not_submitted,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<crate::usecase::DomainAnnotationView> for ApiDomainAnnotationView {
    fn from(d: crate::usecase::DomainAnnotationView) -> Self {
        Self {
            id: d.id,
            form_id: d.form_id,
            name: d.name,
            description: d.description,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

impl From<crate::usecase::AnnotationView> for ApiAnnotationView {
    fn from(a: crate::usecase::AnnotationView) -> Self {
        Self {
            id: a.id,
            domain_annotation_id: a.domain_annotation_id,
            content: a.content,
            assign: a.assign,
            owner: annotation_owner_to_api(a.owner),
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

impl From<crate::usecase::CrfFormDetailView> for ApiCrfFormDetailView {
    fn from(v: crate::usecase::CrfFormDetailView) -> Self {
        Self {
            form: v.form.into(),
            form_annotations: v.form_annotations.into_iter().map(Into::into).collect(),
            items: v.items.into_iter().map(Into::into).collect(),
            domain_annotations: v.domain_annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::CrfItemDetailView> for ApiCrfItemDetailView {
    fn from(v: crate::usecase::CrfItemDetailView) -> Self {
        Self {
            item: v.item.into(),
            options: v.options.into_iter().map(Into::into).collect(),
            units: v.units.into_iter().map(Into::into).collect(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::CrfOptionDetailView> for ApiCrfOptionDetailView {
    fn from(v: crate::usecase::CrfOptionDetailView) -> Self {
        Self {
            option: v.option.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::CrfUnitDetailView> for ApiCrfUnitDetailView {
    fn from(v: crate::usecase::CrfUnitDetailView) -> Self {
        Self {
            unit: v.unit.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

// ---- CrfItemKind conversion ----

impl From<apis::crf::CrfItemKind> for crate::domain::CrfItemKind {
    fn from(k: apis::crf::CrfItemKind) -> Self {
        use apis::crf::CrfItemKind as A;
        match k {
            A::Text => Self::Text,
            A::Selection => Self::Selection,
            A::Checkbox => Self::Checkbox,
            A::Datetime => Self::Datetime,
            A::Label => Self::Label,
        }
    }
}

impl From<crate::domain::CrfItemKind> for apis::crf::CrfItemKind {
    fn from(k: crate::domain::CrfItemKind) -> Self {
        use crate::domain::CrfItemKind as D;
        match k {
            D::Text => Self::Text,
            D::Selection => Self::Selection,
            D::Checkbox => Self::Checkbox,
            D::Datetime => Self::Datetime,
            D::Label => Self::Label,
        }
    }
}

// ---- AnnotationOwner conversion ----

fn annotation_owner_from_api(o: ApiAnnotationOwner) -> AnnotationOwner {
    match o {
        ApiAnnotationOwner::Form(id) => AnnotationOwner::Form { id },
        ApiAnnotationOwner::Item(id) => AnnotationOwner::Item { id },
        ApiAnnotationOwner::Option(id) => AnnotationOwner::Option { id },
        ApiAnnotationOwner::Unit(id) => AnnotationOwner::Unit { id },
    }
}

fn annotation_owner_to_api(o: AnnotationOwner) -> ApiAnnotationOwner {
    match o {
        AnnotationOwner::Form { id } => ApiAnnotationOwner::Form(id),
        AnnotationOwner::Item { id } => ApiAnnotationOwner::Item(id),
        AnnotationOwner::Option { id } => ApiAnnotationOwner::Option(id),
        AnnotationOwner::Unit { id } => ApiAnnotationOwner::Unit(id),
    }
}

// ---- error mapping ----

fn map_error(err: UsecaseError) -> CrfApiError {
    use CrfApiError as A;
    use DomainError as D;
    match err {
        UsecaseError::Validation(D::KindShapeViolation { kind, field }) => A::KindShapeViolation {
            kind: kind.into(),
            field,
        },
        UsecaseError::Validation(d) => A::Validation(d.to_string()),
        UsecaseError::Repository(d) => map_domain_error(d),
    }
}

fn map_domain_error(d: DomainError) -> CrfApiError {
    use CrfApiError as A;
    use DomainError as D;
    match d {
        D::NotFound => A::NotFound,
        D::ProjectNotFound(c) => A::ProjectNotFound(c),
        D::CrfVersionNotFound(id) => A::CrfVersionNotFound(id),
        D::CrfFormNotFound(id) => A::CrfFormNotFound(id),
        D::CrfItemNotFound(id) => A::CrfItemNotFound(id),
        D::CrfOptionNotFound(id) => A::CrfOptionNotFound(id),
        D::CrfUnitNotFound(id) => A::CrfUnitNotFound(id),
        D::DomainAnnotationNotFound(id) => A::DomainAnnotationNotFound(id),
        D::AnnotationNotFound(id) => A::AnnotationNotFound(id),
        D::DuplicateCrfVersion { project_code, name } => {
            A::DuplicateCrfVersion { project_code, name }
        }
        D::DuplicateCrfForm { version_id, code } => A::DuplicateCrfForm { version_id, code },
        D::DuplicateCrfItem { form_id, code } => A::DuplicateCrfItem { form_id, code },
        D::DuplicateDomainAnnotation { form_id, name } => {
            A::DuplicateDomainAnnotation { form_id, name }
        }
        D::FkCrfVersionNotFound(id) => A::FkCrfVersionNotFound(id),
        D::FkCrfFormNotFound(id) => A::FkCrfFormNotFound(id),
        D::FkCrfItemNotFound(id) => A::FkCrfItemNotFound(id),
        D::FkCrfOptionNotFound(id) => A::FkCrfOptionNotFound(id),
        D::FkCrfUnitNotFound(id) => A::FkCrfUnitNotFound(id),
        D::FkDomainAnnotationNotFound(id) => A::FkDomainAnnotationNotFound(id),
        // The validating constructor's empty-* / shape variants
        // reach here only as "contract broken upstream" —
        // surface them as Repository.
        D::EmptyProjectCode
        | D::EmptyName
        | D::EmptyCode
        | D::EmptyValue
        | D::EmptyContent
        | D::InvalidCrfItemKind(_) => A::Repository(d.to_string()),
        D::KindShapeViolation { kind, field } => A::KindShapeViolation {
            kind: kind.into(),
            field,
        },
        D::Repository(msg) => {
            if msg.contains("search fragment cannot be empty") {
                A::EmptySearchFragment
            } else {
                A::Repository(msg)
            }
        }
    }
}
