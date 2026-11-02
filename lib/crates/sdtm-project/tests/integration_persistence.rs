//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `AEGIS_SDTM_PROJECT_DATABASE_URL` set:
//!
//! ```text
//! cargo test -p sdtm-project -- --ignored --test-threads=1
//! ```
//!
//! Each run drops and re-applies the migrations so the live DB stays
//! in a deterministic state. A failure to connect is reported via a
//! clear panic (never silently skipped).
//!
//! ## The URL must point at a dedicated database
//!
//! SQLx keeps a single `_sqlx_migrations` table *per database*, shared
//! by every crate that migrates it. This test drops that table, so
//! pointing it at the server's database would erase the
//! applied-migration record for auth, user, project, terminology,
//! mission and crf alike. Use a database of its own.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sdtm_project::{
    DomainCategory, DomainError, SdtmProjectDomainNew, SdtmProjectDomainOverrideNew,
    SdtmProjectDomainOverrideRepoPg, SdtmProjectDomainOverrideRepository,
    SdtmProjectDomainOverrideUpdate, SdtmProjectDomainRepoPg, SdtmProjectDomainRepository,
    SdtmProjectDomainUpdate, SdtmProjectVariableNew, SdtmProjectVariableOverrideNew,
    SdtmProjectVariableOverrideRepoPg, SdtmProjectVariableOverrideRepository,
    SdtmProjectVariableOverrideUpdate, SdtmProjectVariableRepoPg, SdtmProjectVariableRepository,
    SdtmProjectVariableUpdate, SdtmProjectVersionNew, SdtmProjectVersionRepoPg,
    SdtmProjectVersionRepository, SdtmProjectVersionUpdate, SdtmRole, SdtmVariableCore,
    SdtmVariableType,
};
use sqlx::PgPool;

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_SDTM_PROJECT_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_SDTM_PROJECT_DATABASE_URL must be set (or present in .env \
             at the workspace root) to run --ignored tests. It must point at a \
             DEDICATED database: the test drops the _sqlx_migrations table, \
             which sqlx shares across every crate that migrates that database."
        )
    });

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_SDTM_PROJECT_DATABASE_URL");

    // Drop in reverse FK order so cascading rules never matter. The
    // statements are literals rather than a formatted list: sqlx
    // requires `&'static str`, and there is nothing to interpolate.
    for stmt in [
        "DROP TABLE IF EXISTS sdtm_project_variable_overrides CASCADE",
        "DROP TABLE IF EXISTS sdtm_project_variables CASCADE",
        "DROP TABLE IF EXISTS sdtm_project_domain_overrides CASCADE",
        "DROP TABLE IF EXISTS sdtm_project_domains CASCADE",
        "DROP TABLE IF EXISTS sdtm_project_versions CASCADE",
        "DROP TABLE IF EXISTS _sqlx_migrations CASCADE",
    ] {
        sqlx::query(stmt)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("drop {stmt}: {e}"));
    }

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

    f(pool).await
}

/// A per-run unique value for every column with a `UNIQUE` constraint,
/// so concurrent runs cannot collide.
fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

// ---- SdtmProjectVersion ---------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn version_create_then_list_round_trips() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let project_code = unique("p");
        let version_name = unique("v");
        let v = repo
            .create(SdtmProjectVersionNew {
                project_code: project_code.clone(),
                version_name: version_name.clone(),
            })
            .await
            .expect("version create");
        assert!(v.id > 0);
        assert!(v.created_at <= v.updated_at);
        assert_eq!(v.project_code, project_code);
        assert_eq!(v.version_name, version_name);

        let listed = repo.list_by_project(&project_code).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, v.id);

        let by_id = repo.find_by_id(v.id).await.expect("find by id");
        assert_eq!(by_id.version_name, version_name);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn version_update_changes_name() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let v = repo
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let new_name = unique("v");
        let updated = repo
            .update(SdtmProjectVersionUpdate {
                id: v.id,
                version_name: Some(new_name.clone()),
            })
            .await
            .expect("version update");
        assert_eq!(updated.version_name, new_name);
        assert_eq!(updated.project_code, v.project_code);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn version_update_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let err = repo
            .update(SdtmProjectVersionUpdate {
                id: 999_999,
                version_name: Some("x".into()),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVersionNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn version_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let err = repo.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVersionNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn version_unique_is_enforced_per_project() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let version_name = unique("v");
        // The same version name under two different projects is fine.
        repo.create(SdtmProjectVersionNew {
            project_code: "P-a".into(),
            version_name: version_name.clone(),
        })
        .await
        .expect("first project");
        repo.create(SdtmProjectVersionNew {
            project_code: "P-b".into(),
            version_name: version_name.clone(),
        })
        .await
        .expect("second project");

        // The same pair twice is a duplicate.
        let err = repo
            .create(SdtmProjectVersionNew {
                project_code: "P-a".into(),
                version_name: version_name.clone(),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::DuplicateSdtmProjectVersion { .. }
        ));
    })
    .await
}

