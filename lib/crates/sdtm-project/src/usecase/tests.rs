use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::{
    DomainCategory, DomainError, ProjectLookup, SdtmDomainDescription, SdtmProjectDomain,
    SdtmProjectDomainNew, SdtmProjectDomainOverride, SdtmProjectDomainOverrideNew,
    SdtmProjectDomainOverrideRepository, SdtmProjectDomainOverrideUpdate,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectVariable,
    SdtmProjectVariableNew, SdtmProjectVariableOverride, SdtmProjectVariableOverrideNew,
    SdtmProjectVariableOverrideRepository, SdtmProjectVariableOverrideUpdate,
    SdtmProjectVariableRepository, SdtmProjectVariableUpdate, SdtmProjectVersion,
    SdtmProjectVersionNew, SdtmProjectVersionRepository, SdtmProjectVersionUpdate, SdtmRole,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableType,
};
use crate::usecase::commands::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion, UpdateSdtmProjectDomain,
    UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable, UpdateSdtmProjectVariableOverride,
    UpdateSdtmProjectVersion,
};
use crate::usecase::error::UsecaseError;
use crate::usecase::sdtm_project_usecase::{SdtmProjectUsecase, SdtmProjectUsecaseConfig};

// ---- shared in-memory store ---------------------------------------------
//
// One store behind all five repositories, so the domain and variable
// repositories can hydrate their override exactly the way the SQL
// adapters do with a LEFT JOIN. The stored parent rows always carry
// `domain_override: None`; the override lives in its own collection.

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
struct InMemoryVersionRepo {
    store: Arc<InMemoryStore>,
}

#[async_trait]
impl SdtmProjectVersionRepository for InMemoryVersionRepo {
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
        let now = chrono::Utc::now();
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
        v.updated_at = chrono::Utc::now();
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

#[derive(Clone)]
struct InMemoryDomainRepo {
    store: Arc<InMemoryStore>,
}

impl InMemoryDomainRepo {
    /// Hydrate the override the way the postgres adapter's LEFT JOIN does.
    fn with_override(&self, d: SdtmProjectDomain) -> SdtmProjectDomain {
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
}

#[async_trait]
impl SdtmProjectDomainRepository for InMemoryDomainRepo {
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
        let now = chrono::Utc::now();
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
        Ok(self.with_override(d))
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
        Ok(rows.into_iter().map(|d| self.with_override(d)).collect())
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
        d.updated_at = chrono::Utc::now();
        Ok(self.with_override(d.clone()))
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

#[derive(Clone)]
struct InMemoryDomainOverrideRepo {
    store: Arc<InMemoryStore>,
}

#[async_trait]
impl SdtmProjectDomainOverrideRepository for InMemoryDomainOverrideRepo {
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
        let now = chrono::Utc::now();
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
        o.updated_at = chrono::Utc::now();
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

#[derive(Clone)]
struct InMemoryVariableRepo {
    store: Arc<InMemoryStore>,
}

impl InMemoryVariableRepo {
    fn with_override(&self, v: SdtmProjectVariable) -> SdtmProjectVariable {
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
impl SdtmProjectVariableRepository for InMemoryVariableRepo {
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
        let now = chrono::Utc::now();
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
        Ok(self.with_override(v))
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
        Ok(rows.into_iter().map(|v| self.with_override(v)).collect())
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
        v.updated_at = chrono::Utc::now();
        Ok(self.with_override(v.clone()))
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

#[derive(Clone)]
struct InMemoryVariableOverrideRepo {
    store: Arc<InMemoryStore>,
}

#[async_trait]
impl SdtmProjectVariableOverrideRepository for InMemoryVariableOverrideRepo {
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
        let now = chrono::Utc::now();
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
        o.updated_at = chrono::Utc::now();
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

type TestUsecase = SdtmProjectUsecase<
    InMemoryVersionRepo,
    InMemoryDomainRepo,
    InMemoryDomainOverrideRepo,
    InMemoryVariableRepo,
    InMemoryVariableOverrideRepo,
    FakeProjectLookup,
>;

fn build_usecase() -> TestUsecase {
    let store = Arc::new(InMemoryStore::default());
    SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
        version_repo: InMemoryVersionRepo {
            store: store.clone(),
        },
        domain_repo: InMemoryDomainRepo {
            store: store.clone(),
        },
        domain_override_repo: InMemoryDomainOverrideRepo {
            store: store.clone(),
        },
        variable_repo: InMemoryVariableRepo {
            store: store.clone(),
        },
        variable_override_repo: InMemoryVariableOverrideRepo {
            store: store.clone(),
        },
        projects: Arc::new(FakeProjectLookup::new(&["P1"])),
    })
}

async fn seed_version(uc: &TestUsecase) -> i64 {
    uc.create_version(CreateSdtmProjectVersion {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    })
    .await
    .unwrap()
    .id
}

async fn seed_domain(uc: &TestUsecase, version_id: i64) -> i64 {
    uc.create_domain(CreateSdtmProjectDomain {
        version_id,
        domain_code: "AE".into(),
        key_variables: vec!["AETERM".into()],
    })
    .await
    .unwrap()
    .id
}

fn domain_description() -> SdtmDomainDescription {
    SdtmDomainDescription {
        lang: "en".into(),
        details: crate::domain::SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure: "One record per AE".into(),
        },
    }
}

fn variable_description() -> SdtmVariableDescription {
    SdtmVariableDescription {
        lang: "en".into(),
        details: crate::domain::SdtmVariableDescriptionDetail {
            label: "Adverse Event Term".into(),
        },
    }
}

// ---- version tests -------------------------------------------------------

#[tokio::test]
async fn version_crud_round_trips() {
    let uc = build_usecase();
    let v = uc
        .create_version(CreateSdtmProjectVersion {
            project_code: "P1".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap();
    assert_eq!(v.project_code, "P1");
    assert_eq!(v.version_name, "2024-09-27");

    let listed = uc.list_versions_by_project("P1").await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, v.id);

    let updated = uc
        .update_version(UpdateSdtmProjectVersion {
            id: v.id,
            version_name: Some("2025-01-15".into()),
        })
        .await
        .unwrap();
    assert_eq!(updated.version_name, "2025-01-15");

    uc.delete_version(v.id).await.unwrap();
    assert!(uc.list_versions_by_project("P1").await.unwrap().is_empty());
}

#[tokio::test]
async fn version_create_rejects_unknown_project() {
    let uc = build_usecase();
    let err = uc
        .create_version(CreateSdtmProjectVersion {
            project_code: "NOPE".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::ProjectNotFound(ref code)) if code == "NOPE"
    ));
}

#[tokio::test]
async fn version_create_rejects_blank_version_name() {
    let uc = build_usecase();
    let err = uc
        .create_version(CreateSdtmProjectVersion {
            project_code: "P1".into(),
            version_name: "   ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyVersionName)
    ));
}

// ---- domain tests --------------------------------------------------------

#[tokio::test]
async fn domain_crud_round_trips() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let d = uc
        .create_domain(CreateSdtmProjectDomain {
            version_id,
            domain_code: "AE".into(),
            key_variables: vec!["AETERM".into()],
        })
        .await
        .unwrap();
    assert_eq!(d.domain_code, "AE");

