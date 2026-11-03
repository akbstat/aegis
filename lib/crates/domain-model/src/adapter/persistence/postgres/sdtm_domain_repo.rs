use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription, SdtmDomainNew,
    SdtmDomainRepository, SdtmDomainUpdate,
};

#[derive(FromRow)]
struct SdtmDomainRow {
    id: i64,
    version_id: i64,
    name: String,
    category: String,
    descriptions: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmDomainRow {
    fn into_domain(self) -> Result<SdtmDomain, DomainError> {
        let category = DomainCategory::try_from(self.category.as_str())?;
        let descriptions: Vec<SdtmDomainDescription> = serde_json::from_value(self.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmDomain::for_repository(
            self.id,
            self.version_id,
            self.name,
            category,
            descriptions,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmDomainRepoPg {
    pool: PgPool,
}

impl SdtmDomainRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmDomainRepository for SdtmDomainRepoPg {
    async fn create(&self, input: SdtmDomainNew) -> Result<SdtmDomain, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let category_str = input.category.as_str();
        let row: SdtmDomainRow = sqlx::query_as::<_, SdtmDomainRow>(
            "INSERT INTO sdtm_domains
                (version_id, name, category, descriptions)
             VALUES ($1, $2, $3, $4)
             RETURNING id, version_id, name, category, descriptions,
                       created_at, updated_at",
        )
        .bind(input.version_id)
        .bind(&input.name)
        .bind(category_str)
        .bind(descriptions_json)
        .fetch_one(&self.pool)
        .await
        .map_err(map_domain_err)?;
        row.into_domain()
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmDomain, DomainError> {
        let row: SdtmDomainRow = sqlx::query_as::<_, SdtmDomainRow>(
            "SELECT id, version_id, name, category, descriptions,
                    created_at, updated_at
             FROM sdtm_domains WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_domain_err)?
        .ok_or(DomainError::SdtmDomainNotFound(id))?;
        row.into_domain()
    }

    async fn list_by_version(&self, version_id: i64) -> Result<Vec<SdtmDomain>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmDomainRow>(
            "SELECT id, version_id, name, category, descriptions,
                    created_at, updated_at
             FROM sdtm_domains
             WHERE version_id = $1
             ORDER BY id ASC",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_domain_err)?;
        rows.into_iter().map(SdtmDomainRow::into_domain).collect()
    }

    async fn update(&self, input: SdtmDomainUpdate) -> Result<SdtmDomain, DomainError> {
        // COALESCE: NULL argument -> column unchanged.
        let category_str = input.category.map(|c| c.as_str());
        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let row: SdtmDomainRow = sqlx::query_as::<_, SdtmDomainRow>(
            "UPDATE sdtm_domains SET
                name         = COALESCE($2, name),
                category     = COALESCE($3, category),
                descriptions = COALESCE($4, descriptions)
             WHERE id = $1
             RETURNING id, version_id, name, category, descriptions,
                       created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.name.as_deref())
        .bind(category_str)
        .bind(descriptions_json)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_domain_err)?
        .ok_or(DomainError::SdtmDomainNotFound(input.id))?;
        row.into_domain()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_domains WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_domain_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmDomainNotFound(id));
        }
        Ok(())
    }
}

fn map_domain_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                // UniqueViolation on (version_id, name).
                return DomainError::DuplicateSdtmDomain {
                    version_id: 0,
                    name: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                // FK violation: most likely missing parent version.
                return DomainError::FkSdtmVersionNotFound(0);
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
        let sql = include_str!("../../../../migrations/20261103135401_create_sdtm_domains.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_domains"));
        assert!(sql.contains("descriptions  JSONB"));
        assert!(sql.contains("sdtm_domains_category_check"));
    }
}