// ---- SdtmProjectDomain ----------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_create_then_list_round_trips() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();

        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: vec!["AETERM".into()],
            })
            .await
            .expect("domain create");
        assert!(d.id > 0);
        assert_eq!(d.version_id, v.id);
        assert_eq!(d.domain_code, "AE");
        assert_eq!(d.key_variables, vec!["AETERM".to_string()]);
        assert!(d.domain_override.is_none());

        let listed = domains.list_by_version(v.id).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, d.id);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_key_variables_preserve_caller_order() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();

        // Deliberately not alphabetical: the caller's order is the
        // order that must survive the JSONB round trip.
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: vec!["AESEV".into(), "AETERM".into()],
            })
            .await
            .unwrap();
        assert_eq!(
            d.key_variables,
            vec!["AESEV".to_string(), "AETERM".to_string()]
        );

        let by_id = domains.find_by_id(d.id).await.unwrap();
        assert_eq!(
            by_id.key_variables,
            vec!["AESEV".to_string(), "AETERM".to_string()]
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_update_changes_code_and_key_variables() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: vec!["AETERM".into()],
            })
            .await
            .unwrap();

        let updated = domains
            .update(SdtmProjectDomainUpdate {
                id: d.id,
                domain_code: Some("AE2".into()),
                key_variables: Some(vec!["AESEV".into(), "AETERM".into()]),
            })
            .await
            .expect("domain update");
        assert_eq!(updated.domain_code, "AE2");
        assert_eq!(
            updated.key_variables,
            vec!["AESEV".to_string(), "AETERM".to_string()]
        );

        // `Some(vec![])` clears the column rather than leaving it.
        let cleared = domains
            .update(SdtmProjectDomainUpdate {
                id: d.id,
                domain_code: None,
                key_variables: Some(Vec::new()),
            })
            .await
            .unwrap();
        assert!(cleared.key_variables.is_empty());
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_update_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let err = domains
            .update(SdtmProjectDomainUpdate {
                id: 999_999,
                domain_code: Some("AE".into()),
                key_variables: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectDomainNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let err = domains.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectDomainNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_create_when_version_vanished_maps_to_fk_not_found() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();

        // Delete the version behind the repository's back, so the
        // usecase's existence check cannot catch it. The insert must
        // then fail on the foreign key — and report it as a missing
        // parent (404), not as a repository failure (500).
        sqlx::query("DELETE FROM sdtm_project_versions WHERE id = $1")
            .bind(v.id)
            .execute(&pool)
            .await
            .expect("delete version behind the repository's back");

        let err = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap_err();
        assert!(
            matches!(err, DomainError::FkSdtmProjectVersionNotFound(id) if id == v.id),
            "expected FkSdtmProjectVersionNotFound({}), got {err:?}",
            v.id
        );
    })
    .await
}

