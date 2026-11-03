// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
// `sqlx::query_as` + `sqlx::query` + `FromRow` are sufficient and
// keep the crate reproducible.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionRepository,
    SdtmProjectVersionUpdate,
};

#[derive(FromRow)]
struct SdtmProjectVersionRow {
    id: i64,
    project_code: String,
    version_name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SdtmProjectVersionRow> for SdtmProjectVersion {
    fn from(r: SdtmProjectVersionRow) -> Self {
        SdtmProjectVersion::for_repository(
            r.id,
            r.project_code,
            r.version_name,
            r.created_at,
            r.updated_at,
        )
    }
}

#[derive(Clone)]
pub struct SdtmProjectVersionRepoPg {
    pool: PgPool,
}

impl SdtmProjectVersionRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmProjectVersionRepository for SdtmProjectVersionRepoPg {
    async fn create(
        &self,
        input: SdtmProjectVersionNew,
    ) -> Result<SdtmProjectVersion, DomainError> {
        let row: SdtmProjectVersionRow = sqlx::query_as::<_, SdtmProjectVersionRow>(
            "INSERT INTO sdtm_project_versions (project_code, version_name) VALUES ($1, $2)
             RETURNING id, project_code, version_name, created_at, updated_at",
        )
        .bind(&input.project_code)
        .bind(&input.version_name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_version_err)?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVersion, DomainError> {
        // `fetch_optional` + an explicit not-found error: the
        // per-aggregate variant is what reaches the wire, so the
        // client can tell "no such version" from "no such row".
        let row: SdtmProjectVersionRow = sqlx::query_as::<_, SdtmProjectVersionRow>(
            "SELECT id, project_code, version_name, created_at, updated_at
             FROM sdtm_project_versions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_version_err)?
        .ok_or(DomainError::SdtmProjectVersionNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_project(
        &self,
        project_code: &str,
    ) -> Result<Vec<SdtmProjectVersion>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmProjectVersionRow>(
            "SELECT id, project_code, version_name, created_at, updated_at
             FROM sdtm_project_versions
             WHERE project_code = $1
             ORDER BY id ASC",
        )
        .bind(project_code)
        .fetch_all(&self.pool)
        .await
        .map_err(map_version_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(
        &self,
        input: SdtmProjectVersionUpdate,
    ) -> Result<SdtmProjectVersion, DomainError> {
        // Only `version_name` is mutable on a version; `project_code`
        // is immutable so a version cannot be moved between projects.
        // COALESCE: a NULL argument leaves the column unchanged.
        let row: SdtmProjectVersionRow = sqlx::query_as::<_, SdtmProjectVersionRow>(
            "UPDATE sdtm_project_versions SET version_name = COALESCE($2, version_name)
             WHERE id = $1
             RETURNING id, project_code, version_name, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.version_name.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_version_err)?
        .ok_or(DomainError::SdtmProjectVersionNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_project_versions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_version_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmProjectVersionNotFound(id));
        }
        Ok(())
    }
}

fn map_version_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            // Postgres unique-violation code (`23505`) comes back as
            // `E::Database`. The driver does not surface the natural
            // key, so the placeholders below are the best the adapter
            // can do; the usecase's own pre-check is what gives the
            // caller a precise message.
            if db.code().as_deref() == Some("23505") {
                return DomainError::DuplicateSdtmProjectVersion {
                    project_code: "(unknown)".into(),
                    version_name: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                return DomainError::FkSdtmProjectVersionNotFound(0);
            }
            DomainError::Repository(err.to_string())
        }
        E::RowNotFound => DomainError::NotFound,
        _ => DomainError::Repository(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    /// Tier-2 unit test: confirms the migration file referenced by the
    /// adapter is the one we expect (single source of truth, loadable
    /// by `sqlx::migrate!` at app start). It does **not** open a real
    /// connection — see `tests/integration_persistence.rs` for that.
    #[test]
    fn migration_file_declares_version_table() {
        let sql =
            include_str!("../../../../migrations/20261103135411_create_sdtm_project_versions.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_versions"));
        assert!(sql.contains(
            "CONSTRAINT sdtm_project_versions_project_name_unique UNIQUE (project_code, version_name)"
        ));
        assert!(sql.contains("sdtm_project_versions_updated_at"));
    }
}
