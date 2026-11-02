//! Unit tests for `SdtmProjectServiceImpl`.
//!
//! Wires the adapter on top of in-memory repositories so the behaviour
//! is exercised without touching PostgreSQL. The in-memory
//! repositories are private to this test module — they are
//! intentionally not shared with the usecase tests so each layer can
//! evolve independently.
//!
//! One store backs all five repositories (a single struct implements
//! all five ports) so the domain and variable repositories can hydrate
//! their override the way the postgres adapters' LEFT JOIN does.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use apis::sdtm_project::{
    CreateSdtmProjectDomainOverrideRequest, CreateSdtmProjectDomainRequest,
    CreateSdtmProjectVariableOverrideRequest, CreateSdtmProjectVariableRequest,
    CreateSdtmProjectVersionRequest, DomainCategory as ApiCategory, SdtmProjectApiError,
    SdtmProjectService, SdtmRole as ApiSdtmRole, SdtmVariableCore as ApiSdtmVariableCore,
    SdtmVariableType as ApiSdtmVariableType, UpdateSdtmProjectDomainOverrideRequest,
    UpdateSdtmProjectDomainRequest, UpdateSdtmProjectVariableOverrideRequest,
    UpdateSdtmProjectVariableRequest, UpdateSdtmProjectVersionRequest,
};

use crate::domain::{
    DomainError, ProjectLookup, SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainOverride,
    SdtmProjectDomainOverrideNew, SdtmProjectDomainOverrideRepository,
    SdtmProjectDomainOverrideUpdate, SdtmProjectDomainRepository, SdtmProjectDomainUpdate,
    SdtmProjectVariable, SdtmProjectVariableNew, SdtmProjectVariableOverride,
    SdtmProjectVariableOverrideNew, SdtmProjectVariableOverrideRepository,
    SdtmProjectVariableOverrideUpdate, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionRepository,
    SdtmProjectVersionUpdate,
};

use super::SdtmProjectServiceImpl;

// ---- helpers ----

fn epoch() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

// ---- in-memory store + the single repo implementing all five ports ----

#[derive(Default)]
struct InMemoryStore {
    versions: Mutex<Vec<SdtmProjectVersion>>,
    domains: Mutex<Vec<SdtmProjectDomain>>,
    domain_overrides: Mutex<Vec<SdtmProjectDomainOverride>>,
    variables: Mutex<Vec<SdtmProjectVariable>>,
    variable_overrides: Mutex<Vec<SdtmProjectVariableOverride>>,
    next_id: AtomicI64,
}

impl InMemoryStore {
    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst) + 1
    }
}

#[derive(Clone)]
struct InMemoryRepo {
    store: Arc<InMemoryStore>,
}

impl InMemoryRepo {
    fn new(store: Arc<InMemoryStore>) -> Self {
        Self { store }
    }

    fn hydrate_domain(&self, d: SdtmProjectDomain) -> SdtmProjectDomain {
        let ov = self
            .store
            .domain_overrides
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.domain_id == d.id)
            .cloned();
        SdtmProjectDomain {
            domain_override: ov,
            ..d
        }
    }

    fn hydrate_variable(&self, v: SdtmProjectVariable) -> SdtmProjectVariable {
        let ov = self
            .store
            .variable_overrides
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.variable_id == v.id)
            .cloned();
        SdtmProjectVariable {
            variable_override: ov,
            ..v
        }
    }
}

#[async_trait]
impl SdtmProjectVersionRepository for InMemoryRepo {
    async fn create(
        &self,
        input: SdtmProjectVersionNew,
    ) -> Result<SdtmProjectVersion, DomainError> {
        let mut g = self.store.versions.lock().unwrap();
        if g.iter()
            .any(|v| v.project_code == input.project_code && v.version_name == input.version_name)
        {
            return Err(DomainError::DuplicateSdtmProjectVersion {
                project_code: input.project_code,
                version_name: input.version_name,
            });
        }
        let now = epoch();
        let v = SdtmProjectVersion::for_repository(
            self.store.next_id(),
            input.project_code,
            input.version_name,
            now,
            now,
        );
        g.push(v.clone());
        Ok(v)
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVersion, DomainError> {
        self.store
            .versions
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or(DomainError::SdtmProjectVersionNotFound(id))
    }

    async fn list_by_project(
        &self,
        project_code: &str,
    ) -> Result<Vec<SdtmProjectVersion>, DomainError> {
        let g = self.store.versions.lock().unwrap();
        Ok(g.iter()
            .filter(|v| v.project_code == project_code)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        input: SdtmProjectVersionUpdate,
    ) -> Result<SdtmProjectVersion, DomainError> {
        let mut g = self.store.versions.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmProjectVersionNotFound(input.id))?;
        if let Some(name) = input.version_name {
            v.version_name = name;
        }
        v.updated_at = epoch();
        Ok(v.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.store.versions.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectVersionNotFound(id));
        }
        Ok(())
    }
}

