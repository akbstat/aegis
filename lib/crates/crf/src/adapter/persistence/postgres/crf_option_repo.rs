use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{CrfOption, CrfOptionNew, CrfOptionRepository, CrfOptionUpdate, DomainError};

#[derive(FromRow)]
struct CrfOptionRow {
    id: i64,
    item_id: i64,
    value: String,
    not_submitted: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CrfOptionRow> for CrfOption {
    fn from(r: CrfOptionRow) -> Self {
        CrfOption::for_repository(
            r.id,
            r.item_id,
            r.value,
            r.not_submitted,
            r.created_at,
            r.updated_at,
        )
    }
}

#[derive(Clone)]
pub struct CrfOptionRepoPg {
    pool: PgPool,
}

impl CrfOptionRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrfOptionRepository for CrfOptionRepoPg {
    async fn create(&self, input: CrfOptionNew) -> Result<CrfOption, DomainError> {
        let row: CrfOptionRow = sqlx::query_as::<_, CrfOptionRow>(
            "INSERT INTO crf_options (item_id, value, not_submitted)
             VALUES ($1, $2, $3)
             RETURNING id, item_id, value, not_submitted, created_at, updated_at",
        )
        .bind(input.item_id)
        .bind(&input.value)
        .bind(input.not_submitted)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<CrfOption, DomainError> {
        let row: CrfOptionRow = sqlx::query_as::<_, CrfOptionRow>(
            "SELECT id, item_id, value, not_submitted, created_at, updated_at
             FROM crf_options WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfOptionNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_item(&self, item_id: i64) -> Result<Vec<CrfOption>, DomainError> {
        let rows = sqlx::query_as::<_, CrfOptionRow>(
            "SELECT id, item_id, value, not_submitted, created_at, updated_at
             FROM crf_options WHERE item_id = $1 ORDER BY id ASC",
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_items(&self, item_ids: &[i64]) -> Result<Vec<CrfOption>, DomainError> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, CrfOptionRow>(
            "SELECT id, item_id, value, not_submitted, created_at, updated_at
             FROM crf_options WHERE item_id = ANY($1) ORDER BY id ASC",
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: CrfOptionUpdate) -> Result<CrfOption, DomainError> {
        let row: CrfOptionRow = sqlx::query_as::<_, CrfOptionRow>(
            "UPDATE crf_options SET
                value         = COALESCE($2, value),
                not_submitted = COALESCE($3, not_submitted)
             WHERE id = $1
             RETURNING id, item_id, value, not_submitted, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.value.as_deref())
        .bind(input.not_submitted)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfOptionNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM crf_options WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CrfOptionNotFound(id));
        }
        Ok(())
    }

    async fn count_by_item(&self, item_id: i64) -> Result<i64, DomainError> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crf_options WHERE item_id = $1")
            .bind(item_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(n)
    }

    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfOption>, DomainError> {
        let pat = format!("%{fragment}%");
        let rows = sqlx::query_as::<_, CrfOptionRow>(
            "SELECT o.id, o.item_id, o.value, o.not_submitted, o.created_at, o.updated_at
             FROM crf_options o
             JOIN crf_items i ON i.id = o.item_id
             JOIN crf_forms f ON f.id = i.form_id
             WHERE f.version_id = $1 AND o.value ILIKE $2
             ORDER BY o.id ASC",
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
            if let Some(c) = db.constraint()
                && c.contains("crf_options_item_id_fkey")
            {
                return DomainError::FkCrfItemNotFound(0);
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
        let sql = include_str!("../../../../migrations/20261103135354_create_crf_options.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS crf_options"));
        assert!(sql.contains("crf_options_set_updated_at"));
    }
}
