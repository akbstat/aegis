//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with
//! `AEGIS_TERMINOLOGY_DATABASE_URL` set:
//!
//! ```text
//! cargo test -p terminology -- --ignored --test-threads=1
//! ```
//!
//! Each run drops and re-applies the migrations so the live DB
//! stays in a deterministic state. A failure to connect is
//! reported via a clear panic (never silently skipped).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;
use terminology::{
    CodeItem, CodeItemNew, CodeItemRepo, CodeItemRepository, CodeList, CodeListListQuery,
    CodeListNew, CodeListRepo, CodeListRepository, CreateTerminologyVersion, DomainError,
    TerminologyKind, TerminologyUsecase, TerminologyUsecaseConfig, TerminologyVersion,
    TerminologyVersionNew, TerminologyVersionRepo, TerminologyVersionRepository,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_TERMINOLOGY_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_TERMINOLOGY_DATABASE_URL must be set (or present in .env \
             at the workspace root) to run --ignored tests"
        )
    });

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_TERMINOLOGY_DATABASE_URL");

    sqlx::query("DROP TABLE IF EXISTS code_items CASCADE")
        .execute(&pool)
        .await
        .expect("drop code_items");
    sqlx::query("DROP TABLE IF EXISTS code_lists CASCADE")
        .execute(&pool)
        .await
        .expect("drop code_lists");
    sqlx::query("DROP TABLE IF EXISTS terminology_versions CASCADE")
        .execute(&pool)
        .await
        .expect("drop terminology_versions");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .expect("drop sqlx_migrations bookkeeping");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

    f(pool).await
}

fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn create_then_find_round_trip_for_all_three_levels() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v_name = unique("v");
        let v: TerminologyVersion = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: v_name.clone(),
            })
            .await
            .expect("version create");

        let cl: CodeList = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("cl"),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_list create");

        let item: CodeItem = i_repo
            .create(CodeItemNew {
                codelist_id: cl.id,
                version_id: v.id,
                code: unique("ci"),
                submission_value: ">0".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_item create");

        assert_eq!(v.name, v_name);
        assert_eq!(cl.version_id, v.id);
        assert_eq!(item.codelist_id, cl.id);
        assert_eq!(item.version_id, v.id);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn delete_version_cascades_to_children() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("cascade-v"),
            })
            .await
            .expect("version");

        let cl = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("cascade-cl"),
                extensible: false,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_list");

        let _item = i_repo
            .create(CodeItemNew {
                codelist_id: cl.id,
                version_id: v.id,
                code: unique("cascade-ci"),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_item");

        v_repo.delete(v.id).await.expect("delete version");

        let err = l_repo.find_by_id(cl.id).await.expect_err("cl gone");
        assert!(
            matches!(err, DomainError::CodeListNotFound(_)),
            "expected CodeListNotFound, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_lists_paginates_across_multiple_pages() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("page-v"),
            })
            .await
            .expect("version");

        for i in 0..7 {
            l_repo
                .create(CodeListNew {
                    version_id: v.id,
                    code: format!("page-cl-{i}"),
                    extensible: true,
                    name: format!("Codelist {i}"),
                    submission_value: format!("SV{i}"),
                    synonym: "".into(),
                    definition: "".into(),
                    nci_preferred_term: "".into(),
                })
                .await
                .expect("create");
        }

        // page 1
        let p1 = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 0,
                limit: 3,
            })
            .await
            .expect("page 1");
        assert_eq!(p1.items.len(), 3);
        assert_eq!(p1.next_offset, Some(3));

        // page 2
        let p2 = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 3,
                limit: 3,
            })
            .await
            .expect("page 2");
        assert_eq!(p2.items.len(), 3);
        assert_eq!(p2.next_offset, Some(6));

        // page 3 (final, only 1 row)
        let p3 = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 6,
                limit: 3,
            })
            .await
            .expect("page 3");
        assert_eq!(p3.items.len(), 1);
        assert_eq!(p3.next_offset, None);

        // offset >= total
        let empty = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 100,
                limit: 3,
            })
            .await
            .expect("empty page");
        assert!(empty.items.is_empty());
        assert_eq!(empty.next_offset, None);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn usecase_wires_through_all_three_repos() {
    with_pool(|pool| async move {
        let v = TerminologyVersionRepo::new(pool.clone());
        let l = CodeListRepo::new(pool.clone());
        let i = CodeItemRepo::new(pool.clone());
        let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
            version_repo: v,
            code_list_repo: l,
            code_item_repo: i,
        });

        let v_name = unique("usecase-v");
        let created_v = usecase
            .create_version(CreateTerminologyVersion {
                kind: TerminologyKind::Sdtm,
                name: v_name.clone(),
            })
            .await
            .expect("usecase create version");
        let _ = usecase
            .get_version_by_id(created_v.id)
            .await
            .expect("usecase get");
        assert_eq!(created_v.name, v_name);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_items_by_version_and_code_end_to_end() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("natural-v"),
            })
            .await
            .expect("version");

        let age_code = unique("natural-age");
        let sex_code = unique("natural-sex");
        let age = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: age_code.clone(),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("age cl");
        let sex = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: sex_code.clone(),
                extensible: true,
                name: "SEX".into(),
                submission_value: "SEX".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("sex cl");

        // Item code "C1" appears in both codelists of v.
        i_repo
            .create(CodeItemNew {
                codelist_id: age.id,
                version_id: v.id,
                code: "C1".into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("age C1");
        let sex_c1 = i_repo
            .create(CodeItemNew {
                codelist_id: sex.id,
                version_id: v.id,
                code: "C1".into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("sex C1");
        i_repo
            .create(CodeItemNew {
                codelist_id: age.id,
                version_id: v.id,
                code: "C2".into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("age C2");

        let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
            version_repo: v_repo,
            code_list_repo: l_repo,
            code_item_repo: i_repo,
        });

        // (v, "C1") hits one row in each codelist.
        let c1_items = usecase
            .list_code_items_by_version_and_code(v.id, "C1")
            .await
            .expect("C1 lookup");
        assert_eq!(c1_items.len(), 2);
        assert!(
            c1_items.iter().all(|i| i.version_id == v.id),
            "all returned items carry the version_id"
        );
        assert!(
            c1_items.iter().any(|i| i.codelist_id == age.id),
            "AGE row included"
        );
        assert!(
            c1_items
                .iter()
                .any(|i| i.codelist_id == sex.id && i.id == sex_c1.id),
            "SEX row included"
        );
        assert!(
            c1_items.iter().all(|i| i.code == "C1"),
            "all returned items use the requested code"
        );

        // (v, "C2") matches only the AGE entry.
        let c2_items = usecase
            .list_code_items_by_version_and_code(v.id, "C2")
            .await
            .expect("C2 lookup");
        assert_eq!(c2_items.len(), 1);
        assert_eq!(c2_items[0].codelist_id, age.id);

        // Unknown code returns empty.
        let empty = usecase
            .list_code_items_by_version_and_code(v.id, "C99999")
            .await
            .expect("missing lookup");
        assert!(empty.is_empty());

        // The codelist NCI code is irrelevant to the lookup; we
        // only filter by `(version_id, code_items.code)`.
        let _ = &age_code;
        let _ = &sex_code;
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_items_paginates_across_multiple_pages() {
    use terminology::CodeItemListQuery;
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("item-page-v"),
            })
            .await
            .expect("version");
        let cl = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("item-page-cl"),
                extensible: true,
                name: "items".into(),
                submission_value: "items".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("codelist");

        for i in 0..5 {
            i_repo
                .create(CodeItemNew {
                    codelist_id: cl.id,
                    version_id: v.id,
                    code: format!("CI{i}"),
                    submission_value: format!("SV{i}"),
                    synonym: "".into(),
                    definition: "".into(),
                    nci_preferred_term: "".into(),
                })
                .await
                .expect("create");
        }

        // Page 1: limit=2 → 2 items + nextOffset=2.
        let p1 = i_repo
            .search_or_list(CodeItemListQuery {
                version_id: None,
                codelist_id: Some(cl.id),
                fragment: None,
                offset: 0,
                limit: 2,
            })
            .await
            .expect("page 1");
        assert_eq!(p1.items.len(), 2);
        assert_eq!(p1.next_offset, Some(2));

        // Page 2: offset=2, limit=2 → 2 items + nextOffset=4.
        let p2 = i_repo
            .search_or_list(CodeItemListQuery {
                version_id: None,
                codelist_id: Some(cl.id),
                fragment: None,
                offset: 2,
                limit: 2,
            })
            .await
            .expect("page 2");
        assert_eq!(p2.items.len(), 2);
        assert_eq!(p2.next_offset, Some(4));

        // Page 3: offset=4, limit=2 → 1 item, no nextOffset.
        let p3 = i_repo
            .search_or_list(CodeItemListQuery {
                version_id: None,
                codelist_id: Some(cl.id),
                fragment: None,
                offset: 4,
                limit: 2,
            })
            .await
            .expect("page 3");
        assert_eq!(p3.items.len(), 1);
        assert_eq!(p3.next_offset, None);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_lists_with_fragment_uses_full_text_search() {
    use terminology::CodeListListQuery;
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("fts-v"),
            })
            .await
            .expect("version");

        let age = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("fts-age"),
                extensible: true,
                name: "AGE codelist".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "Subject age".into(),
                nci_preferred_term: "Age".into(),
            })
            .await
            .expect("age");
        let sex = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("fts-sex"),
                extensible: true,
                name: "SEX codelist".into(),
                submission_value: "SEX".into(),
                synonym: "".into(),
                definition: "Subject sex".into(),
                nci_preferred_term: "Sex".into(),
            })
            .await
            .expect("sex");

        // Prefix-match: "ag" must hit AGE but not SEX. Postgres
        // FTS uses the `tsv` column with a GIN index, so the
        // adapter wraps the fragment in `{frag}:*`.
        let page = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: Some("ag".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .expect("search ag");
        let ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![age.id], "AG prefix hits AGE only");
        assert_eq!(page.next_offset, None);

        // Word fragment: "sex" hits SEX only.
        let page = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: Some("sex".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .expect("search sex");
        let ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![sex.id], "sex hits SEX only");
        assert_eq!(page.next_offset, None);

        // Empty fragment falls through to the plain list path.
        let page = l_repo
            .search_or_list(CodeListListQuery {
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
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn multi_word_fragment_matches_every_term_in_postgres() {
    // Regression for the `syntax error in tsquery: "vital sign"`
    // crash: `to_tsquery` parses a tsquery *expression*, so a
    // fragment carrying several words needs an explicit `&` between
    // the terms. The adapter builds that expression.
    use terminology::CodeItemListQuery;
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("multi-v"),
            })
            .await
            .expect("version");

        let signs = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("multi-cl-signs"),
                extensible: true,
                name: "Vital Signs".into(),
                submission_value: "VITAL SIGNS".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("vital signs codelist");
        let status = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("multi-cl-status"),
                extensible: true,
                name: "Vital Status".into(),
                submission_value: "VITAL STATUS".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("vital status codelist");

        // "vital sign" becomes "vital:* & sign:*" — both terms have to
        // match, so "Vital Status" is filtered out.
        let page = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: Some("vital sign".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .expect("multi-word codelist search");
        let ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![signs.id], "every term of the fragment must match");

        // The same holds for code items.
        let signs_item = i_repo
            .create(CodeItemNew {
                codelist_id: signs.id,
                version_id: v.id,
                code: unique("multi-i-signs"),
                submission_value: "Vital Signs".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("vital signs item");
        i_repo
            .create(CodeItemNew {
                codelist_id: status.id,
                version_id: v.id,
                code: unique("multi-i-status"),
                submission_value: "Vital Status".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("vital status item");

        let page = i_repo
            .search_or_list(CodeItemListQuery {
                version_id: Some(v.id),
                codelist_id: None,
                fragment: Some("vital sign".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .expect("multi-word code item search");
        let ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![signs_item.id],
            "every term of the fragment must match"
        );

        // A single term keeps the prefix-match behaviour.
        let page = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: Some("vital".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .expect("single-word search");
        let mut ids: Vec<i64> = page.items.iter().map(|c| c.id).collect();
        ids.sort();
        let mut expected = vec![signs.id, status.id];
        expected.sort();
        assert_eq!(ids, expected);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_items_filters_by_version_id_in_postgres() {
    // Regression for `CodeItemListQuery::version_id: Option<i64>`
    // on the Postgres path: when the caller supplies
    // `version_id`, only items whose `version_id` matches must
    // come back, even when `codelist_id` is omitted.
    use terminology::CodeItemListQuery;
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v1 = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("item-vid-v1"),
            })
            .await
            .expect("version 1");
        let v2 = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("item-vid-v2"),
            })
            .await
            .expect("version 2");
        let cl1 = l_repo
            .create(CodeListNew {
                version_id: v1.id,
                code: unique("item-vid-cl1"),
                extensible: true,
                name: "v1-codelist".into(),
                submission_value: "sv".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("codelist 1");
        let cl2 = l_repo
            .create(CodeListNew {
                version_id: v2.id,
                code: unique("item-vid-cl2"),
                extensible: true,
                name: "v2-codelist".into(),
                submission_value: "sv".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("codelist 2");

        i_repo
            .create(CodeItemNew {
                codelist_id: cl1.id,
                version_id: v1.id,
                code: "ONLY_V1".into(),
                submission_value: "sv".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("item in v1");
        i_repo
            .create(CodeItemNew {
                codelist_id: cl2.id,
                version_id: v2.id,
                code: "ONLY_V2".into(),
                submission_value: "sv".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("item in v2");

        let page = i_repo
            .search_or_list(CodeItemListQuery {
                version_id: Some(v1.id),
                codelist_id: None,
                fragment: None,
                offset: 0,
                limit: 50,
            })
            .await
            .expect("filter by version_id");
        let mut codes: Vec<String> = page.items.iter().map(|i| i.code.clone()).collect();
        codes.sort();
        assert_eq!(codes, vec!["ONLY_V1".to_string()]);
    })
    .await;
}