#[async_trait]
impl SdtmProjectDomainRepository for InMemoryRepo {
    async fn create(&self, input: SdtmProjectDomainNew) -> Result<SdtmProjectDomain, DomainError> {
        let mut g = self.store.domains.lock().unwrap();
        if g.iter()
            .any(|d| d.version_id == input.version_id && d.domain_code == input.domain_code)
        {
            return Err(DomainError::DuplicateSdtmProjectDomain {
                version_id: input.version_id,
                domain_code: input.domain_code,
            });
        }
        let now = epoch();
        let d = SdtmProjectDomain::for_repository(
            self.store.next_id(),
            input.version_id,
            input.domain_code,
            input.key_variables,
            None,
            now,
            now,
        );
        g.push(d.clone());
        Ok(d)
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectDomain, DomainError> {
        let d = self
            .store
            .domains
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(DomainError::SdtmProjectDomainNotFound(id))?;
        Ok(self.hydrate_domain(d))
    }

    async fn list_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<SdtmProjectDomain>, DomainError> {
        let g = self.store.domains.lock().unwrap();
        let rows: Vec<SdtmProjectDomain> = g
            .iter()
            .filter(|d| d.version_id == version_id)
            .cloned()
            .collect();
        Ok(rows.into_iter().map(|d| self.hydrate_domain(d)).collect())
    }

    async fn update(
        &self,
        input: SdtmProjectDomainUpdate,
    ) -> Result<SdtmProjectDomain, DomainError> {
        let mut g = self.store.domains.lock().unwrap();
        let d = g
            .iter_mut()
            .find(|d| d.id == input.id)
            .ok_or(DomainError::SdtmProjectDomainNotFound(input.id))?;
        if let Some(code) = input.domain_code {
            d.domain_code = code;
        }
        if let Some(kv) = input.key_variables {
            d.key_variables = kv;
        }
        d.updated_at = epoch();
        Ok(self.hydrate_domain(d.clone()))
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.store.domains.lock().unwrap();
        let before = g.len();
        g.retain(|d| d.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectDomainNotFound(id));
        }
        Ok(())
    }
}

#[async_trait]
impl SdtmProjectDomainOverrideRepository for InMemoryRepo {
    async fn create(
        &self,
        input: SdtmProjectDomainOverrideNew,
    ) -> Result<SdtmProjectDomainOverride, DomainError> {
        let mut g = self.store.domain_overrides.lock().unwrap();
        if g.iter().any(|o| o.domain_id == input.domain_id) {
            return Err(DomainError::DuplicateSdtmProjectDomainOverride {
                domain_id: input.domain_id,
            });
        }
        let now = epoch();
        let o = SdtmProjectDomainOverride::for_repository(
            self.store.next_id(),
            input.domain_id,
            input.category,
            input.descriptions,
            now,
            now,
        );
        g.push(o.clone());
        Ok(o)
    }

    async fn find_by_domain_id(
        &self,
        domain_id: i64,
    ) -> Result<SdtmProjectDomainOverride, DomainError> {
        self.store
            .domain_overrides
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.domain_id == domain_id)
            .cloned()
            .ok_or(DomainError::SdtmProjectDomainOverrideNotFound(domain_id))
    }

    async fn update(
        &self,
        input: SdtmProjectDomainOverrideUpdate,
    ) -> Result<SdtmProjectDomainOverride, DomainError> {
        let mut g = self.store.domain_overrides.lock().unwrap();
        let o = g
            .iter_mut()
            .find(|o| o.domain_id == input.domain_id)
            .ok_or(DomainError::SdtmProjectDomainOverrideNotFound(
                input.domain_id,
            ))?;
        if let Some(category) = input.category {
            o.category = category;
        }
        if let Some(descriptions) = input.descriptions {
            o.descriptions = descriptions;
        }
        o.updated_at = epoch();
        Ok(o.clone())
    }

    async fn delete(&self, domain_id: i64) -> Result<(), DomainError> {
        let mut g = self.store.domain_overrides.lock().unwrap();
        let before = g.len();
        g.retain(|o| o.domain_id != domain_id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectDomainOverrideNotFound(domain_id));
        }
        Ok(())
    }
}

