use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainAnnotation, DomainAnnotationNew, DomainAnnotationRepository, DomainAnnotationUpdate,
    DomainError,
};

#[derive(FromRow)]
struct DomainAnnotationRow {
    id: i64,
    form_id: i64,
    name: String,
    description: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DomainAnnotationRow> for DomainAnnotation {
    fn from(r: DomainAnnotationRow) -> Self {
        DomainAnnotation::for_repository(
            r.id,
            r.form_id,
            r.name,
            r.description,
            r.created_at,
            r.updated_at,
        )
    }
}

#[derive(Clone)]
pub struct DomainAnnotationRepoPg {
    pool: PgPool,
}

impl DomainAnnotationRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DomainAnnotationRepository for DomainAnnotationRepoPg {
    async fn create(&self, input: DomainAnnotationNew) -> Result<DomainAnnotation, DomainError> {
        let row: DomainAnnotationRow = sqlx::query_as::<_, DomainAnnotationRow>(
            "INSERT INTO crf_domain_annotations (form_id, name, description)
             VALUES ($1, $2, $3)
             RETURNING id, form_id, name, description, created_at, updated_at",
        )
        .bind(input.form_id)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<DomainAnnotation, DomainError> {
        let row: DomainAnnotationRow = sqlx::query_as::<_, DomainAnnotationRow>(
            "SELECT id, form_id, name, description, created_at, updated_at
             FROM crf_domain_annotations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::DomainAnnotationNotFound(id))?;
        Ok(row.into())
    }

    async fn list_by_form(&self, form_id: i64) -> Result<Vec<DomainAnnotation>, DomainError> {
        let rows = sqlx::query_as::<_, DomainAnnotationRow>(
            "SELECT id, form_id, name, description, created_at, updated_at
             FROM crf_domain_annotations WHERE form_id = $1 ORDER BY id ASC",
        )
        .bind(form_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, input: DomainAnnotationUpdate) -> Result<DomainAnnotation, DomainError> {
        let row: DomainAnnotationRow = sqlx::query_as::<_, DomainAnnotationRow>(
            "UPDATE crf_domain_annotations SET
                name        = COALESCE($2, name),
                description = COALESCE($3, description)
             WHERE id = $1
             RETURNING id, form_id, name, description, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.name.as_deref())
        .bind(input.description.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::DomainAnnotationNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM crf_domain_annotations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::DomainAnnotationNotFound(id));
        }
        Ok(())
    }

    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<DomainAnnotation>, DomainError> {
        let pat = format!("%{fragment}%");
        let rows = sqlx::query_as::<_, DomainAnnotationRow>(
            "SELECT d.id, d.form_id, d.name, d.description, d.created_at, d.updated_at
             FROM crf_domain_annotations d
             JOIN crf_forms f ON f.id = d.form_id
             WHERE f.version_id = $1 AND (d.name ILIKE $2 OR d.description ILIKE $2)
             ORDER BY d.id ASC",
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
                return DomainError::DuplicateDomainAnnotation {
                    form_id: 0,
                    name: "(unknown)".into(),
                };
            }
            if let Some(c) = db.constraint() {
                if c.contains("crf_domain_annotations_form_name") {
                    return DomainError::DuplicateDomainAnnotation {
                        form_id: 0,
                        name: "(unknown)".into(),
                    };
                }
                if c.contains("crf_domain_annotations_form_id_fkey") {
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
        let sql =
            include_str!("../../../../migrations/20261103135356_create_crf_domain_annotations.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS crf_domain_annotations"));
        assert!(sql.contains("crf_domain_annotations_form_name_unique"));
    }
}
