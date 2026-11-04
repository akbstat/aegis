//! Tests for the usecase layer wired against in-memory repository
//! fakes. No SQL, no I/O.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use crate::domain::{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemRepository, CodeItemUpdate, CodeList,
    CodeListListQuery, CodeListNew, CodeListRepository, CodeListUpdate, DomainError, Page,
    TerminologyKind, TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate, search_terms,
};
use crate::usecase::commands::{
    CreateCodeItem, CreateCodeList, CreateTerminologyVersion, UpdateCodeList,
    UpdateTerminologyVersion,
};
use crate::usecase::error::UsecaseError;
use crate::usecase::terminology_usecase::{TerminologyUsecase, TerminologyUsecaseConfig};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 18, 0, 0, 0).unwrap()
}

// ---------- in-memory fakes ----------

#[derive(Default)]
struct VersionsState {
    by_id: HashMap<i64, TerminologyVersion>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeVersionRepo {
    state: Arc<Mutex<VersionsState>>,
}

impl FakeVersionRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(VersionsState::default())),
        }
    }
}

#[async_trait]
impl TerminologyVersionRepository for FakeVersionRepo {
    async fn create(
        &self,
        input: TerminologyVersionNew,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|v| v.kind == input.kind && v.name == input.name)
        {
            return Err(DomainError::DuplicateVersion {
                kind: input.kind,
                name: input.name,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = now();
        let v = TerminologyVersion::for_repository(id, input.kind, input.name, ts, ts);
        s.by_id.insert(id, v.clone());
        Ok(v)
    }
    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::VersionNotFound(id))
    }
    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .values()
            .find(|v| v.kind == kind && v.name == name)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError> {
        Ok(self.state.lock().unwrap().by_id.values().cloned().collect())
    }
    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut s = self.state.lock().unwrap();
        let v = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::VersionNotFound(input.id))?;
        if let Some(kind) = input.kind {
            v.kind = kind;
        }
        if let Some(name) = input.name {
            v.name = name;
        }
        v.updated_at = now();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::VersionNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct ListsState {
    by_id: HashMap<i64, CodeList>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeCodeListRepo {
    state: Arc<Mutex<ListsState>>,
}

impl FakeCodeListRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ListsState::default())),
        }
    }
}

#[async_trait]
impl CodeListRepository for FakeCodeListRepo {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|c| c.version_id == input.version_id && c.code == input.code)
        {
            return Err(DomainError::DuplicateCodeList {
                version_id: input.version_id,
                code: input.code,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = now();
        let cl = CodeList::for_repository(
            id,
            input.version_id,
            input.code,
            input.extensible,
            input.name,
            input.submission_value,
            input.synonym,
            input.definition,
            input.nci_preferred_term,
            ts,
            ts,
        );
        s.by_id.insert(id, cl.clone());
        Ok(cl)
    }
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::CodeListNotFound(id))
    }
    async fn search_or_list(&self, q: CodeListListQuery) -> Result<Page<CodeList>, DomainError> {
        let mut all: Vec<CodeList> = self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|c| c.version_id == q.version_id)
            .cloned()
            .collect();

        if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
            let terms = search_terms::split(frag)
                .iter()
                .map(|t| t.to_lowercase())
                .collect::<Vec<_>>();
            all.retain(|cl| {
                let haystack = [
                    cl.name.as_str(),
                    cl.submission_value.as_str(),
                    cl.synonym.as_str(),
                    cl.definition.as_str(),
                    cl.nci_preferred_term.as_str(),
                ]
                .join(" ")
                .to_lowercase();
                terms.iter().all(|term| haystack.contains(term.as_str()))
            });
        }

        all.sort_by_key(|cl| cl.id);
        let limit = q.limit as usize;
        let offset = q.offset as usize;
        let mut items: Vec<CodeList> = all.into_iter().skip(offset).take(limit + 1).collect();
        let next_offset = if items.len() > limit {
            items.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        Ok(Page { items, next_offset })
    }
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError> {
        let mut s = self.state.lock().unwrap();
        let c = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::CodeListNotFound(input.id))?;
        if let Some(code) = input.code {
            c.code = code;
        }
        if let Some(ext) = input.extensible {
            c.extensible = ext;
        }
        if let Some(name) = input.name {
            c.name = name;
        }
        if let Some(sv) = input.submission_value {
            c.submission_value = sv;
        }
        if let Some(syn) = input.synonym {
            c.synonym = syn;
        }
        if let Some(def) = input.definition {
            c.definition = def;
        }
        if let Some(pt) = input.nci_preferred_term {
            c.nci_preferred_term = pt;
        }
        c.updated_at = now();
        Ok(c.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::CodeListNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct ItemsState {
    by_id: HashMap<i64, CodeItem>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeCodeItemRepo {
    state: Arc<Mutex<ItemsState>>,
}

impl FakeCodeItemRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ItemsState::default())),
        }
    }
}

