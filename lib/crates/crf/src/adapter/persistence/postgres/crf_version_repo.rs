use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    CrfVersion, CrfVersionNew, CrfVersionRepository, CrfVersionUpdate, DomainError,
};

#[derive(FromRow)]
struct CrfVersionRow {
    id: i64,
    project_code: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CrfVersionRow> for CrfVersion {
    fn from(r: CrfVersionRow) -> Self {
        CrfVersion::for_repository(r.id, r.project_code, r.name, r.created_at, r.updated_at)
    }
}

#[derive(Clone)]
pub struct CrfVersionRepoPg {
    pool: PgPool,
}

impl CrfVersionRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrfVersionRepository for CrfVersionRepoPg {
    async fn create(&self, input: CrfVersionNew) -> Result<CrfVersion, DomainError> {
        let row: CrfVersionRow = sqlx::query_as::<_, CrfVersionRow>(
            "INSERT INTO crf_versions (project_code, name) VALUES ($1, $2)
             RETURNING id, project_code, name, created_at, updated_at",
        )
        .bind(&input.project_code)
        .bind(&input.name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<CrfVersion, DomainError> {
        let row: CrfVersionRow = sqlx::query_as::<_, CrfVersionRow>(
            "SELECT id, project_code, name, created_at, updated_at
             FROM crf_versions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfVersionNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_project(&self, project_code: &str) -> Result<Vec<CrfVersion>, DomainError> {
        let rows = sqlx::query_as::<_, CrfVersionRow>(
            "SELECT id, project_code, name, created_at, updated_at
             FROM crf_versions WHERE project_code = $1 ORDER BY id ASC",
        )
        .bind(project_code)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: CrfVersionUpdate) -> Result<CrfVersion, DomainError> {
        let row: CrfVersionRow = sqlx::query_as::<_, CrfVersionRow>(
            "UPDATE crf_versions SET name = COALESCE($2, name)
             WHERE id = $1
             RETURNING id, project_code, name, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.name.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfVersionNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM crf_versions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CrfVersionNotFound(id));
        }
        Ok(())
    }

    async fn search_by_version(
        &self,
        project_code: &str,
        fragment: &str,
    ) -> Result<Vec<CrfVersion>, DomainError> {
        let pat = format!("%{fragment}%");
        let rows = sqlx::query_as::<_, CrfVersionRow>(
            "SELECT id, project_code, name, created_at, updated_at
             FROM crf_versions
             WHERE project_code = $1 AND (name ILIKE $2)
             ORDER BY id ASC",
        )
        .bind(project_code)
        .bind(&pat)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

fn map_db_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                return DomainError::DuplicateCrfVersion {
                    project_code: "(unknown)".into(),
                    name: "(unknown)".into(),
                };
            }
            if let Some(c) = db.constraint()
                && c.contains("crf_versions_project_name")
            {
                return DomainError::DuplicateCrfVersion {
                    project_code: "(unknown)".into(),
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
    #[test]
    fn migration_file_is_present_and_idempotent() {
        let sql = include_str!("../../../../migrations/20261103135351_create_crf_versions.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS crf_versions"));
        assert!(sql.contains("crf_versions_project_name_unique"));
        assert!(sql.contains("crf_versions_set_updated_at"));
    }
}
