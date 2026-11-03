use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{CrfUnit, CrfUnitNew, CrfUnitRepository, CrfUnitUpdate, DomainError};

#[derive(FromRow)]
struct CrfUnitRow {
    id: i64,
    item_id: i64,
    value: String,
    not_submitted: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CrfUnitRow> for CrfUnit {
    fn from(r: CrfUnitRow) -> Self {
        CrfUnit::for_repository(
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
pub struct CrfUnitRepoPg {
    pool: PgPool,
}

impl CrfUnitRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrfUnitRepository for CrfUnitRepoPg {
    async fn create(&self, input: CrfUnitNew) -> Result<CrfUnit, DomainError> {
        let row: CrfUnitRow = sqlx::query_as::<_, CrfUnitRow>(
            "INSERT INTO crf_units (item_id, value, not_submitted)
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

    async fn find_by_id(&self, id: i64) -> Result<CrfUnit, DomainError> {
        let row: CrfUnitRow = sqlx::query_as::<_, CrfUnitRow>(
            "SELECT id, item_id, value, not_submitted, created_at, updated_at
             FROM crf_units WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfUnitNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_item(&self, item_id: i64) -> Result<Vec<CrfUnit>, DomainError> {
        let rows = sqlx::query_as::<_, CrfUnitRow>(
            "SELECT id, item_id, value, not_submitted, created_at, updated_at
             FROM crf_units WHERE item_id = $1 ORDER BY id ASC",
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_items(&self, item_ids: &[i64]) -> Result<Vec<CrfUnit>, DomainError> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, CrfUnitRow>(
            "SELECT id, item_id, value, not_submitted, created_at, updated_at
             FROM crf_units WHERE item_id = ANY($1) ORDER BY id ASC",
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: CrfUnitUpdate) -> Result<CrfUnit, DomainError> {
        let row: CrfUnitRow = sqlx::query_as::<_, CrfUnitRow>(
            "UPDATE crf_units SET
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
        .ok_or(DomainError::CrfUnitNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM crf_units WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CrfUnitNotFound(id));
        }
        Ok(())
    }

    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfUnit>, DomainError> {
        let pat = format!("%{fragment}%");
        let rows = sqlx::query_as::<_, CrfUnitRow>(
            "SELECT u.id, u.item_id, u.value, u.not_submitted, u.created_at, u.updated_at
             FROM crf_units u
             JOIN crf_items i ON i.id = u.item_id
             JOIN crf_forms f ON f.id = i.form_id
             WHERE f.version_id = $1 AND u.value ILIKE $2
             ORDER BY u.id ASC",
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
                && c.contains("crf_units_item_id_fkey")
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
        let sql = include_str!("../../../../migrations/20261103135355_create_crf_units.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS crf_units"));
        assert!(sql.contains("crf_units_set_updated_at"));
    }
}
