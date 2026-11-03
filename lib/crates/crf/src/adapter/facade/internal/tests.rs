//! Facade tests. Wires the in-memory fakes from the usecase
//! layer into `CrfServiceImpl` via `from_usecase`, then drives
//! the public `apis::crf::CrfService` trait — confirming the
//! request → command, view → view, error → error projections.

use std::sync::Arc;

use apis::crf::{
    AnnotationOwner, CreateAnnotationRequest, CreateCrfFormRequest, CreateCrfItemRequest,
    CreateCrfOptionRequest, CreateCrfUnitRequest, CreateCrfVersionRequest,
    CreateDomainAnnotationRequest, CrfApiError, CrfService, GetCrfVersionByIdRequest,
    ListCrfItemsByFormRequest, ListCrfVersionsByProjectRequest, SearchCrfFormsByVersionRequest,
    UpdateCrfFormRequest,
};

use crate::usecase::tests::{
    AcceptProject, InMemoryAnnotations, InMemoryBulkForms, InMemoryDomainAnnotations,
    InMemoryForms, InMemoryItems, InMemoryOptions, InMemoryUnits, InMemoryVersions,
};
use crate::usecase::{CrfUsecase, CrfUsecaseConfig};

use super::CrfServiceImpl;

type TestService = CrfServiceImpl<
    InMemoryVersions,
    InMemoryForms,
    InMemoryItems,
    InMemoryOptions,
    InMemoryUnits,
    InMemoryDomainAnnotations,
    InMemoryAnnotations,
    AcceptProject,
    InMemoryBulkForms,
>;

fn service() -> TestService {
    let usecase: CrfUsecase<
        InMemoryVersions,
        InMemoryForms,
        InMemoryItems,
        InMemoryOptions,
        InMemoryUnits,
        InMemoryDomainAnnotations,
        InMemoryAnnotations,
        AcceptProject,
        InMemoryBulkForms,
    > = CrfUsecase::new(CrfUsecaseConfig {
        version_repo: InMemoryVersions::default(),
        form_repo: InMemoryForms::default(),
        item_repo: InMemoryItems::default(),
        option_repo: InMemoryOptions::default(),
        unit_repo: InMemoryUnits::default(),
        domain_annotation_repo: InMemoryDomainAnnotations::default(),
        annotation_repo: InMemoryAnnotations::default(),
        projects: Arc::new(AcceptProject),
        bulk_form_repo: Arc::new(InMemoryBulkForms),
    });
    CrfServiceImpl::from_usecase(usecase)
}

#[tokio::test]
async fn facade_round_trips_version() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    assert_eq!(v.project_code, "P1");

    let got = svc
        .get_version_by_id(GetCrfVersionByIdRequest { id: v.id })
        .await
        .unwrap();
    assert_eq!(got.id, v.id);

    let listed = svc
        .list_versions_by_project(ListCrfVersionsByProjectRequest {
            project_code: "P1".into(),
        })
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);

    svc.delete_version(v.id).await.unwrap();
}

#[tokio::test]
async fn facade_form_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
            approved: false,
        })
        .await
        .unwrap();
    assert_eq!(f.code, "F1");
    let updated = svc
        .update_form(UpdateCrfFormRequest {
            id: f.id,
            code: None,
            name: Some("Form 1 v2".into()),
            order: None,
            not_submitted: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "Form 1 v2");
    svc.delete_form(f.id).await.unwrap();
}

#[tokio::test]
async fn facade_item_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
            approved: false,
        })
        .await
        .unwrap();
    let it = svc
        .create_item(CreateCrfItemRequest {
            form_id: f.id,
            code: "I1".into(),
            name: "Item 1".into(),
            kind: apis::crf::CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let items = svc
        .list_items_by_form(ListCrfItemsByFormRequest { form_id: f.id })
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, it.id);
}

