//! PostgreSQL-backed implementation of `CodeItemRepository`,
//! including the `tsvector` / GIN-backed search.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemRepository, CodeItemUpdate, DomainError, Page,
};

use super::tsquery;

const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";
const SQLSTATE_FK_VIOLATION: &str = "23503";

#[derive(FromRow)]
struct CodeItemRow {
    id: i64,
    codelist_id: i64,
    version_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<CodeItemRow> for CodeItem {
    type Error = DomainError;

    fn try_from(row: CodeItemRow) -> Result<Self, Self::Error> {
        Ok(CodeItem::for_repository(
            row.id,
            row.codelist_id,
            row.version_id,
            row.code,
            row.submission_value,
            row.synonym,
            row.definition,
            row.nci_preferred_term,
            row.created_at,
            row.updated_at,
        ))
    }
}

pub struct CodeItemRepo {
    pool: PgPool,
}

impl CodeItemRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CodeItemRepository for CodeItemRepo {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError> {
        let row: CodeItemRow = sqlx::QueryBuilder::new(
            "INSERT INTO code_items \
             (codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term) \
             VALUES (",
        )
        .push_bind(input.codelist_id)
        .push(", ")
        .push_bind(input.version_id)
        .push(", ")
        .push_bind(&input.code)
        .push(", ")
        .push_bind(&input.submission_value)
        .push(", ")
        .push_bind(&input.synonym)
        .push(", ")
        .push_bind(&input.definition)
        .push(", ")
        .push_bind(&input.nci_preferred_term)
        .push(") RETURNING id, codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at")
        .build_query_as::<CodeItemRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(|err| map_db_error(err, Some(input.codelist_id)))?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError> {
        let row: CodeItemRow = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_items WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<CodeItemRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error_simple)?
        .ok_or(DomainError::CodeItemNotFound(id))?;
        row.try_into()
    }

    async fn search_or_list(&self, q: CodeItemListQuery) -> Result<Page<CodeItem>, DomainError> {
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at FROM code_items",
        );
        // `version_id` and `codelist_id` are optional on the domain
        // query: when supplied we restrict to that owning
        // version / codelist (the typical per-version or per-
        // codelist browse path); when omitted we return every
        // code item known to the backend. Build the WHERE clause
        // dynamically so the fragment filter can chain with
        // `AND` only when at least one of them is present.
        let mut has_where = false;
        if let Some(version_id) = q.version_id {
            qb.push(" WHERE version_id = ");
            qb.push_bind(version_id);
            has_where = true;
        }
        if let Some(codelist_id) = q.codelist_id {
            qb.push(if has_where { " AND " } else { " WHERE " });
            qb.push("codelist_id = ");
            qb.push_bind(codelist_id);
            has_where = true;
        }
        let frag_filter = q.fragment.as_deref().filter(|s| !s.trim().is_empty());

        if let Some(expr) = frag_filter.and_then(tsquery::build) {
            qb.push(if has_where { " AND " } else { " WHERE " });
            qb.push("tsv @@ to_tsquery('english', ");
            qb.push_bind(expr.clone());
            qb.push(") ORDER BY ts_rank(tsv, to_tsquery('english', ");
            qb.push_bind(expr);
            qb.push(")) DESC, id ASC LIMIT ");
        } else {
            qb.push(" ORDER BY id ASC LIMIT ");
        }
        qb.push_bind((q.limit as i64) + 1);
        qb.push(" OFFSET ");
        qb.push_bind(q.offset as i64);

        let mut rows: Vec<CodeItemRow> = qb
            .build_query_as::<CodeItemRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error_simple)?;

        let next_offset = if rows.len() as u32 > q.limit {
            rows.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        let items = rows
            .into_iter()
            .map(CodeItem::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, next_offset })
    }

    async fn list_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItem>, DomainError> {
        let rows: Vec<CodeItemRow> = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, version_id, code, submission_value, \
                    synonym, definition, nci_preferred_term, \
                    created_at, updated_at \
             FROM code_items WHERE version_id = ",
        )
        .push_bind(version_id)
        .push(" AND code = ")
        .push_bind(code)
        .push(" ORDER BY id")
        .build_query_as::<CodeItemRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter().map(CodeItem::try_from).collect()
    }

    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE code_items SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref code) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(code);
        }
        if let Some(ref sv) = input.submission_value {
            sep(&mut qb);
            qb.push("submission_value = ").push_bind(sv);
        }
        if let Some(ref syn) = input.synonym {
            sep(&mut qb);
            qb.push("synonym = ").push_bind(syn);
        }
        if let Some(ref def) = input.definition {
            sep(&mut qb);
            qb.push("definition = ").push_bind(def);
        }
        if let Some(ref pt) = input.nci_preferred_term {
            sep(&mut qb);
            qb.push("nci_preferred_term = ").push_bind(pt);
        }
        if first {
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at");
        let row: CodeItemRow = qb
            .build_query_as::<CodeItemRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error_simple)?
            .ok_or(DomainError::CodeItemNotFound(input.id))?;
        row.try_into()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM code_items WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error_simple)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CodeItemNotFound(id));
        }
        Ok(())
    }

    async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError> {
        if inputs.is_empty() {
            return Ok(0);
        }
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO code_items \
             (codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term) ",
        );
        qb.push_values(inputs, |mut b, item| {
            b.push_bind(item.codelist_id)
                .push_bind(item.version_id)
                .push_bind(&item.code)
                .push_bind(&item.submission_value)
                .push_bind(&item.synonym)
                .push_bind(&item.definition)
                .push_bind(&item.nci_preferred_term);
        });
        let result = qb
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error_simple)?;
        Ok(result.rows_affected() as usize)
    }
}

fn map_db_error_simple(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::Repository(format!(
                    "duplicate key violates unique constraint `{constraint}`"
                ))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}

/// `create` mapper: knows about the codelist_id it just inserted
/// with, so SQLSTATE `23503` becomes `FkCodeListNotFound(codelist_id)`.
fn map_db_error(err: sqlx::Error, codelist_id_hint: Option<i64>) -> DomainError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some(SQLSTATE_FK_VIOLATION)
    {
        return DomainError::FkCodeListNotFound(codelist_id_hint.unwrap_or(0));
    }
    map_db_error_simple(err)
}