// ---- SdtmProjectDomainOverride --------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_override_create_then_get_round_trips() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let overrides = SdtmProjectDomainOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        let o = overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::Events,
                descriptions: Vec::new(),
            })
            .await
            .expect("override create");
        assert_eq!(o.domain_id, d.id);
        assert_eq!(o.category, DomainCategory::Events);

        let got = overrides.find_by_domain_id(d.id).await.expect("get");
        assert_eq!(got.id, o.id);
        assert_eq!(got.category, DomainCategory::Events);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_override_is_unique_per_domain() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let overrides = SdtmProjectDomainOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::Events,
                descriptions: Vec::new(),
            })
            .await
            .unwrap();

        let err = overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::Findings,
                descriptions: Vec::new(),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::DuplicateSdtmProjectDomainOverride { .. }
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_override_update_changes_category() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let overrides = SdtmProjectDomainOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::Events,
                descriptions: Vec::new(),
            })
            .await
            .unwrap();

        let updated = overrides
            .update(SdtmProjectDomainOverrideUpdate {
                domain_id: d.id,
                category: Some(DomainCategory::Findings),
                descriptions: None,
            })
            .await
            .expect("override update");
        assert_eq!(updated.category, DomainCategory::Findings);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_override_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let overrides = SdtmProjectDomainOverrideRepoPg::new(pool);
        let err = overrides.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectDomainOverrideNotFound(999_999)
        ));
    })
    .await
}

/// A domain that has never been overridden reports
/// `SdtmProjectDomainOverrideNotFound` — the exact variant the
/// usecase turns into `Ok(None)`. The repository must keep reporting
/// it as a not-found rather than some other error, or the usecase's
/// "no override" arm would never fire.
#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_override_read_of_a_domain_without_one_is_not_found() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let overrides = SdtmProjectDomainOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        let err = overrides.find_by_domain_id(d.id).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectDomainOverrideNotFound(id) if id == d.id
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_read_carries_the_override() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let overrides = SdtmProjectDomainOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::TrialDesign,
                descriptions: Vec::new(),
            })
            .await
            .unwrap();

        let by_id = domains.find_by_id(d.id).await.expect("find");
        let ov = by_id.domain_override.as_ref().expect("override hydrated");
        assert_eq!(ov.category, DomainCategory::TrialDesign);
        assert_eq!(ov.domain_id, d.id);

        let listed = domains.list_by_version(v.id).await.expect("list");
        let ov = listed[0]
            .domain_override
            .as_ref()
            .expect("override hydrated");
        assert_eq!(ov.category, DomainCategory::TrialDesign);
        assert_eq!(ov.domain_id, d.id);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_update_carries_the_override() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let overrides = SdtmProjectDomainOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::TrialDesign,
                descriptions: Vec::new(),
            })
            .await
            .unwrap();

        // PUT and GET must return the same shape.
        let updated = domains
            .update(SdtmProjectDomainUpdate {
                id: d.id,
                domain_code: Some("AE2".into()),
                key_variables: None,
            })
            .await
            .expect("domain update");
        let ov = updated.domain_override.as_ref().expect("override hydrated");
        assert_eq!(ov.category, DomainCategory::TrialDesign);
        assert_eq!(ov.domain_id, d.id);
    })
    .await
}

// ---- cascade --------------------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn version_delete_cascades_to_the_whole_subtree() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let domain_overrides = SdtmProjectDomainOverrideRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool.clone());
        let variable_overrides = SdtmProjectVariableOverrideRepoPg::new(pool);

        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        domain_overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::Events,
                descriptions: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 1,
            })
            .await
            .unwrap();
        variable_overrides
            .create(SdtmProjectVariableOverrideNew {
                variable_id: var.id,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                descriptions: Vec::new(),
            })
            .await
            .unwrap();

        versions.delete(v.id).await.expect("delete version");

        assert!(domains.find_by_id(d.id).await.is_err());
        assert!(domain_overrides.find_by_domain_id(d.id).await.is_err());
        assert!(variables.find_by_id(var.id).await.is_err());
        assert!(
            variable_overrides
                .find_by_variable_id(var.id)
                .await
                .is_err()
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn domain_delete_cascades_to_override_and_variables() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let domain_overrides = SdtmProjectDomainOverrideRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);

        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        domain_overrides
            .create(SdtmProjectDomainOverrideNew {
                domain_id: d.id,
                category: DomainCategory::Events,
                descriptions: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 1,
            })
            .await
            .unwrap();

        domains.delete(d.id).await.expect("delete domain");

        assert!(domain_overrides.find_by_domain_id(d.id).await.is_err());
        assert!(variables.find_by_id(var.id).await.is_err());
    })
    .await
}