#[async_trait]
impl CodeItemRepository for FakeCodeItemRepo {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|i| i.codelist_id == input.codelist_id && i.code == input.code)
        {
            return Err(DomainError::DuplicateCodeItem {
                codelist_id: input.codelist_id,
                code: input.code,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = now();
        let item = CodeItem::for_repository(
            id,
            input.codelist_id,
            input.version_id,
            input.code,
            input.submission_value,
            input.synonym,
            input.definition,
            input.nci_preferred_term,
            ts,
            ts,
        );
        s.by_id.insert(id, item.clone());
        Ok(item)
    }
    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::CodeItemNotFound(id))
    }
    async fn search_or_list(&self, q: CodeItemListQuery) -> Result<Page<CodeItem>, DomainError> {
        let mut all: Vec<CodeItem> = self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|i| q.codelist_id.map_or(true, |cid| i.codelist_id == cid))
            .cloned()
            .collect();

        if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
            let terms = search_terms::split(frag)
                .iter()
                .map(|t| t.to_lowercase())
                .collect::<Vec<_>>();
            all.retain(|item| {
                let haystack = [
                    item.submission_value.as_str(),
                    item.synonym.as_str(),
                    item.definition.as_str(),
                    item.nci_preferred_term.as_str(),
                    item.code.as_str(),
                ]
                .join(" ")
                .to_lowercase();
                terms.iter().all(|term| haystack.contains(term.as_str()))
            });
        }

        all.sort_by_key(|i| i.id);
        let limit = q.limit as usize;
        let offset = q.offset as usize;
        let mut items: Vec<CodeItem> = all.into_iter().skip(offset).take(limit + 1).collect();
        let next_offset = if items.len() > limit {
            items.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        Ok(Page { items, next_offset })
    }
    async fn list_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItem>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|i| i.version_id == version_id && i.code == code)
            .cloned()
            .collect())
    }
    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError> {
        let mut s = self.state.lock().unwrap();
        let i = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::CodeItemNotFound(input.id))?;
        if let Some(code) = input.code {
            i.code = code;
        }
        if let Some(sv) = input.submission_value {
            i.submission_value = sv;
        }
        if let Some(syn) = input.synonym {
            i.synonym = syn;
        }
        if let Some(def) = input.definition {
            i.definition = def;
        }
        if let Some(pt) = input.nci_preferred_term {
            i.nci_preferred_term = pt;
        }
        i.updated_at = now();
        Ok(i.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::CodeItemNotFound(id));
        }
        Ok(())
    }
    async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError> {
        let mut s = self.state.lock().unwrap();
        let count = inputs.len();
        for input in inputs {
            if s.by_id
                .values()
                .any(|i| i.codelist_id == input.codelist_id && i.code == input.code)
            {
                return Err(DomainError::DuplicateCodeItem {
                    codelist_id: input.codelist_id,
                    code: input.code,
                });
            }
            let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
            let ts = now();
            let item = CodeItem::for_repository(
                id,
                input.codelist_id,
                input.version_id,
                input.code,
                input.submission_value,
                input.synonym,
                input.definition,
                input.nci_preferred_term,
                ts,
                ts,
            );
            s.by_id.insert(id, item);
        }
        Ok(count)
    }
}

// ---------- fixture ----------

fn make_usecase() -> (
    FakeVersionRepo,
    FakeCodeListRepo,
    FakeCodeItemRepo,
    TerminologyUsecase<FakeVersionRepo, FakeCodeListRepo, FakeCodeItemRepo>,
) {
    let v = FakeVersionRepo::new();
    let l = FakeCodeListRepo::new();
    let i = FakeCodeItemRepo::new();
    let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
        version_repo: v.clone(),
        code_list_repo: l.clone(),
        code_item_repo: i.clone(),
    });
    (v, l, i, usecase)
}

// ---------- tests ----------

#[tokio::test]
async fn create_version_rejects_empty_name() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyName)
    ));
}

#[tokio::test]
async fn create_then_get_version_by_id_round_trip() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
        })
        .await
        .expect("create");
    assert_eq!(created.name, "2026-03-27");
    let fetched = usecase.get_version_by_id(created.id).await.expect("get");
    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
async fn update_version_then_list_returns_updated_name() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_version(UpdateTerminologyVersion {
            id: created.id,
            name: Some("2026-06-15".into()),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.name, "2026-06-15");
    let listed = usecase.list_versions().await.expect("list");
    assert!(
        listed
            .iter()
            .any(|v| v.id == created.id && v.name == "2026-06-15")
    );
}