    let by_id = uc.get_domain_by_id(d.id).await.unwrap();
    assert_eq!(by_id.id, d.id);

    let list = uc.list_domains_by_version(version_id).await.unwrap();
    assert_eq!(list.len(), 1);

    let updated = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: d.id,
            domain_code: Some("AE2".into()),
            key_variables: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.domain_code, "AE2");

    uc.delete_domain(d.id).await.unwrap();
    assert!(
        uc.list_domains_by_version(version_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn domain_create_rejects_missing_version() {
    let uc = build_usecase();
    let err = uc
        .create_domain(CreateSdtmProjectDomain {
            version_id: 999,
            domain_code: "AE".into(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::FkSdtmProjectVersionNotFound(999))
    ));
}

#[tokio::test]
async fn domain_list_carries_the_override() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    uc.create_domain_override(CreateSdtmProjectDomainOverride {
        domain_id,
        category: DomainCategory::Events,
        descriptions: vec![domain_description()],
    })
    .await
    .unwrap();

    let list = uc.list_domains_by_version(version_id).await.unwrap();
    assert_eq!(list.len(), 1);
    let ov = list[0].domain_override.as_ref().expect("override hydrated");
    assert_eq!(ov.category, DomainCategory::Events);
    assert_eq!(ov.descriptions.len(), 1);
    // The override's parent is the domain, not the version.
    assert_eq!(ov.domain_id, list[0].id);
    assert_ne!(ov.domain_id, version_id);
}

#[tokio::test]
async fn domain_update_keeps_the_override() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    uc.create_domain_override(CreateSdtmProjectDomainOverride {
        domain_id,
        category: DomainCategory::Events,
        descriptions: Vec::new(),
    })
    .await
    .unwrap();

    let updated = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: domain_id,
            domain_code: Some("AE2".into()),
            key_variables: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.domain_code, "AE2");
    assert!(updated.domain_override.is_some());
}

#[tokio::test]
async fn domain_override_crud_round_trips() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;

