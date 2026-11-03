// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
//
// This repository is the **only** writer of
// `sdtm_project_domain_overrides`. The domain repository reads the
// table (via a LEFT JOIN) but never writes to it.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainCategory, DomainError, SdtmDomainDescription, SdtmProjectDomainOverride,
    SdtmProjectDomainOverrideNew, SdtmProjectDomainOverrideRepository,
    SdtmProjectDomainOverrideUpdate,
};

#[derive(FromRow)]
struct SdtmProjectDomainOverrideRow {
    id: i64,
    domain_id: i64,
    category: String,
    descriptions: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmProjectDomainOverrideRow {
    fn into_override(self) -> Result<SdtmProjectDomainOverride, DomainError> {
        let category = DomainCategory::try_from(self.category.as_str())?;
        let descriptions: Vec<SdtmDomainDescription> = serde_json::from_value(self.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmProjectDomainOverride::for_repository(
            self.id,
            self.domain_id,
            category,
            descriptions,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmProjectDomainOverrideRepoPg {
    pool: PgPool,
}

impl SdtmProjectDomainOverrideRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmProjectDomainOverrideRepository for SdtmProjectDomainOverrideRepoPg {
    async fn create(
        &self,
        input: SdtmProjectDomainOverrideNew,
    ) -> Result<SdtmProjectDomainOverride, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let category_str = input.category.as_str();
        let row: SdtmProjectDomainOverrideRow = sqlx::query_as::<_, SdtmProjectDomainOverrideRow>(
            "INSERT INTO sdtm_project_domain_overrides (domain_id, category, descriptions)
             VALUES ($1, $2, $3)
             RETURNING id, domain_id, category, descriptions, created_at, updated_at",
        )
        .bind(input.domain_id)
        .bind(category_str)
        .bind(descriptions_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_domain_override_err(e, input.domain_id))?;
        row.into_override()
    }

    async fn find_by_domain_id(
        &self,
        domain_id: i64,
    ) -> Result<SdtmProjectDomainOverride, DomainError> {
        let row: SdtmProjectDomainOverrideRow = sqlx::query_as::<_, SdtmProjectDomainOverrideRow>(
            "SELECT id, domain_id, category, descriptions, created_at, updated_at
             FROM sdtm_project_domain_overrides
             WHERE domain_id = $1",
        )
        .bind(domain_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_domain_override_err(e, 0))?
        .ok_or(DomainError::SdtmProjectDomainOverrideNotFound(domain_id))?;
        row.into_override()
    }

    async fn update(
        &self,
        input: SdtmProjectDomainOverrideUpdate,
    ) -> Result<SdtmProjectDomainOverride, DomainError> {
        // COALESCE: a NULL argument leaves the column unchanged. A
        // `Some(vec![])` binds an empty JSONB array (not NULL), so the
        // column is cleared rather than left untouched.
        let category_str = input.category.map(|c| c.as_str());
        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let row: SdtmProjectDomainOverrideRow = sqlx::query_as::<_, SdtmProjectDomainOverrideRow>(
            "UPDATE sdtm_project_domain_overrides SET
                category     = COALESCE($2, category),
                descriptions = COALESCE($3, descriptions)
             WHERE domain_id = $1
             RETURNING id, domain_id, category, descriptions, created_at, updated_at",
        )
        .bind(input.domain_id)
        .bind(category_str)
        .bind(descriptions_json)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_domain_override_err(e, 0))?
        .ok_or(DomainError::SdtmProjectDomainOverrideNotFound(
            input.domain_id,
        ))?;
        row.into_override()
    }

    async fn delete(&self, domain_id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_project_domain_overrides WHERE domain_id = $1")
            .bind(domain_id)
            .execute(&self.pool)
            .await
            .map_err(|e| map_domain_override_err(e, 0))?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmProjectDomainOverrideNotFound(domain_id));
        }
        Ok(())
    }
}

/// Map a driver error to a domain error.
///
/// `parent_id` is the parent the statement referenced. A foreign-key
/// violation can only arise from an INSERT, so the read and update paths
/// pass 0; the create path passes the id the caller actually used.
fn map_domain_override_err(err: sqlx::Error, parent_id: i64) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                // Unique violation on (domain_id): the domain already
                // holds an override.
                return DomainError::DuplicateSdtmProjectDomainOverride { domain_id: 0 };
            }
            if db.code().as_deref() == Some("23503") {
                // FK violation: the parent domain is gone.
                return DomainError::FkSdtmProjectDomainNotFound(parent_id);
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
    fn migration_file_declares_domain_override_table() {
        let sql = include_str!(
            "../../../../migrations/20261103135413_create_sdtm_project_domain_overrides.sql"
        );
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_domain_overrides"));
        assert!(
            sql.contains(
                "CONSTRAINT sdtm_project_domain_overrides_domain_unique UNIQUE (domain_id)"
            )
        );
        assert!(sql.contains("sdtm_project_domain_overrides_category_check"));
        assert!(sql.contains("sdtm_project_domain_overrides_updated_at"));
    }
}
