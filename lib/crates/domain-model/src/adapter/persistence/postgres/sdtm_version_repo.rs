// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
// `sqlx::query_as` + `sqlx::query` + `FromRow` + `QueryBuilder`
// are sufficient and keep the crate reproducible.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmVersion, SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
};

#[derive(FromRow)]
struct SdtmVersionRow {
    id: i64,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SdtmVersionRow> for SdtmVersion {
    fn from(r: SdtmVersionRow) -> Self {
        SdtmVersion::for_repository(r.id, r.name, r.created_at, r.updated_at)
    }
}

#[derive(Clone)]
pub struct SdtmVersionRepoPg {
    pool: PgPool,
}

impl SdtmVersionRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmVersionRepository for SdtmVersionRepoPg {
    async fn create(&self, input: SdtmVersionNew) -> Result<SdtmVersion, DomainError> {
        let row: SdtmVersionRow = sqlx::query_as::<_, SdtmVersionRow>(
            "INSERT INTO sdtm_versions (name) VALUES ($1)
             RETURNING id, name, created_at, updated_at",
        )
        .bind(&input.name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.into())
    }

    async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmVersionRow>(
            "SELECT id, name, created_at, updated_at
             FROM sdtm_versions ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: SdtmVersionUpdate) -> Result<SdtmVersion, DomainError> {
        // Spec: only `name` is mutable on a version. The
        // UPDATE … RETURNING path materialises the resulting
        // row; if the row doesn't exist we surface
        // `SdtmVersionNotFound`.
        let row: SdtmVersionRow = sqlx::query_as::<_, SdtmVersionRow>(
            "UPDATE sdtm_versions SET name = COALESCE($2, name)
             WHERE id = $1
             RETURNING id, name, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.name.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::SdtmVersionNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_versions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmVersionNotFound(id));
        }
        Ok(())
    }
}

fn map_db_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            // Postgres unique-violation codes (`23505`) come back as
            // `E::Database` with the column name on the constraint.
            if db.code().as_deref() == Some("23505") {
                return DomainError::DuplicateSdtmVersion {
                    name: "(unknown)".into(),
                };
            }
            DomainError::Repository(err.to_string())
        }
        E::RowNotFound => DomainError::NotFound,
        _ => DomainError::Repository(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    /// Tier-2 unit test: confirms the migration file referenced by
    /// the adapter is the one we expect (single source of truth,
    /// loadable by `sqlx::migrate!` at app start). It does **not**
    /// open a real connection — see `tests/integration_persistence.rs`
    /// for that.
    #[test]
    fn migration_file_is_present_and_idempotent() {
        let sql = include_str!("../../../../migrations/20261103135400_create_sdtm_versions.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_versions"));
        assert!(sql.contains("sdtm_versions_updated_at"));
    }
}