#[tokio::test]
async fn facade_option_unit_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
            approved: false,
        })
        .await
        .unwrap();
    // Use Text kind so we can attach options without the
    // shape rule kicking in.
    let it = svc
        .create_item(CreateCrfItemRequest {
            form_id: f.id,
            code: "I1".into(),
            name: "Item 1".into(),
            kind: apis::crf::CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let _o = svc
        .create_option(CreateCrfOptionRequest {
            item_id: it.id,
            value: "yes".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
    let _u = svc
        .create_unit(CreateCrfUnitRequest {
            item_id: it.id,
            value: "mg".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn facade_selection_without_options_returns_kind_shape_violation() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
            approved: false,
        })
        .await
        .unwrap();
    let err = svc
        .create_item(CreateCrfItemRequest {
            form_id: f.id,
            code: "S".into(),
            name: "Status".into(),
            kind: apis::crf::CrfItemKind::Selection,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, CrfApiError::KindShapeViolation { .. }));
}

#[tokio::test]
async fn facade_search_rejects_empty_fragment() {
    let svc = service();
    let err = svc
        .search_forms_by_version(SearchCrfFormsByVersionRequest {
            version_id: 1,
            fragment: "  ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, CrfApiError::EmptySearchFragment));
}

#[tokio::test]
async fn facade_domain_annotation_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
            approved: false,
        })
        .await
        .unwrap();
    let d = svc
        .create_domain_annotation(CreateDomainAnnotationRequest {
            form_id: f.id,
            name: "Required".into(),
            description: "must supply".into(),
        })
        .await
        .unwrap();
    assert_eq!(d.name, "Required");
    svc.delete_domain_annotation(d.id).await.unwrap();
}

#[tokio::test]
async fn facade_annotation_polymorphic() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
            approved: false,
        })
        .await
        .unwrap();
    let d = svc
        .create_domain_annotation(CreateDomainAnnotationRequest {
            form_id: f.id,
            name: "Required".into(),
            description: "".into(),
        })
        .await
        .unwrap();
    let a = svc
        .create_annotation(CreateAnnotationRequest {
            domain_annotation_id: d.id,
            content: "must supply".into(),
            assign: false,
            owner: AnnotationOwner::Form(f.id),
        })
        .await
        .unwrap();
    assert!(matches!(a.owner, AnnotationOwner::Form(_)));
    svc.delete_annotation(a.id).await.unwrap();
}

#[tokio::test]
async fn facade_bulk_create_form_round_trip() {
    // Round-trips the bulk create through the apis wire shape and
    // confirms the projections on the response — every item
    // stamped with the freshly-allocated form_id, the response
    // shape mirrors the request items list.
    use apis::crf::{BulkCreateCrfItemInput, CrfItemKind as ApiKind};

    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let r = svc
        .bulk_create_form(apis::crf::BulkCreateCrfFormRequest {
            form: apis::crf::CreateCrfFormRequest {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
                approved: false,
            },
            items: vec![
                BulkCreateCrfItemInput {
                    item: apis::crf::CreateCrfItemRequest {
                        form_id: 0,
                        code: "I1".into(),
                        name: "Item 1".into(),
                        kind: ApiKind::Selection,
                        order: 0,
                        not_submitted: false,
                    },
                    options: vec![
                        apis::crf::CreateCrfOptionRequest {
                            item_id: 0,
                            value: "yes".into(),
                            not_submitted: false,
                        },
                        apis::crf::CreateCrfOptionRequest {
                            item_id: 0,
                            value: "no".into(),
                            not_submitted: false,
                        },
                    ],
                    units: vec![],
                },
                BulkCreateCrfItemInput {
                    item: apis::crf::CreateCrfItemRequest {
                        form_id: 0,
                        code: "I2".into(),
                        name: "Item 2".into(),
                        kind: ApiKind::Text,
                        order: 1,
                        not_submitted: false,
                    },
                    options: vec![],
                    units: vec![apis::crf::CreateCrfUnitRequest {
                        item_id: 0,
                        value: "mg".into(),
                        not_submitted: false,
                    }],
                },
            ],
        })
        .await
        .unwrap();
    assert_eq!(r.form.code, "F1");
    assert_eq!(r.form.version_id, v.id);
    assert_eq!(r.items.len(), 2);
    assert_eq!(r.items[0].code, "I1");
    assert_eq!(r.items[1].code, "I2");
    assert_eq!(r.items[0].form_id, r.form.id);
    assert_eq!(r.items[1].form_id, r.form.id);
    assert!(r.items[0].id > 0);
    assert!(r.items[1].id > 0);
}