#[async_trait]
impl SdtmProjectVariableRepository for InMemoryRepo {
    async fn create(
        &self,
        input: SdtmProjectVariableNew,
    ) -> Result<SdtmProjectVariable, DomainError> {
        let mut g = self.store.variables.lock().unwrap();
        if g.iter()
            .any(|v| v.domain_id == input.domain_id && v.variable_code == input.variable_code)
        {
            return Err(DomainError::DuplicateSdtmProjectVariable {
                domain_id: input.domain_id,
                variable_code: input.variable_code,
            });
        }
        let now = epoch();
        let v = SdtmProjectVariable::for_repository(
            self.store.next_id(),
            input.domain_id,
            input.variable_code,
            input.variable_sequence,
            None,
            now,
            now,
        );
        g.push(v.clone());
        Ok(v)
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVariable, DomainError> {
        let v = self
            .store
            .variables
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or(DomainError::SdtmProjectVariableNotFound(id))?;
        Ok(self.hydrate_variable(v))
    }

    async fn list_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmProjectVariable>, DomainError> {
        let g = self.store.variables.lock().unwrap();
        let rows: Vec<SdtmProjectVariable> = g
            .iter()
            .filter(|v| v.domain_id == domain_id)
            .cloned()
            .collect();
        Ok(rows.into_iter().map(|v| self.hydrate_variable(v)).collect())
    }

    async fn update(
        &self,
        input: SdtmProjectVariableUpdate,
    ) -> Result<SdtmProjectVariable, DomainError> {
        let mut g = self.store.variables.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmProjectVariableNotFound(input.id))?;
        if let Some(code) = input.variable_code {
            v.variable_code = code;
        }
        if let Some(seq) = input.variable_sequence {
            v.variable_sequence = seq;
        }
        v.updated_at = epoch();
        Ok(self.hydrate_variable(v.clone()))
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.store.variables.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectVariableNotFound(id));
        }
        Ok(())
    }
}

#[async_trait]
impl SdtmProjectVariableOverrideRepository for InMemoryRepo {
    async fn create(
        &self,
        input: SdtmProjectVariableOverrideNew,
    ) -> Result<SdtmProjectVariableOverride, DomainError> {
        let mut g = self.store.variable_overrides.lock().unwrap();
        if g.iter().any(|o| o.variable_id == input.variable_id) {
            return Err(DomainError::DuplicateSdtmProjectVariableOverride {
                variable_id: input.variable_id,
            });
        }
        let now = epoch();
        let o = SdtmProjectVariableOverride::for_repository(
            self.store.next_id(),
            input.variable_id,
            input.variable_type,
            input.variable_core,
            input.variable_role,
            input.descriptions,
            now,
            now,
        );
        g.push(o.clone());
        Ok(o)
    }

    async fn find_by_variable_id(
        &self,
        variable_id: i64,
    ) -> Result<SdtmProjectVariableOverride, DomainError> {
        self.store
            .variable_overrides
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.variable_id == variable_id)
            .cloned()
            .ok_or(DomainError::SdtmProjectVariableOverrideNotFound(
                variable_id,
            ))
    }

    async fn update(
        &self,
        input: SdtmProjectVariableOverrideUpdate,
    ) -> Result<SdtmProjectVariableOverride, DomainError> {
        let mut g = self.store.variable_overrides.lock().unwrap();
        let o = g
            .iter_mut()
            .find(|o| o.variable_id == input.variable_id)
            .ok_or(DomainError::SdtmProjectVariableOverrideNotFound(
                input.variable_id,
            ))?;
        if let Some(t) = input.variable_type {
            o.variable_type = t;
        }
        if let Some(c) = input.variable_core {
            o.variable_core = c;
        }
        if let Some(role) = input.variable_role {
            o.variable_role = role;
        }
        if let Some(descriptions) = input.descriptions {
            o.descriptions = descriptions;
        }
        o.updated_at = epoch();
        Ok(o.clone())
    }

    async fn delete(&self, variable_id: i64) -> Result<(), DomainError> {
        let mut g = self.store.variable_overrides.lock().unwrap();
        let before = g.len();
        g.retain(|o| o.variable_id != variable_id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectVariableOverrideNotFound(
                variable_id,
            ));
        }
        Ok(())
    }
}

// ---- project lookup ----

#[derive(Default)]
struct FakeProjectLookup {
    known: Mutex<Vec<String>>,
}

impl FakeProjectLookup {
    fn new(known: &[&str]) -> Self {
        Self {
            known: Mutex::new(known.iter().map(|s| s.to_string()).collect()),
        }
    }
}

