use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{CrfForm, CrfFormNew, CrfFormRepository, CrfFormUpdate, DomainError};

#[derive(FromRow)]
struct CrfFormRow {
    id: i64,
    version_id: i64,
    code: String,
    name: String,
    order: i32,
    not_submitted: bool,
    approved: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CrfFormRow> for CrfForm {
    fn from(r: CrfFormRow) -> Self {
        CrfForm::for_repository(
            r.id,
            r.version_id,
            r.code,
            r.name,
            r.order,
            r.not_submitted,
            r.approved,
            r.created_at,
            r.updated_at,
        )
    }
}

#[derive(Clone)]
pub struct CrfFormRepoPg {
    pool: PgPool,
}

impl CrfFormRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrfFormRepository for CrfFormRepoPg {
    async fn create(&self, input: CrfFormNew) -> Result<CrfForm, DomainError> {
        let row: CrfFormRow = sqlx::query_as::<_, CrfFormRow>(
            "INSERT INTO crf_forms (version_id, code, name, \"order\", not_submitted, approved)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at",
        )
        .bind(input.version_id)
        .bind(&input.code)
        .bind(&input.name)
        .bind(input.order)
        .bind(input.not_submitted)
        .bind(input.approved)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<CrfForm, DomainError> {
        let row: CrfFormRow = sqlx::query_as::<_, CrfFormRow>(
            "SELECT id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at
             FROM crf_forms WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfFormNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CrfForm>, DomainError> {
        let rows = sqlx::query_as::<_, CrfFormRow>(
            "SELECT id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at
             FROM crf_forms WHERE version_id = $1
             ORDER BY \"order\" ASC, id ASC",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: CrfFormUpdate) -> Result<CrfForm, DomainError> {
        let row: CrfFormRow = sqlx::query_as::<_, CrfFormRow>(
            "UPDATE crf_forms SET
                code          = COALESCE($2, code),
                name          = COALESCE($3, name),
                \"order\"       = COALESCE($4, \"order\"),
                not_submitted = COALESCE($5, not_submitted)
             WHERE id = $1
             RETURNING id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.code.as_deref())
        .bind(input.name.as_deref())
        .bind(input.order)
        .bind(input.not_submitted)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfFormNotFound(input.id))?;
        Ok(row.into())
    }

    async fn set_approved(&self, id: i64, approved: bool) -> Result<CrfForm, DomainError> {
        let row: CrfFormRow = sqlx::query_as::<_, CrfFormRow>(
            "UPDATE crf_forms SET approved = $2
             WHERE id = $1
             RETURNING id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at",
        )
        .bind(id)
        .bind(approved)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfFormNotFound(id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM crf_forms WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CrfFormNotFound(id));
        }
        Ok(())
    }

    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfForm>, DomainError> {
        let pat = format!("%{fragment}%");
        let rows = sqlx::query_as::<_, CrfFormRow>(
            "SELECT id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at
             FROM crf_forms
             WHERE version_id = $1 AND (code ILIKE $2 OR name ILIKE $2)
             ORDER BY \"order\" ASC, id ASC",
        )
        .bind(version_id)
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
                return DomainError::DuplicateCrfForm {
                    version_id: 0,
                    code: "(unknown)".into(),
                };
            }
            if let Some(c) = db.constraint()
                && c.contains("crf_forms_version_code")
            {
                return DomainError::DuplicateCrfForm {
                    version_id: 0,
                    code: "(unknown)".into(),
                };
            }
            // FK violation on version_id → version missing
            if let Some(c) = db.constraint()
                && c.contains("crf_forms_version_id_fkey")
            {
                return DomainError::FkCrfVersionNotFound(0);
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
        let sql = include_str!("../../../../migrations/20261103135352_create_crf_forms.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS crf_forms"));
        assert!(sql.contains("crf_forms_version_code_unique"));
        assert!(sql.contains("crf_forms_set_updated_at"));
    }
}