#[tokio::test]
async fn facade_bulk_create_form_rejects_text_with_options() {
    // The wire shape `kind` enum is distinct from the domain's,
    // so it must be routed through the kind-shape rule at the
    // usecase. A Text item with options should surface as
    // `KindShapeViolation` on the wire.
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let err = svc
        .bulk_create_form(apis::crf::BulkCreateCrfFormRequest {
            form: apis::crf::CreateCrfFormRequest {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
                approved: false,
            },
            items: vec![apis::crf::BulkCreateCrfItemInput {
                item: apis::crf::CreateCrfItemRequest {
                    form_id: 0,
                    code: "I1".into(),
                    name: "Item 1".into(),
                    kind: apis::crf::CrfItemKind::Text,
                    order: 0,
                    not_submitted: false,
                },
                options: vec![apis::crf::CreateCrfOptionRequest {
                    item_id: 0,
                    value: "yes".into(),
                    not_submitted: false,
                }],
                units: vec![],
            }],
        })
        .await
        .unwrap_err();
    assert!(matches!(err, CrfApiError::KindShapeViolation { .. }));
}

#[tokio::test]
async fn facade_get_form_detail_round_trip() {
    // Drives the public apis wire shape end-to-end and
    // confirms the projections on the response.
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
            approved: false,
        })
        .await
        .unwrap();
    let form_id = f.id;
    let i1 = svc
        .create_item(CreateCrfItemRequest {
            form_id,
            code: "I1".into(),
            name: "Item 1".into(),
            kind: apis::crf::CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let item1_id = i1.id;
    svc.create_option(CreateCrfOptionRequest {
        item_id: item1_id,
        value: "yes".into(),
        not_submitted: false,
    })
    .await
    .unwrap();
    svc.create_unit(CreateCrfUnitRequest {
        item_id: item1_id,
        value: "mg".into(),
        not_submitted: false,
    })
    .await
    .unwrap();
    let i2 = svc
        .create_item(CreateCrfItemRequest {
            form_id,
            code: "I2".into(),
            name: "Item 2".into(),
            kind: apis::crf::CrfItemKind::Text,
            order: 1,
            not_submitted: false,
        })
        .await
        .unwrap();
    let item2_id = i2.id;
    let d = svc
        .create_domain_annotation(CreateDomainAnnotationRequest {
            form_id,
            name: "Hint".into(),
            description: "".into(),
        })
        .await
        .unwrap();
    svc.create_annotation(CreateAnnotationRequest {
        domain_annotation_id: d.id,
        content: "form-level".into(),
        assign: false,
        owner: AnnotationOwner::Form(form_id),
    })
    .await
    .unwrap();
    svc.create_annotation(CreateAnnotationRequest {
        domain_annotation_id: d.id,
        content: "item-1".into(),
        assign: false,
        owner: AnnotationOwner::Item(item1_id),
    })
    .await
    .unwrap();

    let detail = svc
        .get_form_detail(apis::crf::GetCrfFormDetailRequest { form_id })
        .await
        .unwrap();

    assert_eq!(detail.form.id, form_id);
    assert_eq!(detail.items.len(), 2);
    assert_eq!(detail.items[0].item.id, item1_id);
    assert_eq!(detail.items[0].options.len(), 1);
    assert_eq!(detail.items[0].units.len(), 1);
    assert_eq!(detail.items[0].annotations.len(), 1);
    assert_eq!(detail.items[1].item.id, item2_id);
    assert_eq!(detail.items[1].options.len(), 0);
    assert_eq!(detail.form_annotations.len(), 1);
    assert_eq!(detail.domain_annotations.len(), 1);
}

#[tokio::test]
async fn facade_get_form_detail_missing_form() {
    let svc = service();
    let err = svc
        .get_form_detail(apis::crf::GetCrfFormDetailRequest { form_id: 9_999_999 })
        .await
        .unwrap_err();
    assert!(matches!(err, CrfApiError::CrfFormNotFound(9_999_999)));
}