#[async_trait]
impl ProjectLookup for FakeProjectLookup {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if self.known.lock().unwrap().iter().any(|c| c == code) {
            Ok(())
        } else {
            Err(DomainError::ProjectNotFound(code.to_string()))
        }
    }
}

// ---- service construction ----

type TestService = SdtmProjectServiceImpl<
    InMemoryRepo,
    InMemoryRepo,
    InMemoryRepo,
    InMemoryRepo,
    InMemoryRepo,
    FakeProjectLookup,
>;

fn make_service() -> TestService {
    let store = Arc::new(InMemoryStore::default());
    let repo = InMemoryRepo::new(store);
    SdtmProjectServiceImpl::from_repos(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        Arc::new(FakeProjectLookup::new(&["P1"])),
    )
}

async fn seed_version(svc: &TestService) -> i64 {
    svc.create_version(CreateSdtmProjectVersionRequest {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    })
    .await
    .unwrap()
    .id
}

// ---- version ----

#[tokio::test]
async fn version_crud_round_trips_through_api() {
    let svc = make_service();
    let created = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "P1".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap();
    assert_eq!(created.project_code, "P1");

    let listed = svc.list_versions_by_project("P1").await.unwrap();
    assert_eq!(listed.versions.len(), 1);

    let updated = svc
        .update_version(UpdateSdtmProjectVersionRequest {
            id: created.id,
            version_name: Some("2025-01-15".into()),
        })
        .await
        .unwrap();
    assert_eq!(updated.version_name, "2025-01-15");

    svc.delete_version(created.id).await.unwrap();
    assert!(
        svc.list_versions_by_project("P1")
            .await
            .unwrap()
            .versions
            .is_empty()
    );
}

#[tokio::test]
async fn duplicate_version_maps_to_api_error() {
    let svc = make_service();
    let _ = seed_version(&svc).await;
    let err = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "P1".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SdtmProjectApiError::DuplicateSdtmProjectVersion { .. }
    ));
}

#[tokio::test]
async fn unknown_version_maps_to_api_error() {
    let svc = make_service();
    let err = svc.get_version_by_id(999).await.unwrap_err();
    assert!(matches!(
        err,
        SdtmProjectApiError::SdtmProjectVersionNotFound(999)
    ));
}

#[tokio::test]
async fn missing_project_maps_to_api_error() {
    let svc = make_service();
    let err = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "NOPE".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SdtmProjectApiError::ProjectNotFound(ref code) if code == "NOPE"
    ));
}

#[tokio::test]
async fn validation_error_maps_to_validation_api_error() {
    let svc = make_service();
    let err = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "P1".into(),
            version_name: "   ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, SdtmProjectApiError::Validation(_)));
}

// ---- domain ----

#[tokio::test]
async fn domain_crud_round_trips_through_api() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let created = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: vec!["AETERM".into()],
        })
        .await
        .unwrap();
    assert_eq!(created.domain_code, "AE");

    let by_id = svc.get_domain_by_id(created.id).await.unwrap();
    assert_eq!(by_id.id, created.id);

    let listed = svc.list_domains_by_version(version_id).await.unwrap();
    assert_eq!(listed.domains.len(), 1);

    let updated = svc
        .update_domain(UpdateSdtmProjectDomainRequest {
            id: created.id,
            domain_code: Some("AE2".into()),
            key_variables: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.domain_code, "AE2");

    svc.delete_domain(created.id).await.unwrap();
    assert!(
        svc.list_domains_by_version(version_id)
            .await
            .unwrap()
            .domains
            .is_empty()
    );
}

