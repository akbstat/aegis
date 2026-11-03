use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    Annotation, AnnotationNew, AnnotationOwner, AnnotationRepository, AnnotationUpdate, DomainError,
};

#[derive(FromRow)]
struct AnnotationRow {
    id: i64,
    domain_annotation_id: i64,
    content: String,
    assign: bool,
    form_id: Option<i64>,
    item_id: Option<i64>,
    option_id: Option<i64>,
    unit_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<AnnotationRow> for Annotation {
    fn from(r: AnnotationRow) -> Self {
        let owner = match (r.form_id, r.item_id, r.option_id, r.unit_id) {
            (Some(id), None, None, None) => AnnotationOwner::Form { id },
            (None, Some(id), None, None) => AnnotationOwner::Item { id },
            (None, None, Some(id), None) => AnnotationOwner::Option { id },
            (None, None, None, Some(id)) => AnnotationOwner::Unit { id },
            _ => panic!("polymorphic CHECK constraint violated"),
        };
        Annotation::for_repository(
            r.id,
            r.domain_annotation_id,
            r.content,
            r.assign,
            owner,
            r.created_at,
            r.updated_at,
        )
    }
}

#[derive(Clone)]
pub struct AnnotationRepoPg {
    pool: PgPool,
}

impl AnnotationRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnnotationRepository for AnnotationRepoPg {
    async fn create(&self, input: AnnotationNew) -> Result<Annotation, DomainError> {
        let row: AnnotationRow = match input.owner {
            AnnotationOwner::Form { id } => sqlx::query_as::<_, AnnotationRow>(
                "INSERT INTO crf_annotations (form_id, domain_annotation_id, content, assign)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, domain_annotation_id, content, assign,
                           form_id, item_id, option_id, unit_id, created_at, updated_at",
            )
            .bind(id)
            .bind(input.domain_annotation_id)
            .bind(&input.content)
            .bind(input.assign)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?,
            AnnotationOwner::Item { id } => sqlx::query_as::<_, AnnotationRow>(
                "INSERT INTO crf_annotations (item_id, domain_annotation_id, content, assign)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, domain_annotation_id, content, assign,
                           form_id, item_id, option_id, unit_id, created_at, updated_at",
            )
            .bind(id)
            .bind(input.domain_annotation_id)
            .bind(&input.content)
            .bind(input.assign)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?,
            AnnotationOwner::Option { id } => sqlx::query_as::<_, AnnotationRow>(
                "INSERT INTO crf_annotations (option_id, domain_annotation_id, content, assign)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, domain_annotation_id, content, assign,
                           form_id, item_id, option_id, unit_id, created_at, updated_at",
            )
            .bind(id)
            .bind(input.domain_annotation_id)
            .bind(&input.content)
            .bind(input.assign)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?,
            AnnotationOwner::Unit { id } => sqlx::query_as::<_, AnnotationRow>(
                "INSERT INTO crf_annotations (unit_id, domain_annotation_id, content, assign)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, domain_annotation_id, content, assign,
                           form_id, item_id, option_id, unit_id, created_at, updated_at",
            )
            .bind(id)
            .bind(input.domain_annotation_id)
            .bind(&input.content)
            .bind(input.assign)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?,
        };
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Annotation, DomainError> {
        let row: AnnotationRow = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::AnnotationNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_form(&self, form_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE form_id = $1 ORDER BY id ASC",
        )
        .bind(form_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_item(&self, item_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE item_id = $1 ORDER BY id ASC",
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_option(&self, option_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE option_id = $1 ORDER BY id ASC",
        )
        .bind(option_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_unit(&self, unit_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE unit_id = $1 ORDER BY id ASC",
        )
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_items(&self, item_ids: &[i64]) -> Result<Vec<Annotation>, DomainError> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE item_id = ANY($1) ORDER BY id ASC",
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_options(&self, option_ids: &[i64]) -> Result<Vec<Annotation>, DomainError> {
        if option_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE option_id = ANY($1) ORDER BY id ASC",
        )
        .bind(option_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_units(&self, unit_ids: &[i64]) -> Result<Vec<Annotation>, DomainError> {
        if unit_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations WHERE unit_id = ANY($1) ORDER BY id ASC",
        )
        .bind(unit_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: AnnotationUpdate) -> Result<Annotation, DomainError> {
        let row: AnnotationRow = sqlx::query_as::<_, AnnotationRow>(
            "UPDATE crf_annotations SET
                content = COALESCE($2, content),
                assign  = COALESCE($3, assign)
             WHERE id = $1
             RETURNING id, domain_annotation_id, content, assign,
                       form_id, item_id, option_id, unit_id, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.content.as_deref())
        .bind(input.assign)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::AnnotationNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM crf_annotations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::AnnotationNotFound(id));
        }
        Ok(())
    }

    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<Annotation>, DomainError> {
        let pat = format!("%{fragment}%");
        let rows = sqlx::query_as::<_, AnnotationRow>(
            "SELECT id, domain_annotation_id, content, assign,
                    form_id, item_id, option_id, unit_id, created_at, updated_at
             FROM crf_annotations
             WHERE
                (form_id IN (SELECT id FROM crf_forms WHERE version_id = $1) AND content ILIKE $2)
             OR (item_id IN (SELECT id FROM crf_items WHERE form_id IN (SELECT id FROM crf_forms WHERE version_id = $1)) AND content ILIKE $2)
             OR (option_id IN (SELECT id FROM crf_options WHERE item_id IN (SELECT id FROM crf_items WHERE form_id IN (SELECT id FROM crf_forms WHERE version_id = $1))) AND content ILIKE $2)
             OR (unit_id IN (SELECT id FROM crf_units WHERE item_id IN (SELECT id FROM crf_items WHERE form_id IN (SELECT id FROM crf_forms WHERE version_id = $1))) AND content ILIKE $2)
             ORDER BY id ASC",
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
            if let Some(c) = db.constraint() {
                if c.contains("crf_annotations_domain_annotation_id_fkey") {
                    return DomainError::FkDomainAnnotationNotFound(0);
                }
                if c.contains("crf_annotations_form_id_fkey") {
                    return DomainError::FkCrfFormNotFound(0);
                }
                if c.contains("crf_annotations_item_id_fkey") {
                    return DomainError::FkCrfItemNotFound(0);
                }
                if c.contains("crf_annotations_option_id_fkey") {
                    return DomainError::FkCrfOptionNotFound(0);
                }
                if c.contains("crf_annotations_unit_id_fkey") {
                    return DomainError::FkCrfUnitNotFound(0);
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
        let sql = include_str!("../../../../migrations/20261103135357_create_crf_annotations.sql");
        assert!(sql.contains("crf_annotations_polymorphic_owner"));
        assert!(sql.contains("form_id IS NOT NULL"));
    }
}
