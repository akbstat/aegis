//! Transactional Postgres implementation of [`CrfBulkFormRepository`].
//!
//! Opens a single `pool.begin()` transaction, walks the bulk input
//! (form → items → options / units), and commits at the end. Any
//! `Err` returned by sqlx drops the in-flight transaction, rolling
//! back every insert — true all-or-nothing semantics.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    CrfBulkCreateForm, CrfBulkCreateFormResult, CrfBulkFormRepository, CrfForm, CrfItem,
    CrfItemKind, DomainError,
};

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
        let kind = CrfItemKind::try_from_str(&r.kind)
            .expect("DB CHECK constraint guarantees a valid kind");
        CrfItem::for_repository(
            r.id,
            r.form_id,
            r.code,
            r.name,
            kind,
            r.order,
            r.not_submitted,
            r.created_at,
            r.updated_at,
        )
    }
}

#[derive(Clone)]
pub struct CrfBulkFormRepoPg {
    pool: PgPool,
}

impl CrfBulkFormRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrfBulkFormRepository for CrfBulkFormRepoPg {
    async fn bulk_create(
        &self,
        input: CrfBulkCreateForm,
    ) -> Result<CrfBulkCreateFormResult, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        // 1. Insert the form. RETURNING gives us the freshly
        //    stamped surrogate id so child rows can bind to it.
        let form_row: CrfFormRow = sqlx::query_as::<_, CrfFormRow>(
            "INSERT INTO crf_forms (version_id, code, name, \"order\", not_submitted, approved)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at",
        )
        .bind(input.form.version_id)
        .bind(&input.form.code)
        .bind(&input.form.name)
        .bind(input.form.order)
        .bind(input.form.not_submitted)
        .bind(input.form.approved)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_err)?;
        let form: CrfForm = form_row.into();

        // 2. Insert each item, then its options + units.
        let mut inserted_items: Vec<CrfItem> = Vec::with_capacity(input.items.len());
        for bi in input.items {
            let item_row: CrfItemRow = sqlx::query_as::<_, CrfItemRow>(
                "INSERT INTO crf_items (form_id, code, name, kind, \"order\", not_submitted)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING id, form_id, code, name, kind, \"order\", not_submitted, created_at, updated_at",
            )
            .bind(form.id)
            .bind(&bi.item.code)
            .bind(&bi.item.name)
            .bind(bi.item.kind.as_str())
            .bind(bi.item.order)
            .bind(bi.item.not_submitted)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_err)?;
            let item: CrfItem = item_row.into();

            for opt in bi.options {
                sqlx::query(
                    "INSERT INTO crf_options (item_id, value, not_submitted)
                     VALUES ($1, $2, $3)",
                )
                .bind(item.id)
                .bind(&opt.value)
                .bind(opt.not_submitted)
                .execute(&mut *tx)
                .await
                .map_err(map_db_err)?;
            }
            for u in bi.units {
                sqlx::query(
                    "INSERT INTO crf_units (item_id, value, not_submitted)
                     VALUES ($1, $2, $3)",
                )
                .bind(item.id)
                .bind(&u.value)
                .bind(u.not_submitted)
                .execute(&mut *tx)
                .await
                .map_err(map_db_err)?;
            }
            inserted_items.push(item);
        }

        // 3. Commit. If we got here without an `Err`, every row
        //    is now durable; the `tx` would otherwise drop and
        //    roll back on any earlier error path.
        tx.commit().await.map_err(map_db_err)?;
        Ok(CrfBulkCreateFormResult {
            form,
            items: inserted_items,
        })
    }
}

/// Map sqlx errors to `DomainError`. Mirrors the four existing
/// per-aggregate `map_db_err` functions: 23505 on a unique
/// constraint surfaces as the matching `DuplicateCrf*`; FK
/// violations on `version_id` / `form_id` / `item_id` surface as
/// the matching `Fk*NotFound`; everything else is a
/// `Repository`.
fn map_db_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                if let Some(c) = db.constraint() {
                    if c.contains("crf_forms_version_code") {
                        return DomainError::DuplicateCrfForm {
                            version_id: 0,
                            code: "(unknown)".into(),
                        };
                    }
                    if c.contains("crf_items_form_code") {
                        return DomainError::DuplicateCrfItem {
                            form_id: 0,
                            code: "(unknown)".into(),
                        };
                    }
                }
                // Fall back: any other unique violation gets the
                // generic form.
                return DomainError::DuplicateCrfForm {
                    version_id: 0,
                    code: "(unknown)".into(),
                };
            }
            if let Some(c) = db.constraint() {
                if c.contains("crf_forms_version_id_fkey") {
                    return DomainError::FkCrfVersionNotFound(0);
                }
                if c.contains("crf_items_form_id_fkey") {
                    return DomainError::FkCrfFormNotFound(0);
                }
                if c.contains("crf_options_item_id_fkey") {
                    return DomainError::FkCrfItemNotFound(0);
                }
                if c.contains("crf_units_item_id_fkey") {
                    return DomainError::FkCrfItemNotFound(0);
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
    fn migration_files_are_present_and_idempotent() {
        let forms_sql = include_str!("../../../../migrations/20261103135352_create_crf_forms.sql");
        assert!(forms_sql.contains("CREATE TABLE IF NOT EXISTS crf_forms"));
        assert!(forms_sql.contains("crf_forms_version_code_unique"));
        let items_sql = include_str!("../../../../migrations/20261103135353_create_crf_items.sql");
        assert!(items_sql.contains("CREATE TABLE IF NOT EXISTS crf_items"));
        assert!(items_sql.contains("crf_items_form_code_unique"));
        let options_sql =
            include_str!("../../../../migrations/20261103135354_create_crf_options.sql");
        assert!(options_sql.contains("CREATE TABLE IF NOT EXISTS crf_options"));
        let units_sql = include_str!("../../../../migrations/20261103135355_create_crf_units.sql");
        assert!(units_sql.contains("CREATE TABLE IF NOT EXISTS crf_units"));
    }
}