#[tokio::test]
async fn domain_category_round_trips() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();
    let ov = svc
        .create_domain_override(CreateSdtmProjectDomainOverrideRequest {
            domain_id: domain.id,
            category: ApiCategory::TrialDesign,
            descriptions: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(ov.category, ApiCategory::TrialDesign);

    let got = svc
        .get_domain_override(domain.id)
        .await
        .unwrap()
        .expect("the override exists");
    assert_eq!(got.category, ApiCategory::TrialDesign);
}

#[tokio::test]
async fn domain_override_round_trips_through_api() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();

    let created = svc
        .create_domain_override(CreateSdtmProjectDomainOverrideRequest {
            domain_id: domain.id,
            category: ApiCategory::Events,
            descriptions: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(created.domain_id, domain.id);

    let updated = svc
        .update_domain_override(UpdateSdtmProjectDomainOverrideRequest {
            domain_id: domain.id,
            category: Some(ApiCategory::Findings),
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.category, ApiCategory::Findings);

    svc.delete_domain_override(domain.id).await.unwrap();
    // The override is optional: once it is gone, fetching it yields
    // `None` rather than an error.
    assert!(svc.get_domain_override(domain.id).await.unwrap().is_none());
}

/// A domain that has never been overridden is the common case, and
/// reading its override is `None` — not a not-found error.
#[tokio::test]
async fn a_domain_without_an_override_reads_as_none() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();

    assert!(svc.get_domain_override(domain.id).await.unwrap().is_none());
}

/// The variable-side twin of `a_domain_without_an_override_reads_as_none`.
#[tokio::test]
async fn a_variable_without_an_override_reads_as_none() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();
    let variable = svc
        .create_variable(CreateSdtmProjectVariableRequest {
            domain_id: domain.id,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
        })
        .await
        .unwrap();

    assert!(
        svc.get_variable_override(variable.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn key_variables_order_survives_the_api() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let created = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: vec!["AESEV".into(), "AETERM".into()],
        })
        .await
        .unwrap();
    // The caller's order is authoritative and is not re-sorted.
    assert_eq!(
        created.key_variables,
        vec!["AESEV".to_string(), "AETERM".to_string()]
    );

    let by_id = svc.get_domain_by_id(created.id).await.unwrap();
    assert_eq!(
        by_id.key_variables,
        vec!["AESEV".to_string(), "AETERM".to_string()]
    );
}

// ---- variable ----

#[tokio::test]
async fn variable_crud_round_trips_through_api() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();

    let created = svc
        .create_variable(CreateSdtmProjectVariableRequest {
            domain_id: domain.id,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
        })
        .await
        .unwrap();
    assert_eq!(created.variable_code, "AETERM");
    assert_eq!(created.variable_sequence, 11);

    let listed = svc.list_variables_by_domain(domain.id).await.unwrap();
    assert_eq!(listed.variables.len(), 1);

    let updated = svc
        .update_variable(UpdateSdtmProjectVariableRequest {
            id: created.id,
            variable_code: None,
            variable_sequence: Some(12),
        })
        .await
        .unwrap();
    assert_eq!(updated.variable_sequence, 12);

    svc.delete_variable(created.id).await.unwrap();
    assert!(
        svc.list_variables_by_domain(domain.id)
            .await
            .unwrap()
            .variables
            .is_empty()
    );
}

#[tokio::test]
async fn variable_role_enum_round_trips() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();
    let variable = svc
        .create_variable(CreateSdtmProjectVariableRequest {
            domain_id: domain.id,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
        })
        .await
        .unwrap();

    let ov = svc
        .create_variable_override(CreateSdtmProjectVariableOverrideRequest {
            variable_id: variable.id,
            variable_type: ApiSdtmVariableType::Character,
            variable_core: ApiSdtmVariableCore::Req,
            variable_role: Some(ApiSdtmRole::RecordQualifier),
            descriptions: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(ov.variable_role, Some(ApiSdtmRole::RecordQualifier));
    assert_eq!(ov.variable_type, ApiSdtmVariableType::Character);
    assert_eq!(ov.variable_core, ApiSdtmVariableCore::Req);

    let got = svc
        .get_variable_override(variable.id)
        .await
        .unwrap()
        .expect("the override exists");
    assert_eq!(got.variable_role, Some(ApiSdtmRole::RecordQualifier));
}

#[tokio::test]
async fn variable_role_three_state_clears_field() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            version_id,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();
    let variable = svc
        .create_variable(CreateSdtmProjectVariableRequest {
            domain_id: domain.id,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
        })
        .await
        .unwrap();
    svc.create_variable_override(CreateSdtmProjectVariableOverrideRequest {
        variable_id: variable.id,
        variable_type: ApiSdtmVariableType::Character,
        variable_core: ApiSdtmVariableCore::Req,
        variable_role: Some(ApiSdtmRole::Topic),
        descriptions: Vec::new(),
    })
    .await
    .unwrap();

    // Outer Some(None) clears the role.
    let cleared = svc
        .update_variable_override(UpdateSdtmProjectVariableOverrideRequest {
            variable_id: variable.id,
            variable_type: None,
            variable_core: None,
            variable_role: Some(None),
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(cleared.variable_role, None);

    // Outer None leaves the field unchanged.
    let unchanged = svc
        .update_variable_override(UpdateSdtmProjectVariableOverrideRequest {
            variable_id: variable.id,
            variable_type: None,
            variable_core: None,
            variable_role: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(unchanged.variable_role, None);
}
