//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with
//! `AEGIS_TEST_DATABASE_URL` set:
//!
//! ```text
//! cargo test -p domain-model -- --ignored --test-threads=1
//! ```
//!
//! Each run drops and re-applies the migrations so the live DB
//! stays in a deterministic state. A failure to connect is
//! reported via a clear panic (never silently skipped).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use domain_model::{
    DomainCategory, DomainError, DomainModelUsecase, DomainModelUsecaseConfig, SdtmDomain,
    SdtmDomainDescription, SdtmDomainDescriptionDetail, SdtmDomainNew, SdtmDomainRepoPg,
    SdtmDomainRepository, SdtmDomainUpdate, SdtmRole, SdtmVariableControlled, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableNew, SdtmVariableRepoPg,
    SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate, SdtmVersion, SdtmVersionNew,
    SdtmVersionRepoPg, SdtmVersionRepository, SdtmVersionUpdate,
};
use sqlx::PgPool;

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_TEST_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_TEST_DATABASE_URL must be set (or present in .env \
             at the workspace root) to run --ignored tests"
        )
    });

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_TEST_DATABASE_URL");

    // Drop in reverse FK order so cascading rules never matter.
    sqlx::query("DROP TABLE IF EXISTS sdtm_variables CASCADE")
        .execute(&pool)
        .await
        .expect("drop sdtm_variables");
    sqlx::query("DROP TABLE IF EXISTS sdtm_domains CASCADE")
        .execute(&pool)
        .await
        .expect("drop sdtm_domains");
    sqlx::query("DROP TABLE IF EXISTS sdtm_versions CASCADE")
        .execute(&pool)
        .await
        .expect("drop sdtm_versions");
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