// ---- SdtmProjectVariable --------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_create_then_list_orders_by_sequence() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AESEV".into(),
                variable_sequence: 20,
            })
            .await
            .unwrap();
        variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 10,
            })
            .await
            .unwrap();

        let listed = variables.list_by_domain(d.id).await.expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].variable_code, "AETERM");
        assert_eq!(listed[1].variable_code, "AESEV");
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_update_changes_sequence() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 1,
            })
            .await
            .unwrap();

        let updated = variables
            .update(SdtmProjectVariableUpdate {
                id: var.id,
                variable_code: Some("AETERMX".into()),
                variable_sequence: Some(12),
            })
            .await
            .expect("variable update");
        assert_eq!(updated.variable_code, "AETERMX");
        assert_eq!(updated.variable_sequence, 12);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_update_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let err = variables
            .update(SdtmProjectVariableUpdate {
                id: 999_999,
                variable_code: Some("AETERM".into()),
                variable_sequence: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVariableNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let err = variables.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVariableNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_delete_cascades_to_override() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool.clone());
        let variable_overrides = SdtmProjectVariableOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 1,
            })
            .await
            .unwrap();
        variable_overrides
            .create(SdtmProjectVariableOverrideNew {
                variable_id: var.id,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                descriptions: Vec::new(),
            })
            .await
            .unwrap();

        variables.delete(var.id).await.expect("delete variable");
        assert!(
            variable_overrides
                .find_by_variable_id(var.id)
                .await
                .is_err()
        );
    })
    .await
}

// ---- SdtmProjectVariableOverride ------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_override_create_then_get_round_trips() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool.clone());
        let overrides = SdtmProjectVariableOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 1,
            })
            .await
            .unwrap();

        let o = overrides
            .create(SdtmProjectVariableOverrideNew {
                variable_id: var.id,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                descriptions: Vec::new(),
            })
            .await
            .expect("override create");
        assert_eq!(o.variable_id, var.id);
        assert_eq!(o.variable_role, Some(SdtmRole::Topic));

        let got = overrides.find_by_variable_id(var.id).await.expect("get");
        assert_eq!(got.id, o.id);
        assert_eq!(got.variable_type, SdtmVariableType::Character);
        assert_eq!(got.variable_core, SdtmVariableCore::Req);
        assert_eq!(got.variable_role, Some(SdtmRole::Topic));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_override_role_can_be_cleared() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool.clone());
        let overrides = SdtmProjectVariableOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 1,
            })
            .await
            .unwrap();
        overrides
            .create(SdtmProjectVariableOverrideNew {
                variable_id: var.id,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                descriptions: Vec::new(),
            })
            .await
            .unwrap();

        // Outer Some(None) clears the role.
        let cleared = overrides
            .update(SdtmProjectVariableOverrideUpdate {
                variable_id: var.id,
                variable_type: None,
                variable_core: None,
                variable_role: Some(None),
                descriptions: None,
            })
            .await
            .expect("clear the role");
        assert_eq!(cleared.variable_role, None);

        // Outer None leaves the role unchanged.
        let re_set = overrides
            .update(SdtmProjectVariableOverrideUpdate {
                variable_id: var.id,
                variable_type: None,
                variable_core: None,
                variable_role: Some(Some(SdtmRole::Timing)),
                descriptions: None,
            })
            .await
            .expect("set the role");
        assert_eq!(re_set.variable_role, Some(SdtmRole::Timing));

        let unchanged = overrides
            .update(SdtmProjectVariableOverrideUpdate {
                variable_id: var.id,
                variable_type: None,
                variable_core: None,
                variable_role: None,
                descriptions: None,
            })
            .await
            .expect("leave the role alone");
        assert_eq!(unchanged.variable_role, Some(SdtmRole::Timing));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_override_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let overrides = SdtmProjectVariableOverrideRepoPg::new(pool);
        let err = overrides.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVariableOverrideNotFound(999_999)
        ));
    })
    .await
}

/// The variable-side twin of
/// `domain_override_read_of_a_domain_without_one_is_not_found`.
#[tokio::test]
#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]
async fn variable_override_read_of_a_variable_without_one_is_not_found() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool.clone());
        let overrides = SdtmProjectVariableOverrideRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                version_id: v.id,
                domain_code: "AE".into(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_sequence: 11,
            })
            .await
            .unwrap();

        let err = overrides.find_by_variable_id(var.id).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVariableOverrideNotFound(id) if id == var.id
        ));
    })
    .await
}