    let created = uc
        .create_domain_override(CreateSdtmProjectDomainOverride {
            domain_id,
            category: DomainCategory::Events,
            descriptions: vec![domain_description()],
        })
        .await
        .unwrap();
    assert_eq!(created.category, DomainCategory::Events);

    let got = uc
        .get_domain_override(domain_id)
        .await
        .unwrap()
        .expect("the override exists");
    assert_eq!(got.id, created.id);

    let updated = uc
        .update_domain_override(UpdateSdtmProjectDomainOverride {
            domain_id,
            category: Some(DomainCategory::Findings),
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.category, DomainCategory::Findings);

    uc.delete_domain_override(domain_id).await.unwrap();
    // The override is optional: once it is gone, fetching it yields
    // `None` rather than an error.
    assert!(uc.get_domain_override(domain_id).await.unwrap().is_none());
}

#[tokio::test]
async fn domain_override_create_rejects_missing_domain() {
    let uc = build_usecase();
    let err = uc
        .create_domain_override(CreateSdtmProjectDomainOverride {
            domain_id: 999,
            category: DomainCategory::Events,
            descriptions: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::FkSdtmProjectDomainNotFound(999))
    ));
}

// ---- variable tests ------------------------------------------------------

#[tokio::test]
async fn variable_crud_round_trips() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;

    let v = uc
        .create_variable(CreateSdtmProjectVariable {
            domain_id,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
        })
        .await
        .unwrap();
    assert_eq!(v.variable_code, "AETERM");
    assert_eq!(v.variable_sequence, 11);

    let list = uc.list_variables_by_domain(domain_id).await.unwrap();
    assert_eq!(list.len(), 1);

    let updated = uc
        .update_variable(UpdateSdtmProjectVariable {
            id: v.id,
            variable_code: None,
            variable_sequence: Some(12),
        })
        .await
        .unwrap();
    assert_eq!(updated.variable_sequence, 12);

    uc.delete_variable(v.id).await.unwrap();
    assert!(
        uc.list_variables_by_domain(domain_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn variable_create_rejects_missing_domain() {
    let uc = build_usecase();
    let err = uc
        .create_variable(CreateSdtmProjectVariable {
            domain_id: 999,
            variable_code: "AETERM".into(),
            variable_sequence: 1,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::FkSdtmProjectDomainNotFound(999))
    ));
}

#[tokio::test]
async fn variable_override_crud_round_trips() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    let variable_id = uc
        .create_variable(CreateSdtmProjectVariable {
            domain_id,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
        })
        .await
        .unwrap()
        .id;

    let created = uc
        .create_variable_override(CreateSdtmProjectVariableOverride {
            variable_id,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: Some(SdtmRole::Topic),
            descriptions: vec![variable_description()],
        })
        .await
        .unwrap();
    assert_eq!(created.variable_type, SdtmVariableType::Character);
    assert_eq!(created.variable_role, Some(SdtmRole::Topic));

    let got = uc
        .get_variable_override(variable_id)
        .await
        .unwrap()
        .expect("the override exists");
    assert_eq!(got.id, created.id);

    let updated = uc
        .update_variable_override(UpdateSdtmProjectVariableOverride {
            variable_id,
            variable_type: None,
            variable_core: None,
            variable_role: None,
            descriptions: Some(Vec::new()),
        })
        .await
        .unwrap();
    assert!(updated.descriptions.is_empty());
    assert_eq!(updated.variable_role, Some(SdtmRole::Topic));

    uc.delete_variable_override(variable_id).await.unwrap();
    // The override is optional: once it is gone, fetching it yields
    // `None` rather than an error.
    assert!(
        uc.get_variable_override(variable_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn variable_override_create_rejects_missing_variable() {
    let uc = build_usecase();
    let err = uc
        .create_variable_override(CreateSdtmProjectVariableOverride {
            variable_id: 999,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: None,
            descriptions: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::FkSdtmProjectVariableNotFound(999))
    ));
}

#[tokio::test]
async fn variable_override_update_clears_the_role() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    let variable_id = uc
        .create_variable(CreateSdtmProjectVariable {
            domain_id,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
        })
        .await
        .unwrap()
        .id;
    uc.create_variable_override(CreateSdtmProjectVariableOverride {
        variable_id,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        descriptions: Vec::new(),
    })
    .await
    .unwrap();

    let updated = uc
        .update_variable_override(UpdateSdtmProjectVariableOverride {
            variable_id,
            variable_type: None,
            variable_core: None,
            variable_role: Some(None),
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.variable_role, None);
}