// ---- SdtmVersion -------------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_create_then_list_round_trips() {
    with_pool(|pool| async move {
        let repo = SdtmVersionRepoPg::new(pool);
        let v: SdtmVersion = repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .expect("version create");
        assert!(v.id > 0);
        assert!(v.created_at <= v.updated_at);

        let listed = repo.list().await.expect("list versions");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, v.id);
        assert_eq!(listed[0].name, v.name);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_update_changes_name() {
    with_pool(|pool| async move {
        let repo = SdtmVersionRepoPg::new(pool);
        let v = repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();

        let new_name = unique("v");
        let updated = repo
            .update(SdtmVersionUpdate {
                id: v.id,
                name: Some(new_name.clone()),
            })
            .await
            .expect("version update");
        assert_eq!(updated.name, new_name);
        assert!(updated.updated_at >= v.updated_at);

        let fetched = repo.list().await.unwrap();
        assert_eq!(fetched[0].name, new_name);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_duplicate_name_returns_duplicate_error() {
    with_pool(|pool| async move {
        let repo = SdtmVersionRepoPg::new(pool);
        let name = unique("v");
        repo.create(SdtmVersionNew { name: name.clone() })
            .await
            .expect("first version create");
        let err = repo
            .create(SdtmVersionNew { name })
            .await
            .expect_err("second create must fail");
        assert!(
            matches!(err, DomainError::DuplicateSdtmVersion { .. }),
            "got {err:?}"
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_delete_then_recreate_works() {
    with_pool(|pool| async move {
        let repo = SdtmVersionRepoPg::new(pool);
        let v = repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        repo.delete(v.id).await.expect("version delete");
        let listed = repo.list().await.unwrap();
        assert_eq!(listed.len(), 0);

        // The slot is free: re-creating with a fresh name works.
        let v2 = repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .expect("re-create after delete");
        assert_ne!(v.id, v2.id);
    })
    .await
}

// ---- SdtmDomain --------------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_create_then_list_round_trips_with_descriptions() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();

        let descs = vec![
            SdtmDomainDescription {
                lang: "en".into(),
                details: SdtmDomainDescriptionDetail {
                    description: "Adverse Events".into(),
                    structure: "One record per event".into(),
                },
            },
            SdtmDomainDescription {
                lang: "ja".into(),
                details: SdtmDomainDescriptionDetail {
                    description: "有害事象".into(),
                    structure: "イベント毎に1レコード".into(),
                },
            },
        ];

        let d: SdtmDomain = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: descs.clone(),
            })
            .await
            .expect("domain create");
        assert_eq!(d.version_id, v.id);
        assert_eq!(d.descriptions.len(), 2);
        assert_eq!(d.descriptions[0].lang, "en");
        assert_eq!(d.descriptions[1].details.description, "有害事象");

        let listed = d_repo.list_by_version(v.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, d.id);
        assert_eq!(listed[0].descriptions.len(), 2);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_duplicate_returns_duplicate_error() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool);
        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let name = unique("AE");
        d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: name.clone(),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .expect("first domain");
        let err = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name,
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .expect_err("duplicate");
        assert!(
            matches!(err, DomainError::DuplicateSdtmDomain { .. }),
            "got {err:?}"
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_missing_parent_version_returns_fk_error() {
    with_pool(|pool| async move {
        let d_repo = SdtmDomainRepoPg::new(pool);
        let err = d_repo
            .create(SdtmDomainNew {
                version_id: 999_999,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .expect_err("FK violation");
        assert!(
            matches!(err, DomainError::FkSdtmVersionNotFound(_)),
            "got {err:?}"
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_update_with_partial_fields() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool);
        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();

        let updated = d_repo
            .update(SdtmDomainUpdate {
                id: d.id,
                name: None,
                category: Some(DomainCategory::Findings),
                descriptions: None,
            })
            .await
            .expect("domain update");
        assert_eq!(updated.name, d.name); // unchanged
        assert_eq!(updated.category, DomainCategory::Findings); // replaced

        // `Some(vec![])` clears the descriptions column.
        let cleared = d_repo
            .update(SdtmDomainUpdate {
                id: d.id,
                name: None,
                category: None,
                descriptions: Some(vec![]),
            })
            .await
            .expect("descriptions clear");
        assert!(cleared.descriptions.is_empty());
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_delete_cascades_to_variables() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();
        let var = va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name: unique("AETERM"),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                variable_sequence: 1,
                descriptions: vec![],
            })
            .await
            .unwrap();
        assert_eq!(va_repo.list_by_domain(d.id).await.unwrap().len(), 1);

        d_repo.delete(d.id).await.expect("domain delete");
        // The variable row is gone too (ON DELETE CASCADE).
        let err = va_repo.find_by_id(var.id).await.expect_err("var gone");
        assert!(matches!(err, DomainError::SdtmVariableNotFound(_)));
    })
    .await
}

// ---- SdtmVariable ------------------------------------------------------

/// A list that is present but empty must survive the JSONB round-trip
/// as *present* — `Some(vec![])` and "absent" are different values,
/// and the column must not collapse one into the other.
#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_with_present_but_empty_code_list_round_trips_as_present() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();

        let var = va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name: unique("AETERM"),
                variable_controlled: Some(SdtmVariableControlled {
                    code: Some(vec![]),
                    format: None,
                }),
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                variable_sequence: 1,
                descriptions: vec![],
            })
            .await
            .expect("variable create");
        assert_eq!(
            var.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec![]),
                format: None,
            })
        );

        let fetched = va_repo.find_by_id(var.id).await.unwrap();
        assert_eq!(
            fetched.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec![]),
                format: None,
            })
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_create_then_list_round_trips_with_descriptions() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();

        let descs = vec![SdtmVariableDescription {
            lang: "en".into(),
            details: SdtmVariableDescriptionDetail {
                label: "Reported Term".into(),
            },
        }];

        let var = va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name: unique("AETERM"),
                variable_controlled: Some(SdtmVariableControlled {
                    code: Some(vec!["AETERM".into()]),
                    format: None,
                }),
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                variable_sequence: 1,
                descriptions: descs.clone(),
            })
            .await
            .expect("variable create");
        assert_eq!(
            var.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["AETERM".into()]),
                format: None,
            })
        );
        assert_eq!(var.descriptions.len(), 1);
        assert_eq!(var.descriptions[0].details.label, "Reported Term");

        let listed = va_repo.list_by_domain(d.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, var.id);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_three_state_update_does_not_change_when_outer_none() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();
        let var = va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name: unique("AETERM"),
                variable_controlled: Some(SdtmVariableControlled {
                    code: Some(vec!["AETERM".into()]),
                    format: None,
                }),
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                variable_sequence: 1,
                descriptions: vec![],
            })
            .await
            .unwrap();

        let updated = va_repo
            .update(SdtmVariableUpdate {
                id: var.id,
                name: None,
                variable_controlled: None,
                variable_type: None,
                variable_core: None,
                variable_role: None,
                variable_sequence: None,
                descriptions: None,
            })
            .await
            .expect("noop update");
        assert_eq!(
            updated.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["AETERM".into()]),
                format: None,
            })
        );
        assert_eq!(updated.variable_role, Some(SdtmRole::Topic));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_three_state_update_replaces_when_outer_some_inner_some() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();
        let var = va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name: unique("AETERM"),
                variable_controlled: Some(SdtmVariableControlled {
                    code: Some(vec!["AETERM".into()]),
                    format: None,
                }),
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                variable_sequence: 1,
                descriptions: vec![],
            })
            .await
            .unwrap();

        let updated = va_repo
            .update(SdtmVariableUpdate {
                id: var.id,
                name: None,
                variable_controlled: Some(Some(SdtmVariableControlled {
                    code: Some(vec!["AETERMCD".into()]),
                    format: None,
                })),
                variable_type: None,
                variable_core: None,
                variable_role: Some(Some(SdtmRole::Identifier)),
                variable_sequence: None,
                descriptions: None,
            })
            .await
            .expect("replace update");
        assert_eq!(
            updated.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["AETERMCD".into()]),
                format: None,
            })
        );
        assert_eq!(updated.variable_role, Some(SdtmRole::Identifier));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_three_state_update_clears_when_outer_some_inner_none() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();
        let var = va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name: unique("AETERM"),
                variable_controlled: Some(SdtmVariableControlled {
                    code: Some(vec!["AETERM".into()]),
                    format: None,
                }),
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                variable_sequence: 1,
                descriptions: vec![],
            })
            .await
            .unwrap();

        let updated = va_repo
            .update(SdtmVariableUpdate {
                id: var.id,
                name: None,
                variable_controlled: Some(None),
                variable_type: None,
                variable_core: None,
                variable_role: Some(None),
                variable_sequence: None,
                descriptions: None,
            })
            .await
            .expect("clear update");
        assert_eq!(updated.variable_controlled, None);
        assert_eq!(updated.variable_role, None);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_duplicate_returns_duplicate_error() {
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let v = v_repo
            .create(SdtmVersionNew { name: unique("v") })
            .await
            .unwrap();
        let d = d_repo
            .create(SdtmDomainNew {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();
        let name = unique("AETERM");
        va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name: name.clone(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 1,
                descriptions: vec![],
            })
            .await
            .expect("first variable");
        let err = va_repo
            .create(SdtmVariableNew {
                domain_id: d.id,
                name,
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 2,
                descriptions: vec![],
            })
            .await
            .expect_err("duplicate");
        assert!(
            matches!(err, DomainError::DuplicateSdtmVariable { .. }),
            "got {err:?}"
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_missing_parent_domain_returns_fk_error() {
    with_pool(|pool| async move {
        let va_repo = SdtmVariableRepoPg::new(pool);
        let err = va_repo
            .create(SdtmVariableNew {
                domain_id: 999_999,
                name: unique("AETERM"),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 1,
                descriptions: vec![],
            })
            .await
            .expect_err("FK violation");
        assert!(
            matches!(err, DomainError::FkSdtmDomainNotFound(_)),
            "got {err:?}"
        );
    })
    .await
}

// ---- end-to-end usecase over Pg ---------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn usecase_end_to_end_against_pg() {
    use domain_model::CreateSdtmDomain;
    with_pool(|pool| async move {
        let v_repo = SdtmVersionRepoPg::new(pool.clone());
        let d_repo = SdtmDomainRepoPg::new(pool.clone());
        let va_repo = SdtmVariableRepoPg::new(pool);

        let usecase = DomainModelUsecase::new(DomainModelUsecaseConfig {
            version_repo: v_repo,
            domain_repo: d_repo,
            variable_repo: va_repo,
        });

        let v = usecase
            .create_version(domain_model::CreateSdtmVersion { name: unique("v") })
            .await
            .unwrap();

        let d = usecase
            .create_domain(CreateSdtmDomain {
                version_id: v.id,
                name: unique("AE"),
                category: DomainCategory::Events,
                descriptions: vec![],
            })
            .await
            .unwrap();

        let listed = usecase.list_domains_by_version(v.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, d.id);
    })
    .await
}

// helper: domain description detail literal
