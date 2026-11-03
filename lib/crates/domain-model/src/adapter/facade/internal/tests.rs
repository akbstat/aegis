//! Unit tests for `DomainModelServiceImpl`.
//!
//! Wires the adapter on top of three in-memory repositories so the
//! behaviour is exercised without touching PostgreSQL. The
//! in-memory repositories are private to this test module — they
//! are intentionally not shared with the usecase tests so each
//! layer can evolve independently.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use apis::domain_model::{
    CreateSdtmDomainRequest, CreateSdtmVariableRequest, CreateSdtmVersionRequest,
    DomainCategory as ApiCategory, DomainModelApiError, DomainModelService,
    SdtmDomainDescription as ApiSdtmDomainDescription,
    SdtmDomainDescriptionDetail as ApiSdtmDomainDescriptionDetail, SdtmRole as ApiSdtmRole,
    SdtmVariableControlled as ApiSdtmVariableControlled, SdtmVariableCore as ApiSdtmVariableCore,
    SdtmVariableDescription as ApiSdtmVariableDescription,
    SdtmVariableDescriptionDetail as ApiSdtmVariableDescriptionDetail,
    SdtmVariableType as ApiSdtmVariableType, UpdateSdtmDomainRequest, UpdateSdtmVariableRequest,
    UpdateSdtmVersionRequest,
};

use crate::domain::{
    DomainError, SdtmDomain, SdtmDomainNew, SdtmDomainRepository, SdtmDomainUpdate, SdtmVariable,
    SdtmVariableNew, SdtmVariableRepository, SdtmVariableUpdate, SdtmVersion, SdtmVersionNew,
    SdtmVersionRepository, SdtmVersionUpdate,
};

use super::DomainModelServiceImpl;

// ---- helpers ----

fn epoch() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

fn api_domain_desc(lang: &str, desc: &str, structure: &str) -> ApiSdtmDomainDescription {
    ApiSdtmDomainDescription {
        lang: lang.to_string(),
        details: ApiSdtmDomainDescriptionDetail {
            description: desc.to_string(),
            structure: structure.to_string(),
        },
    }
}

fn api_variable_desc(lang: &str, label: &str) -> ApiSdtmVariableDescription {
    ApiSdtmVariableDescription {
        lang: lang.to_string(),
        details: ApiSdtmVariableDescriptionDetail {
            label: label.to_string(),
        },
    }
}

// ---- in-memory repositories (shared by every test below) ----

#[derive(Default)]
struct InMemorySdtmVersionRepo {
    inner: Arc<Mutex<Vec<SdtmVersion>>>,
    next_id: AtomicI64,
}

