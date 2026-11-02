// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
//
// Every read here LEFT JOINs `sdtm_project_variable_overrides` so the
// returned variable carries its override in the same round trip. The
// UPDATE path uses a data-modifying CTE for the same reason: `PUT` and
// `GET` on a variable must return the same shape.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmProjectVariable, SdtmProjectVariableNew, SdtmProjectVariableRepository,
    SdtmProjectVariableUpdate, SdtmRole, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableType,
};

#[derive(FromRow)]
struct SdtmProjectVariableRow {
    id: i64,
    domain_id: i64,
    variable_code: String,
    variable_sequence: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // The override columns come from a LEFT JOIN and are all NULL
    // when the variable has no override.
    override_id: Option<i64>,
    override_variable_type: Option<String>,
    override_variable_core: Option<String>,
    override_variable_role: Option<String>,
    override_descriptions: Option<JsonValue>,
    override_created_at: Option<DateTime<Utc>>,
    override_updated_at: Option<DateTime<Utc>>,
}

impl SdtmProjectVariableRow {
    fn into_variable(self) -> Result<SdtmProjectVariable, DomainError> {
        let variable_override = match self.override_id {
            None => None,
            Some(id) => {
                let variable_type = SdtmVariableType::try_from(
                    self.override_variable_type.as_deref().unwrap_or_default(),
                )?;
                let variable_core = SdtmVariableCore::try_from(
                    self.override_variable_core.as_deref().unwrap_or_default(),
                )?;
                let variable_role = match self.override_variable_role.as_deref() {
                    None => None,
                    Some(s) => Some(SdtmRole::try_from(s)?),
                };
                let descriptions: Vec<SdtmVariableDescription> =
                    serde_json::from_value(self.override_descriptions.unwrap_or(JsonValue::Null))
                        .map_err(|e| DomainError::Repository(e.to_string()))?;
                Some(crate::domain::SdtmProjectVariableOverride::for_repository(
                    id,
                    self.id,
                    variable_type,
                    variable_core,
                    variable_role,
                    descriptions,
                    self.override_created_at.unwrap_or(self.created_at),
                    self.override_updated_at.unwrap_or(self.updated_at),
                ))
            }
        };
        Ok(SdtmProjectVariable::for_repository(
            self.id,
            self.domain_id,
            self.variable_code,
            self.variable_sequence,
            variable_override,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmProjectVariableRepoPg {
    pool: PgPool,
}

impl SdtmProjectVariableRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmProjectVariableRepository for SdtmProjectVariableRepoPg {
    async fn create(
        &self,
        input: SdtmProjectVariableNew,
    ) -> Result<SdtmProjectVariable, DomainError> {
        let row: SdtmProjectVariableRow = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "WITH inserted AS (
                 INSERT INTO sdtm_project_variables (domain_id, variable_code, variable_sequence)
                 VALUES ($1, $2, $3)
                 RETURNING id, domain_id, variable_code, variable_sequence, created_at, updated_at
             )
             SELECT i.id, i.domain_id, i.variable_code, i.variable_sequence,
                    i.created_at, i.updated_at,
                    o.id AS override_id,
                    o.variable_type AS override_variable_type,
                    o.variable_core AS override_variable_core,
                    o.variable_role AS override_variable_role,
                    o.descriptions AS override_descriptions,
                    o.created_at AS override_created_at,
                    o.updated_at AS override_updated_at
             FROM inserted i
             LEFT JOIN sdtm_project_variable_overrides o ON o.variable_id = i.id",
        )
        .bind(input.domain_id)
        .bind(&input.variable_code)
        .bind(input.variable_sequence)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, input.domain_id))?;
        row.into_variable()
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVariable, DomainError> {
        let row: SdtmProjectVariableRow = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "SELECT v.id, v.domain_id, v.variable_code, v.variable_sequence,
                    v.created_at, v.updated_at,
                    o.id AS override_id,
                    o.variable_type AS override_variable_type,
                    o.variable_core AS override_variable_core,
                    o.variable_role AS override_variable_role,
                    o.descriptions AS override_descriptions,
                    o.created_at AS override_created_at,
                    o.updated_at AS override_updated_at
             FROM sdtm_project_variables v
             LEFT JOIN sdtm_project_variable_overrides o ON o.variable_id = v.id
             WHERE v.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, 0))?
        .ok_or(DomainError::SdtmProjectVariableNotFound(id))?;
        row.into_variable()
    }

    async fn list_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmProjectVariable>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "SELECT v.id, v.domain_id, v.variable_code, v.variable_sequence,
                    v.created_at, v.updated_at,
                    o.id AS override_id,
                    o.variable_type AS override_variable_type,
                    o.variable_core AS override_variable_core,
                    o.variable_role AS override_variable_role,
                    o.descriptions AS override_descriptions,
                    o.created_at AS override_created_at,
                    o.updated_at AS override_updated_at
             FROM sdtm_project_variables v
             LEFT JOIN sdtm_project_variable_overrides o ON o.variable_id = v.id
             WHERE v.domain_id = $1
             ORDER BY v.variable_sequence ASC, v.id ASC",
        )
        .bind(domain_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, 0))?;
        rows.into_iter()
            .map(SdtmProjectVariableRow::into_variable)
            .collect()
    }

    async fn update(
        &self,
        input: SdtmProjectVariableUpdate,
    ) -> Result<SdtmProjectVariable, DomainError> {
        // COALESCE: a NULL argument leaves the column unchanged.
        let row: SdtmProjectVariableRow = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "WITH updated AS (
                 UPDATE sdtm_project_variables
                 SET variable_code     = COALESCE($2, variable_code),
                     variable_sequence = COALESCE($3, variable_sequence)
                 WHERE id = $1
                 RETURNING id, domain_id, variable_code, variable_sequence, created_at, updated_at
             )
             SELECT u.id, u.domain_id, u.variable_code, u.variable_sequence,
                    u.created_at, u.updated_at,
                    o.id AS override_id,
                    o.variable_type AS override_variable_type,
                    o.variable_core AS override_variable_core,
                    o.variable_role AS override_variable_role,
                    o.descriptions AS override_descriptions,
                    o.created_at AS override_created_at,
                    o.updated_at AS override_updated_at
             FROM updated u
             LEFT JOIN sdtm_project_variable_overrides o ON o.variable_id = u.id",
        )
        .bind(input.id)
        .bind(input.variable_code.as_deref())
        .bind(input.variable_sequence)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, 0))?
        .ok_or(DomainError::SdtmProjectVariableNotFound(input.id))?;
        row.into_variable()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        // The FK cascades: deleting a variable removes its override.
        let res = sqlx::query("DELETE FROM sdtm_project_variables WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| map_variable_err(e, 0))?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmProjectVariableNotFound(id));
        }
        Ok(())
    }
}

/// Map a driver error to a domain error.
///
/// `parent_domain_id` is the domain the statement referenced. A
/// foreign-key violation can only arise from an INSERT, so the read
/// and update paths pass 0; the create path passes the id the caller
/// actually used.
fn map_variable_err(err: sqlx::Error, parent_domain_id: i64) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                // Unique violation on (domain_id, variable_code). The
                // driver does not surface the natural key.
                return DomainError::DuplicateSdtmProjectVariable {
                    domain_id: 0,
                    variable_code: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                // FK violation: the parent domain is gone. This is
                // the backstop for the race where the domain is
                // deleted between the usecase's check and the insert.
                return DomainError::FkSdtmProjectDomainNotFound(parent_domain_id);
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
    fn migration_file_declares_variable_table() {
        let sql = include_str!("../../../../migrations/0004_create_sdtm_project_variables.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_variables"));
        assert!(sql.contains(
            "CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (domain_id, variable_code)"
        ));
        assert!(sql.contains("sdtm_project_variables_updated_at"));
    }
}
