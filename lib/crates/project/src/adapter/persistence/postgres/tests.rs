//! Schema + row-conversion tests for the PostgreSQL adapter.
//!
//! These tests do NOT require a live database. They read the migration
//! file and the row-bridge impls directly. Live-database round-trips
//! live in `tests/integration_persistence.rs` and are `#[ignore]`-gated.

use std::fs;
use std::path::PathBuf;

fn migration_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("migrations").join(name)
}

fn load_migration(name: &str) -> String {
    fs::read_to_string(migration_path(name))
        .unwrap_or_else(|_| panic!("migration file {name} must exist"))
}

fn create_table_block(sql: &str) -> String {
    let start = sql.find("CREATE TABLE").expect("CREATE TABLE");
    let close = sql[start..]
        .find(");")
        .expect("CREATE TABLE terminated by `);`");
    sql[start..start + close + 2].to_string()
}

#[test]
fn projects_migration_creates_projects_table() {
    let sql = load_migration("20261103135407_create_projects.sql");
    let block = create_table_block(&sql);
    assert!(block.contains("CREATE TABLE") && block.contains("projects"));
}

#[test]
fn projects_migration_has_required_columns() {
    let block = create_table_block(&load_migration("20261103135407_create_projects.sql"));
    let upper = block.to_uppercase();
    for required in [
        "ID INTEGER",
        "CODE TEXT",
        "DESCRIPTION TEXT",
        "ACTIVE BOOLEAN",
        "CREATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "UPDATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
    ] {
        assert!(
            upper.contains(&required.to_uppercase()),
            "projects table must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn projects_migration_has_updated_at_trigger() {
    let sql = load_migration("20261103135407_create_projects.sql");
    assert!(sql.contains("CREATE TRIGGER projects_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON projects"));
}

#[test]
fn projects_migration_makes_code_unique() {
    let block = create_table_block(&load_migration("20261103135407_create_projects.sql"));
    assert!(
        block.contains("UNIQUE (code)") || block.contains("UNIQUE(\"code\")"),
        "expected UNIQUE on code; got:\n{block}"
    );
}

#[test]
fn projects_migration_no_longer_has_product_id() {
    let sql = load_migration("20261103135407_create_projects.sql");
    assert!(
        !sql.contains("product_id"),
        "projects table must not reference product_id; got:\n{sql}"
    );
}

#[test]
fn projects_migration_no_longer_has_tags_column() {
    // The legacy `tags` JSONB column is dropped in 0002; the
    // CREATE TABLE block in 0001 still declares it (that block
    // remains so the table can be built fresh in environments
    // without 0002 applied). This test confirms 0002 has actually
    // dropped the column.
    let sql = load_migration("20261103135408_add_project_configuration.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("DROP COLUMN TAGS"),
        "the 0002 migration must drop the legacy tags column; got:\n{sql}"
    );
}

#[test]
fn configuration_migration_has_configuration_object_check() {
    let sql = load_migration("20261103135408_add_project_configuration.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("JSONB_TYPEOF(CONFIGURATION) = 'OBJECT'"),
        "the 0002 migration must enforce jsonb_typeof(configuration) = 'object'; got:\n{sql}"
    );
}

#[test]
fn configuration_migration_drops_legacy_tags() {
    let sql = load_migration("20261103135408_add_project_configuration.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("DROP COLUMN TAGS"),
        "the 0002 migration must drop the legacy tags column"
    );
    // PostgreSQL drops the `projects_tags_is_array` CHECK
    // automatically when its `tags` column is dropped; the explicit
    // `DROP CONSTRAINT` is unnecessary and was deliberately omitted
    // from the migration to keep the SQL compact.
}

#[test]
fn project_members_migration_has_composite_pk_and_checks() {
    let sql = load_migration("20261103135407_create_projects.sql");
    let upper = sql.to_uppercase();
    let start = upper
        .find("CREATE TABLE PROJECT_MEMBERS")
        .expect("project_members");
    let close = upper[start..].find(");").expect("close") + start + 2;
    let block = &sql[start..close];
    let upper_block = block.to_uppercase();
    assert!(
        upper_block.contains("PRIMARY KEY (PROJECT_ID, TEAM_TYPE, ROLE_TYPE, USER_CODE)"),
        "project_members PK must be the composite; got:\n{block}"
    );
    assert!(upper_block.contains("CHECK"));
    assert!(upper_block.contains("'MEMBERS'") && upper_block.contains("'UNBLIND_MEMBERS'"));
    assert!(upper_block.contains("'LEADER'") && upper_block.contains("'WORKER'"));
}

#[test]
fn project_members_migration_cascades_on_delete() {
    let sql = load_migration("20261103135407_create_projects.sql");
    assert!(
        sql.contains("REFERENCES projects(id) ON DELETE CASCADE"),
        "project_members FK must cascade on delete"
    );
}

#[test]
fn sdtmig_migration_adds_sdtmig_key_to_configuration() {
    let sql = load_migration("20261103135409_add_project_configuration_sdtmig.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("SDTMIG"),
        "the 0003 migration must add the sdtmig key to the configuration JSONB; got:\n{sql}"
    );
    assert!(
        upper.contains("CONFIGURATION = CONFIGURATION ||"),
        "the 0003 migration must merge the sdtmig key into existing rows; got:\n{sql}"
    );
}

#[test]
fn sdtmig_migration_updates_default_to_include_sdtmig_null() {
    let sql = load_migration("20261103135409_add_project_configuration_sdtmig.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("DEFAULT"),
        "the 0003 migration must update the column default; got:\n{sql}"
    );
    assert!(
        upper.contains("SDTMIG"),
        "the column default must include sdtmig; got:\n{sql}"
    );
}

#[test]
fn sdtm_terminology_migration_adds_sdtm_terminology_key_to_configuration() {
    let sql = load_migration("20261103135410_add_project_configuration_sdtm_terminology.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("SDTM_TERMINOLOGY"),
        "the 0004 migration must add the sdtm_terminology key to the configuration JSONB; got:\n{sql}"
    );
    assert!(
        upper.contains("CONFIGURATION = CONFIGURATION ||"),
        "the 0004 migration must merge the sdtm_terminology key into existing rows; got:\n{sql}"
    );
}

#[test]
fn sdtm_terminology_migration_updates_default_to_include_sdtm_terminology_null() {
    let sql = load_migration("20261103135410_add_project_configuration_sdtm_terminology.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("DEFAULT"),
        "the 0004 migration must update the column default; got:\n{sql}"
    );
    assert!(
        upper.contains("SDTM_TERMINOLOGY"),
        "the column default must include sdtm_terminology; got:\n{sql}"
    );
}

#[cfg(test)]
mod row_tests {
    use chrono::{TimeZone, Utc};

    use super::super::row::{ProjectMemberRow, ProjectRow};
    use crate::domain::{
        ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectTag, RoleType, TeamType,
    };

    fn ts() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
    }

    #[test]
    fn project_row_converts_to_project_with_empty_configuration() {
        let row = ProjectRow {
            id: 1,
            code: "proj1".into(),
            description: "".into(),
            active: true,
            configuration: sqlx::types::Json(ProjectConfiguration::default()),
            created_at: ts(),
            updated_at: ts(),
        };
        let p: crate::domain::Project = row.try_into().expect("convert");
        assert_eq!(p.id, 1);
        assert_eq!(p.members, ProjectMember::default());
        assert_eq!(p.unblind_members, ProjectMember::default());
        assert!(p.configurations.tags.is_empty());
        assert!(p.configurations.language.is_none());
    }

    #[test]
    fn project_row_converts_to_project_with_configuration() {
        let row = ProjectRow {
            id: 1,
            code: "proj1".into(),
            description: "".into(),
            active: true,
            configuration: sqlx::types::Json(ProjectConfiguration::for_repository(
                Some(ProjectLanguage::English),
                vec![
                    ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                    ProjectTag::for_repository("Region".into(), "EU".into()),
                ],
                None,
                None,
            )),
            created_at: ts(),
            updated_at: ts(),
        };
        let p: crate::domain::Project = row.try_into().expect("convert");
        assert_eq!(p.configurations.language, Some(ProjectLanguage::English));
        assert_eq!(p.configurations.tags.len(), 2);
        assert_eq!(p.configurations.tags[0].key, "Product");
        assert_eq!(p.configurations.tags[1].value, "EU");
    }

    #[test]
    fn project_member_row_carries_team_and_role_strings() {
        let row = ProjectMemberRow {
            project_id: 1,
            team_type: "members".into(),
            role_type: "leader".into(),
            user_code: "u1".into(),
        };
        assert_eq!(
            TeamType::try_from(row.team_type.as_str()).unwrap(),
            TeamType::Members
        );
        assert_eq!(
            RoleType::try_from(row.role_type.as_str()).unwrap(),
            RoleType::Leader
        );
    }
}