impl InMemorySdtmVersionRepo {
    fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SdtmVersionRepository for InMemorySdtmVersionRepo {
    async fn create(&self, input: SdtmVersionNew) -> Result<SdtmVersion, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter().any(|v| v.name == input.name) {
            return Err(DomainError::DuplicateSdtmVersion {
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let v = SdtmVersion::for_repository(id, input.name, epoch(), epoch());
        g.push(v.clone());
        Ok(v)
    }
    async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError> {
        Ok(self.inner.lock().unwrap().clone())
    }
    async fn update(&self, input: SdtmVersionUpdate) -> Result<SdtmVersion, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmVersionNotFound(input.id))?;
        if let Some(name) = input.name {
            v.name = name;
        }
        v.updated_at = epoch();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmVersionNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct InMemorySdtmDomainRepo {
    inner: Arc<Mutex<Vec<SdtmDomain>>>,
    next_id: AtomicI64,
}

impl InMemorySdtmDomainRepo {
    fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SdtmDomainRepository for InMemorySdtmDomainRepo {
    async fn create(&self, input: SdtmDomainNew) -> Result<SdtmDomain, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter()
            .any(|d| d.version_id == input.version_id && d.name == input.name)
        {
            return Err(DomainError::DuplicateSdtmDomain {
                version_id: input.version_id,
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let d = SdtmDomain::for_repository(
            id,
            input.version_id,
            input.name,
            input.category,
            input.descriptions,
            epoch(),
            epoch(),
        );
        g.push(d.clone());
        Ok(d)
    }
    async fn find_by_id(&self, id: i64) -> Result<SdtmDomain, DomainError> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(DomainError::SdtmDomainNotFound(id))
    }
    async fn list_by_version(&self, version_id: i64) -> Result<Vec<SdtmDomain>, DomainError> {
        let g = self.inner.lock().unwrap();
        Ok(g.iter()
            .filter(|d| d.version_id == version_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: SdtmDomainUpdate) -> Result<SdtmDomain, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let d = g
            .iter_mut()
            .find(|d| d.id == input.id)
            .ok_or(DomainError::SdtmDomainNotFound(input.id))?;
        if let Some(name) = input.name {
            d.name = name;
        }
        if let Some(category) = input.category {
            d.category = category;
        }
        if let Some(descriptions) = input.descriptions {
            d.descriptions = descriptions;
        }
        d.updated_at = epoch();
        Ok(d.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|d| d.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmDomainNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct InMemorySdtmVariableRepo {
    inner: Arc<Mutex<Vec<SdtmVariable>>>,
    next_id: AtomicI64,
}

impl InMemorySdtmVariableRepo {
    fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SdtmVariableRepository for InMemorySdtmVariableRepo {
    async fn create(&self, input: SdtmVariableNew) -> Result<SdtmVariable, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter()
            .any(|v| v.domain_id == input.domain_id && v.name == input.name)
        {
            return Err(DomainError::DuplicateSdtmVariable {
                domain_id: input.domain_id,
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let v = SdtmVariable::for_repository(
            id,
            input.domain_id,
            input.name,
            input.variable_controlled,
            input.variable_type,
            input.variable_core,
            input.variable_role,
            input.variable_sequence,
            input.descriptions,
            epoch(),
            epoch(),
        );
        g.push(v.clone());
        Ok(v)
    }
    async fn find_by_id(&self, id: i64) -> Result<SdtmVariable, DomainError> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or(DomainError::SdtmVariableNotFound(id))
    }
    async fn list_by_domain(&self, domain_id: i64) -> Result<Vec<SdtmVariable>, DomainError> {
        let g = self.inner.lock().unwrap();
        Ok(g.iter()
            .filter(|v| v.domain_id == domain_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: SdtmVariableUpdate) -> Result<SdtmVariable, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmVariableNotFound(input.id))?;
        if let Some(name) = input.name {
            v.name = name;
        }
        if let Some(controlled) = input.variable_controlled {
            v.variable_controlled = controlled;
        }
        if let Some(t) = input.variable_type {
            v.variable_type = t;
        }
        if let Some(c) = input.variable_core {
            v.variable_core = c;
        }
        if let Some(role) = input.variable_role {
            v.variable_role = role;
        }
        if let Some(seq) = input.variable_sequence {
            v.variable_sequence = seq;
        }
        if let Some(descriptions) = input.descriptions {
            v.descriptions = descriptions;
        }
        v.updated_at = epoch();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmVariableNotFound(id));
        }
        Ok(())
    }
}

// ---- service construction ----

fn make_service()
-> DomainModelServiceImpl<InMemorySdtmVersionRepo, InMemorySdtmDomainRepo, InMemorySdtmVariableRepo>
{
    DomainModelServiceImpl::from_repos(
        InMemorySdtmVersionRepo::new(),
        InMemorySdtmDomainRepo::new(),
        InMemorySdtmVariableRepo::new(),
    )
}

// ---- tests ----

#[tokio::test]
async fn version_crud_round_trips_through_api() {
    let svc = make_service();

    let created = svc
        .create_version(CreateSdtmVersionRequest {
            name: "2026-08-24".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(created.id, 1);
    assert_eq!(created.name, "2026-08-24");

    let updated = svc
        .update_version(UpdateSdtmVersionRequest {
            id: created.id,
            name: Some("2026-09-01".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "2026-09-01");

    let listed = svc.list_versions().await.unwrap();
    assert_eq!(listed.versions.len(), 1);
    assert_eq!(listed.versions[0].name, "2026-09-01");

    svc.delete_version(created.id).await.unwrap();
    assert_eq!(svc.list_versions().await.unwrap().versions.len(), 0);
}

#[tokio::test]
async fn duplicate_version_maps_to_api_error() {
    let svc = make_service();
    svc.create_version(CreateSdtmVersionRequest {
        name: "2026-08-24".to_string(),
    })
    .await
    .unwrap();
    let err = svc
        .create_version(CreateSdtmVersionRequest {
            name: "2026-08-24".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainModelApiError::DuplicateSdtmVersion { ref name } if name == "2026-08-24"
    ));
}

#[tokio::test]
async fn unknown_version_maps_to_api_error() {
    let svc = make_service();
    let err = svc.delete_version(42).await.unwrap_err();
    assert!(matches!(err, DomainModelApiError::SdtmVersionNotFound(42)));
}

#[tokio::test]
async fn empty_version_name_maps_to_validation_error() {
    let svc = make_service();
    let err = svc
        .create_version(CreateSdtmVersionRequest {
            name: "   ".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainModelApiError::Validation(_)));
}

#[tokio::test]
async fn domain_crud_round_trips_through_api_with_descriptions() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "2026-08-24".to_string(),
        })
        .await
        .unwrap();

    let descs = vec![
        api_domain_desc("en", "Adverse Events", "One record per event"),
        api_domain_desc("ja", "有害事象", "イベント毎に1レコード"),
    ];

    let created = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: descs.clone(),
        })
        .await
        .unwrap();
    assert_eq!(created.id, 1);
    assert_eq!(created.category, ApiCategory::Events);
    assert_eq!(created.descriptions.len(), 2);
    assert_eq!(created.descriptions[0].lang, "en");
    assert_eq!(
        created.descriptions[0].details.description,
        "Adverse Events"
    );
    assert_eq!(
        created.descriptions[0].details.structure,
        "One record per event"
    );
    assert_eq!(created.descriptions[1].lang, "ja");

    let fetched = svc.get_domain_by_id(created.id).await.unwrap();
    assert_eq!(fetched.descriptions.len(), 2);

    let listed = svc.list_domains_by_version(v.id).await.unwrap();
    assert_eq!(listed.domains.len(), 1);
    assert_eq!(listed.domains[0].name, "AE");

    // Replace descriptions with a single-language list.
    let new_descs = vec![api_domain_desc(
        "en",
        "Adverse Events v2",
        "Updated structure",
    )];
    let updated = svc
        .update_domain(UpdateSdtmDomainRequest {
            id: created.id,
            name: None,
            category: None,
            descriptions: Some(new_descs.clone()),
        })
        .await
        .unwrap();
    assert_eq!(updated.descriptions.len(), 1);
    assert_eq!(
        updated.descriptions[0].details.description,
        "Adverse Events v2"
    );

    // Empty list clears the column.
    let cleared = svc
        .update_domain(UpdateSdtmDomainRequest {
            id: created.id,
            name: None,
            category: None,
            descriptions: Some(vec![]),
        })
        .await
        .unwrap();
    assert!(cleared.descriptions.is_empty());

    svc.delete_domain(created.id).await.unwrap();
    assert_eq!(
        svc.list_domains_by_version(v.id)
            .await
            .unwrap()
            .domains
            .len(),
        0
    );
}

#[tokio::test]
async fn domain_category_round_trips() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    for cat in [
        ApiCategory::SpecialPurpose,
        ApiCategory::Interventions,
        ApiCategory::Events,
        ApiCategory::Findings,
        ApiCategory::TrialDesign,
        ApiCategory::Relationships,
        ApiCategory::StudyReference,
    ] {
        let d = svc
            .create_domain(CreateSdtmDomainRequest {
                version_id: v.id,
                name: format!("{cat:?}").to_string(),
                category: cat,
                descriptions: vec![],
            })
            .await
            .unwrap();
        assert_eq!(d.category, cat);
    }
}

#[tokio::test]
async fn duplicate_domain_maps_to_api_error() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    svc.create_domain(CreateSdtmDomainRequest {
        version_id: v.id,
        name: "AE".to_string(),
        category: ApiCategory::Events,
        descriptions: vec![],
    })
    .await
    .unwrap();
    let err = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainModelApiError::DuplicateSdtmDomain { version_id: 1, ref name } if name == "AE"
    ));
}

#[tokio::test]
async fn missing_parent_version_is_only_caught_by_persistence() {
    // The in-memory fakes intentionally don't enforce FK
    // constraints — they trust the caller to maintain referential
    // integrity. The Postgres adapter rejects this case with
    // `FkSdtmVersionNotFound`; that path is covered by the
    // `integration_persistence` test suite (gated on a live DB).
    let svc = make_service();
    let _ = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: 999,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn variable_crud_round_trips_through_api_with_descriptions() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    let d = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();

    let descs = vec![api_variable_desc("en", "Reported Term")];

    let created = svc
        .create_variable(CreateSdtmVariableRequest {
            domain_id: d.id,
            name: "AETERM".to_string(),
            variable_controlled: Some(ApiSdtmVariableControlled {
                code: Some(vec!["AETERM".to_string()]),
                format: None,
            }),
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: Some(ApiSdtmRole::Topic),
            variable_sequence: 1,
            descriptions: descs.clone(),
        })
        .await
        .unwrap();
    assert_eq!(created.id, 1);
    assert_eq!(created.variable_type, ApiSdtmVariableType::Character);
    assert_eq!(created.variable_core, ApiSdtmVariableCore::Req);
    assert_eq!(created.variable_role, Some(ApiSdtmRole::Topic));
    assert_eq!(
        created.variable_controlled,
        Some(ApiSdtmVariableControlled {
            code: Some(vec!["AETERM".to_string()]),
            format: None,
        })
    );
    assert_eq!(created.descriptions.len(), 1);

    let fetched = svc.get_variable_by_id(created.id).await.unwrap();
    assert_eq!(fetched.descriptions[0].details.label, "Reported Term");

    let listed = svc.list_variables_by_domain(d.id).await.unwrap();
    assert_eq!(listed.variables.len(), 1);
    assert_eq!(listed.variables[0].name, "AETERM");

    svc.delete_variable(created.id).await.unwrap();
    assert_eq!(
        svc.list_variables_by_domain(d.id)
            .await
            .unwrap()
            .variables
            .len(),
        0
    );
}

#[tokio::test]
async fn variable_role_enum_round_trips() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    let d = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();

    for (i, role) in [
        Some(ApiSdtmRole::Identifier),
        Some(ApiSdtmRole::Topic),
        Some(ApiSdtmRole::Timing),
        Some(ApiSdtmRole::RecordQualifier),
        Some(ApiSdtmRole::SynonymQualifier),
        Some(ApiSdtmRole::VariableQualifier),
        Some(ApiSdtmRole::GroupingQualifier),
        Some(ApiSdtmRole::Rule),
    ]
    .into_iter()
    .enumerate()
    {
        let var = svc
            .create_variable(CreateSdtmVariableRequest {
                domain_id: d.id,
                name: format!("VAR{i}").to_string(),
                variable_controlled: None,
                variable_type: ApiSdtmVariableType::Character,
                variable_core: ApiSdtmVariableCore::Exp,
                variable_role: role,
                variable_sequence: i as i64 + 1,
                descriptions: vec![],
            })
            .await
            .unwrap();
        assert_eq!(var.variable_role, role);
    }
}

#[tokio::test]
async fn variable_three_state_outer_none_leaves_field_unchanged() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    let d = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();
    let var = svc
        .create_variable(CreateSdtmVariableRequest {
            domain_id: d.id,
            name: "AETERM".to_string(),
            variable_controlled: Some(ApiSdtmVariableControlled {
                code: Some(vec!["AETERM".to_string()]),
                format: None,
            }),
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: Some(ApiSdtmRole::Topic),
            variable_sequence: 1,
            descriptions: vec![],
        })
        .await
        .unwrap();

    let updated = svc
        .update_variable(UpdateSdtmVariableRequest {
            id: var.id,
            name: None,
            variable_controlled: None, // don't change
            variable_type: None,
            variable_core: None,
            variable_role: None, // don't change
            variable_sequence: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(
        updated.variable_controlled,
        Some(ApiSdtmVariableControlled {
            code: Some(vec!["AETERM".to_string()]),
            format: None,
        })
    );
    assert_eq!(updated.variable_role, Some(ApiSdtmRole::Topic));
}

#[tokio::test]
async fn variable_three_state_some_some_replaces_field() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    let d = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();
    let var = svc
        .create_variable(CreateSdtmVariableRequest {
            domain_id: d.id,
            name: "AETERM".to_string(),
            variable_controlled: Some(ApiSdtmVariableControlled {
                code: Some(vec!["AETERM".to_string()]),
                format: None,
            }),
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: Some(ApiSdtmRole::Topic),
            variable_sequence: 1,
            descriptions: vec![],
        })
        .await
        .unwrap();

    let updated = svc
        .update_variable(UpdateSdtmVariableRequest {
            id: var.id,
            name: None,
            variable_controlled: Some(Some(ApiSdtmVariableControlled {
                code: Some(vec!["AETERMCD".to_string()]),
                format: None,
            })),
            variable_type: None,
            variable_core: None,
            variable_role: Some(Some(ApiSdtmRole::Identifier)),
            variable_sequence: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(
        updated.variable_controlled,
        Some(ApiSdtmVariableControlled {
            code: Some(vec!["AETERMCD".to_string()]),
            format: None,
        })
    );
    assert_eq!(updated.variable_role, Some(ApiSdtmRole::Identifier));
}

#[tokio::test]
async fn variable_three_state_some_none_clears_field() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    let d = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();
    let var = svc
        .create_variable(CreateSdtmVariableRequest {
            domain_id: d.id,
            name: "AETERM".to_string(),
            variable_controlled: Some(ApiSdtmVariableControlled {
                code: Some(vec!["AETERM".to_string()]),
                format: None,
            }),
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: Some(ApiSdtmRole::Topic),
            variable_sequence: 1,
            descriptions: vec![],
        })
        .await
        .unwrap();

    let updated = svc
        .update_variable(UpdateSdtmVariableRequest {
            id: var.id,
            name: None,
            variable_controlled: Some(None), // clear
            variable_type: None,
            variable_core: None,
            variable_role: Some(None), // clear
            variable_sequence: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.variable_controlled, None);
    assert_eq!(updated.variable_role, None);
}

#[tokio::test]
async fn duplicate_variable_maps_to_api_error() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    let d = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();
    svc.create_variable(CreateSdtmVariableRequest {
        domain_id: d.id,
        name: "AETERM".to_string(),
        variable_controlled: None,
        variable_type: ApiSdtmVariableType::Character,
        variable_core: ApiSdtmVariableCore::Req,
        variable_role: None,
        variable_sequence: 1,
        descriptions: vec![],
    })
    .await
    .unwrap();
    let err = svc
        .create_variable(CreateSdtmVariableRequest {
            domain_id: d.id,
            name: "AETERM".to_string(),
            variable_controlled: None,
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: None,
            variable_sequence: 2,
            descriptions: vec![],
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainModelApiError::DuplicateSdtmVariable { domain_id: 1, ref name } if name == "AETERM"
    ));
}

#[tokio::test]
async fn missing_parent_domain_is_only_caught_by_persistence() {
    // In-memory fakes trust the caller; FK enforcement is the
    // Postgres adapter's responsibility (see
    // `integration_persistence`).
    let svc = make_service();
    let _ = svc
        .create_variable(CreateSdtmVariableRequest {
            domain_id: 999,
            name: "AETERM".to_string(),
            variable_controlled: None,
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: None,
            variable_sequence: 1,
            descriptions: vec![],
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn empty_variable_name_maps_to_validation_error() {
    let svc = make_service();
    let v = svc
        .create_version(CreateSdtmVersionRequest {
            name: "v1".to_string(),
        })
        .await
        .unwrap();
    let d = svc
        .create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".to_string(),
            category: ApiCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();
    let err = svc
        .create_variable(CreateSdtmVariableRequest {
            domain_id: d.id,
            name: " ".to_string(),
            variable_controlled: None,
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: None,
            variable_sequence: 1,
            descriptions: vec![],
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainModelApiError::Validation(_)));
}
