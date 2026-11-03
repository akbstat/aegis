use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    CrfItem, CrfItemKind, CrfItemNew, CrfItemRepository, CrfItemUpdate, DomainError,
};

#[derive(FromRow)]
struct CrfItemRow {
    id: i64,
    form_id: i64,
    code: String,
    name: String,
    kind: String,
    order: i32,
    not_submitted: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CrfItemRow> for CrfItem {
    fn from(r: CrfItemRow) -> Self {
        CrfItem::for_repository(
            r.id,
            r.form_id,
            r.code,
            r.name,
            CrfItemKind::try_from_str(&r.kind).expect("CHECK constraint"),
            r.order,
            r.not_submitted,
            r.created_at,
            r.updated_at,
        )
    }
}

#[derive(Clone)]
pub struct CrfItemRepoPg {
    pool: PgPool,
}

impl CrfItemRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrfItemRepository for CrfItemRepoPg {
    async fn create(&self, input: CrfItemNew) -> Result<CrfItem, DomainError> {
        let row: CrfItemRow = sqlx::query_as::<_, CrfItemRow>(
            "INSERT INTO crf_items (form_id, code, name, kind, \"order\", not_submitted)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, form_id, code, name, kind, \"order\", not_submitted, created_at, updated_at",
        )
        .bind(input.form_id)
        .bind(&input.code)
        .bind(&input.name)
        .bind(input.kind.as_str())
        .bind(input.order)
        .bind(input.not_submitted)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<CrfItem, DomainError> {
        let row: CrfItemRow = sqlx::query_as::<_, CrfItemRow>(
            "SELECT id, form_id, code, name, kind, \"order\", not_submitted, created_at, updated_at
             FROM crf_items WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfItemNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_form(&self, form_id: i64) -> Result<Vec<CrfItem>, DomainError> {
        let rows = sqlx::query_as::<_, CrfItemRow>(
            "SELECT id, form_id, code, name, kind, \"order\", not_submitted, created_at, updated_at
             FROM crf_items WHERE form_id = $1
             ORDER BY \"order\" ASC, id ASC",
        )
        .bind(form_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: CrfItemUpdate) -> Result<CrfItem, DomainError> {
        let row: CrfItemRow = sqlx::query_as::<_, CrfItemRow>(
            "UPDATE crf_items SET
                code          = COALESCE($2, code),
                name          = COALESCE($3, name),
                \"order\"       = COALESCE($4, \"order\"),
                not_submitted = COALESCE($5, not_submitted)
             WHERE id = $1
             RETURNING id, form_id, code, name, kind, \"order\", not_submitted, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.code.as_deref())
        .bind(input.name.as_deref())
        .bind(input.order)
        .bind(input.not_submitted)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::CrfItemNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM crf_items WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CrfItemNotFound(id));
        }
        Ok(())
    }

    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfItem>, DomainError> {
        let pat = format!("%{fragment}%");
        let rows = sqlx::query_as::<_, CrfItemRow>(
            "SELECT i.id, i.form_id, i.code, i.name, i.kind, i.\"order\", i.not_submitted, i.created_at, i.updated_at
             FROM crf_items i
             JOIN crf_forms f ON f.id = i.form_id
             WHERE f.version_id = $1 AND (i.code ILIKE $2 OR i.name ILIKE $2)
             ORDER BY i.\"order\" ASC, i.id ASC",
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
                return DomainError::DuplicateCrfItem {
                    form_id: 0,
                    code: "(unknown)".into(),
                };
            }
            if let Some(c) = db.constraint() {
                if c.contains("crf_items_form_code") {
                    return DomainError::DuplicateCrfItem {
                        form_id: 0,
                        code: "(unknown)".into(),
                    };
                }
                if c.contains("crf_items_form_id_fkey") {
                    return DomainError::FkCrfFormNotFound(0);
                }
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
        let sql = include_str!("../../../../migrations/20261103135353_create_crf_items.sql");
        assert!(sql.contains("crf_items_kind_check"));
        assert!(sql.contains("'Selection'"));
    }
}
