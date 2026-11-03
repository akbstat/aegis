//! Tests for the PostgreSQL adapter that do NOT require a live database
//! connection.
//!
//! 1. The two migration files (the schema that downstream consumers
//!    will apply). The leading doc comments are stripped before
//!    assertion so the tests anchor on the CREATE TABLE block rather
//!    than keywords in the header.
//! 2. The `CredentialRow` -> `UserCredentials` and
//!    `DomainIdentityRow` -> `DomainIdentity` conversions.

use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};

use crate::domain::{DomainError, DomainIdentity, UserCredentials};

use super::row::{CredentialRow, DomainIdentityRow};

fn row_test_timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap()
}

fn migration_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("migrations").join(name)
}

fn load_migration(name: &str) -> String {
    fs::read_to_string(migration_path(name))
        .unwrap_or_else(|_| panic!("migration file lib/crates/auth/migrations/{name} must exist"))
}

fn create_table_block(sql: &str) -> String {
    let start = sql
        .find("CREATE TABLE")
        .expect("migration must contain a CREATE TABLE statement");
    let close = sql[start..]
        .find(");")
        .expect("CREATE TABLE body must be terminated by `);`");
    sql[start..start + close + 2].to_string()
}

#[test]
fn migration_create_auth_user_credentials_table() {
    let sql = load_migration("20261103135349_create_auth_user_credentials.sql");
    let block = create_table_block(&sql);
    assert!(
        block.contains("CREATE TABLE") && block.contains("auth_user_credentials"),
        "expected auth_user_credentials table; got:\n{block}"
    );
}

#[test]
fn migration_auth_user_credentials_has_required_columns() {
    let block = create_table_block(&load_migration(
        "20261103135349_create_auth_user_credentials.sql",
    ));
    let upper = block.to_uppercase();
    for required in ["CODE TEXT", "PASSWORD_HASH TEXT", "TOKEN_VERSION INTEGER"] {
        assert!(
            upper.contains(required),
            "auth_user_credentials must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn migration_auth_user_credentials_password_hash_is_not_null_and_checked() {
    let block = create_table_block(&load_migration(
        "20261103135349_create_auth_user_credentials.sql",
    ));
    let upper = block.to_uppercase();
    assert!(upper.contains("PASSWORD_HASH TEXT NOT NULL"));
    assert!(upper.contains("CHECK"));
    assert!(upper.contains("LENGTH(PASSWORD_HASH) > 0"));
}

#[test]
fn migration_auth_user_credentials_token_version_defaults_to_one() {
    let block = create_table_block(&load_migration(
        "20261103135349_create_auth_user_credentials.sql",
    ));
    let upper = block.to_uppercase();
    assert!(
        upper.contains("TOKEN_VERSION INTEGER NOT NULL DEFAULT 1"),
        "token_version must default to 1; got:\n{block}"
    );
}

#[test]
fn migration_auth_user_credentials_has_updated_at_trigger() {
    let sql = load_migration("20261103135349_create_auth_user_credentials.sql");
    assert!(sql.contains("CREATE TRIGGER auth_user_credentials_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON auth_user_credentials"));
    assert!(sql.contains("CREATE OR REPLACE FUNCTION auth_user_credentials_set_updated_at"));
}

#[test]
fn migration_create_auth_user_domain_identities_table() {
    let sql = load_migration("20261103135350_create_auth_user_domain_identities.sql");
    let block = create_table_block(&sql);
    assert!(
        block.contains("CREATE TABLE") && block.contains("auth_user_domain_identities"),
        "expected auth_user_domain_identities table; got:\n{block}"
    );
}

#[test]
fn migration_auth_user_domain_identities_has_required_columns() {
    let block = create_table_block(&load_migration(
        "20261103135350_create_auth_user_domain_identities.sql",
    ));
    let upper = block.to_uppercase();
    for required in [
        "USER_CODE TEXT",
        "DOMAIN_NAME TEXT",
        "HOSTNAME TEXT",
        "SID TEXT",
    ] {
        assert!(
            upper.contains(required),
            "auth_user_domain_identities must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn migration_auth_user_domain_identities_unique_constraint_covers_all_four_columns() {
    let block = create_table_block(&load_migration(
        "20261103135350_create_auth_user_domain_identities.sql",
    ));
    assert!(
        block.contains("UNIQUE (user_code, domain_name, hostname, sid)"),
        "unique constraint must cover all four columns; got:\n{block}"
    );
}

#[test]
fn credential_row_converts_to_user_credentials() {
    let row = CredentialRow {
        code: "u1".into(),
        password_hash: "hash".into(),
        token_version: 7,
        created_at: row_test_timestamp(),
        updated_at: row_test_timestamp(),
    };
    let creds: UserCredentials = row.try_into().expect("convert succeeds");
    assert_eq!(creds.code, "u1");
    assert_eq!(creds.password_hash, "hash");
    assert_eq!(creds.token_version, 7);
}

#[test]
fn credential_row_with_negative_token_version_is_rejected() {
    let row = CredentialRow {
        code: "u1".into(),
        password_hash: "hash".into(),
        token_version: -1,
        created_at: row_test_timestamp(),
        updated_at: row_test_timestamp(),
    };
    let err = UserCredentials::try_from(row).expect_err("negative rejected");
    assert!(matches!(err, DomainError::Repository(_)));
}

#[test]
fn domain_identity_row_converts_to_domain_identity() {
    let row = DomainIdentityRow {
        user_code: "u1".into(),
        domain_name: "DOM".into(),
        hostname: "host".into(),
        sid: "S-1-5".into(),
    };
    let id: DomainIdentity = row.try_into().expect("convert succeeds");
    assert_eq!(id.user_code, "u1");
    assert_eq!(id.domain_name, "DOM");
    assert_eq!(id.hostname, "host");
    assert_eq!(id.sid, "S-1-5");
}