#[tokio::test]
async fn create_code_list_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_code_list(CreateCodeList {
            version_id: 1,
            code: "   ".into(),
            extensible: false,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn create_code_list_then_list_by_version_round_trip() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_list(CreateCodeList {
            version_id: 7,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "Age".into(),
            definition: "Age".into(),
            nci_preferred_term: "Age".into(),
        })
        .await
        .expect("create");
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 7,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .expect("list");
    assert!(page.items.iter().any(|c| c.id == created.id));
    assert_eq!(page.next_offset, None);
}

#[tokio::test]
async fn update_code_list_applies_partial_changes() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_list(CreateCodeList {
            version_id: 1,
            code: "C66741".into(),
            extensible: false,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_code_list(UpdateCodeList {
            id: created.id,
            extensible: Some(true),
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(updated.extensible);
    assert_eq!(updated.code, "C66741");
}

#[tokio::test]
async fn list_code_lists_with_fragment_filters_to_matches() {
    let (_, _, _, usecase) = make_usecase();

    // Create two codelists, one with "AGE" in its name, one with
    // "SEX". Both live under the same version.
    let v = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-age-sex".into(),
        })
        .await
        .expect("v");
    let age = usecase
        .create_code_list(CreateCodeList {
            version_id: v.id,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("age");
    let sex = usecase
        .create_code_list(CreateCodeList {
            version_id: v.id,
            code: "C66732".into(),
            extensible: true,
            name: "SEX".into(),
            submission_value: "SEX".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("sex");

    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: Some("AGE".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("search");
    let ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
    assert_eq!(ids, vec![age.id], "AGE fragment matches only AGE");
    assert_eq!(page.next_offset, None);

    // Empty fragment is treated as "no filter" and returns both.
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: Some(String::new()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("empty fragment");
    let mut ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
    ids.sort();
    assert_eq!(ids, vec![age.id, sex.id]);

    // Whitespace-only fragment likewise falls through to "no filter".
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: Some("   ".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("whitespace fragment");
    assert_eq!(page.items.len(), 2);
}

#[tokio::test]
async fn list_code_lists_pagination_signals_next_offset_and_terminates() {
    let (_, _, _, usecase) = make_usecase();
    let v = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-page".into(),
        })
        .await
        .expect("v");
    // 5 codelists under v.
    for i in 0..5 {
        usecase
            .create_code_list(CreateCodeList {
                version_id: v.id,
                code: format!("C{i}"),
                extensible: false,
                name: format!("LIST{i}"),
                submission_value: format!("LIST{i}"),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }

    // Page 1: limit=2 → 2 items + nextOffset = 2.
    let page1 = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: None,
            offset: 0,
            limit: 2,
        })
        .await
        .expect("page 1");
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.next_offset, Some(2));

    // Page 2: offset=2, limit=2 → 2 items + nextOffset = 4.
    let page2 = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: None,
            offset: 2,
            limit: 2,
        })
        .await
        .expect("page 2");
    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.next_offset, Some(4));

    // Page 3: offset=4, limit=2 → 1 item, no nextOffset (terminator).
    let page3 = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: None,
            offset: 4,
            limit: 2,
        })
        .await
        .expect("page 3");
    assert_eq!(page3.items.len(), 1);
    assert_eq!(page3.next_offset, None);
}

#[tokio::test]
async fn list_code_lists_rejects_fragment_with_reserved_tsquery_chars() {
    let (_, _, _, usecase) = make_usecase();
    for bad in ["a&b", "a|b", "a!b", "a(b", "a)b", "a:b", "a'b", "a\"b"] {
        let err = usecase
            .list_code_lists(CodeListListQuery {
                version_id: 1,
                fragment: Some(bad.into()),
                offset: 0,
                limit: 50,
            })
            .await
            .unwrap_err();
        assert!(
            matches!(err, UsecaseError::Validation(DomainError::InvalidFragment)),
            "fragment {bad:?} should be rejected with InvalidFragment, got {err:?}"
        );
    }
}

#[tokio::test]
async fn list_code_lists_accepts_multi_word_fragment_and_requires_every_term() {
    let (_, _, _, usecase) = make_usecase();

    let v = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-multi-word".into(),
        })
        .await
        .expect("v");
    let vital_signs = usecase
        .create_code_list(CreateCodeList {
            version_id: v.id,
            code: "C1".into(),
            extensible: true,
            name: "Vital Signs".into(),
            submission_value: "VITAL SIGNS".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("vital signs");
    let vital_status = usecase
        .create_code_list(CreateCodeList {
            version_id: v.id,
            code: "C2".into(),
            extensible: true,
            name: "Vital Status".into(),
            submission_value: "VITAL STATUS".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("vital status");

    // "vital sign" is two terms, both of which must match — so
    // "Vital Status" is filtered out.
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: Some("vital sign".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("multi-word search");
    let ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
    assert_eq!(
        ids,
        vec![vital_signs.id],
        "a multi-word fragment must match every term"
    );
    assert!(!ids.contains(&vital_status.id));

    // A single term still matches both rows.
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: Some("vital".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("single-word search");
    let mut ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
    ids.sort();
    let mut expected = vec![vital_signs.id, vital_status.id];
    expected.sort();
    assert_eq!(ids, expected);
}

#[tokio::test]
async fn list_code_lists_clamps_limit_to_max_when_excessive() {
    // limit=0 → server default 50; limit=u32::MAX → clamped to 500.
    let (_, _, _, usecase) = make_usecase();
    let v = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-clamp".into(),
        })
        .await
        .expect("v");
    // Create a few rows so we can confirm the clamp doesn't zero the page.
    for i in 0..3 {
        usecase
            .create_code_list(CreateCodeList {
                version_id: v.id,
                code: format!("C{i}"),
                extensible: false,
                name: format!("N{i}"),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: v.id,
            fragment: None,
            offset: 0,
            limit: u32::MAX,
        })
        .await
        .expect("clamp");
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.next_offset, None);
}

#[tokio::test]
async fn create_code_item_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: 1,
            version_id: 1,
            code: "".into(),
            submission_value: "X".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn create_code_item_round_trip_then_list_by_codelist() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: 9,
            version_id: 7,
            code: "C12345".into(),
            submission_value: "> 0".into(),
            synonym: "positive".into(),
            definition: "Greater than zero".into(),
            nci_preferred_term: "Greater Than Zero".into(),
        })
        .await
        .expect("create");
    let page = usecase
        .list_code_items(CodeItemListQuery {
            version_id: None,
            codelist_id: Some(9),
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .expect("list");
    assert!(page.items.iter().any(|i| i.id == created.id));
    assert_eq!(page.next_offset, None);
}

#[tokio::test]
async fn list_code_items_by_version_and_code_returns_matching_items() {
    let (_, _, _, usecase) = make_usecase();

    // Two versions, two codelists under v_a and one under v_b.
    // The same item code appears in multiple codelists so the
    // lookup must return more than one row.
    let v_a = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-a".into(),
        })
        .await
        .expect("v_a");
    let v_b = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-b".into(),
        })
        .await
        .expect("v_b");

    let age_a = usecase
        .create_code_list(CreateCodeList {
            version_id: v_a.id,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("age_a");
    let sex_a = usecase
        .create_code_list(CreateCodeList {
            version_id: v_a.id,
            code: "C66732".into(),
            extensible: true,
            name: "SEX".into(),
            submission_value: "SEX".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("sex_a");
    let age_b = usecase
        .create_code_list(CreateCodeList {
            version_id: v_b.id,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("age_b");

    // Code "C1" appears in age_a and sex_a (same version, two
    // codelists) — should return both.
    for (cl_id, code) in [(age_a.id, "C1"), (sex_a.id, "C1"), (age_a.id, "C2")] {
        usecase
            .create_code_item(CreateCodeItem {
                codelist_id: cl_id,
                version_id: v_a.id,
                code: code.into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("v_a item");
    }
    let v_b_item = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: age_b.id,
            version_id: v_b.id,
            code: "C1".into(),
            submission_value: "".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("v_b item");

    // (v_a, "C1") covers both codelists of v_a.
    let c1_a = usecase
        .list_code_items_by_version_and_code(v_a.id, "C1")
        .await
        .expect("lookup C1 in v_a");
    assert_eq!(
        c1_a.len(),
        2,
        "C1 appears in both AGE and SEX codelists of v_a"
    );
    assert!(
        c1_a.iter().all(|i| i.version_id == v_a.id),
        "all returned items are scoped to v_a"
    );
    let mut codelist_ids: Vec<i64> = c1_a.iter().map(|i| i.codelist_id).collect();
    codelist_ids.sort();
    assert_eq!(codelist_ids, vec![age_a.id, sex_a.id]);

    // (v_a, "C2") matches only the AGE entry.
    let c2_a = usecase
        .list_code_items_by_version_and_code(v_a.id, "C2")
        .await
        .expect("lookup C2 in v_a");
    assert_eq!(c2_a.len(), 1);
    assert_eq!(c2_a[0].codelist_id, age_a.id);

    // (v_b, "C1") must not bleed into v_a.
    let c1_b = usecase
        .list_code_items_by_version_and_code(v_b.id, "C1")
        .await
        .expect("lookup C1 in v_b");
    assert_eq!(c1_b.len(), 1);
    assert_eq!(c1_b[0].id, v_b_item.id);

    // Unknown code under a known version: empty, not an error.
    let empty = usecase
        .list_code_items_by_version_and_code(v_a.id, "C99999")
        .await
        .expect("lookup missing");
    assert!(empty.is_empty());
}

#[tokio::test]
async fn list_code_items_by_version_and_code_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .list_code_items_by_version_and_code(1, "   ")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}
