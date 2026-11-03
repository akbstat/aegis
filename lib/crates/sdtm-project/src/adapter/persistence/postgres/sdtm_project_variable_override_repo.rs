// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
//
// This repository is the **only** writer of
// `sdtm_project_variable_overrides`. The variable repository reads the
// table (via a LEFT JOIN) but never writes to it.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{AssertSqlSafe, FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmProjectVariableOverride, SdtmProjectVariableOverrideNew,
    SdtmProjectVariableOverrideRepository, SdtmProjectVariableOverrideUpdate, SdtmRole,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableType,
};

#[derive(FromRow)]
struct SdtmProjectVariableOverrideRow {
    id: i64,
    variable_id: i64,
    variable_type: String,
    variable_core: String,
    variable_role: Option<String>,
    descriptions: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmProjectVariableOverrideRow {
    fn into_override(self) -> Result<SdtmProjectVariableOverride, DomainError> {
        let variable_type = SdtmVariableType::try_from(self.variable_type.as_str())?;
        let variable_core = SdtmVariableCore::try_from(self.variable_core.as_str())?;
        let variable_role = match self.variable_role.as_deref() {
            None => None,
            Some(s) => Some(SdtmRole::try_from(s)?),
        };
        let descriptions: Vec<SdtmVariableDescription> = serde_json::from_value(self.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmProjectVariableOverride::for_repository(
            self.id,
            self.variable_id,
            variable_type,
            variable_core,
            variable_role,
            descriptions,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmProjectVariableOverrideRepoPg {
    pool: PgPool,
}

impl SdtmProjectVariableOverrideRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmProjectVariableOverrideRepository for SdtmProjectVariableOverrideRepoPg {
    async fn create(
        &self,
        input: SdtmProjectVariableOverrideNew,
    ) -> Result<SdtmProjectVariableOverride, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let type_str = input.variable_type.as_str();
        let core_str = input.variable_core.as_str();
        let role_str = input.variable_role.map(|r| r.as_str());
        let row: SdtmProjectVariableOverrideRow =
            sqlx::query_as::<_, SdtmProjectVariableOverrideRow>(
                "INSERT INTO sdtm_project_variable_overrides
                    (variable_id, variable_type, variable_core, variable_role, descriptions)
                 VALUES ($1, $2, $3, $4, $5)
                 RETURNING id, variable_id, variable_type, variable_core, variable_role,
                           descriptions, created_at, updated_at",
            )
            .bind(input.variable_id)
            .bind(type_str)
            .bind(core_str)
            .bind(role_str)
            .bind(descriptions_json)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| map_variable_override_err(e, input.variable_id))?;
        row.into_override()
    }

    async fn find_by_variable_id(
        &self,
        variable_id: i64,
    ) -> Result<SdtmProjectVariableOverride, DomainError> {
        let row: SdtmProjectVariableOverrideRow =
            sqlx::query_as::<_, SdtmProjectVariableOverrideRow>(
                "SELECT id, variable_id, variable_type, variable_core, variable_role,
                        descriptions, created_at, updated_at
                 FROM sdtm_project_variable_overrides
                 WHERE variable_id = $1",
            )
            .bind(variable_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_variable_override_err(e, 0))?
            .ok_or(DomainError::SdtmProjectVariableOverrideNotFound(
                variable_id,
            ))?;
        row.into_override()
    }

    async fn update(
        &self,
        input: SdtmProjectVariableOverrideUpdate,
    ) -> Result<SdtmProjectVariableOverride, DomainError> {
        // For the nullable `variable_role` column the three-state
        // semantics are:
        //   - outer None       -> column unchanged
        //   - outer Some(None) -> column cleared to NULL
        //   - outer Some(Some) -> column replaced
        // We translate that to SQL by picking between `variable_role =
        // $4` (for "clear" / "replace") and
        // `variable_role = COALESCE($4, variable_role)` (for "don't
        // change"). The bound value is the flattened role, so the
        // "don't change" case binds NULL and reads the column back.
        let type_str = input.variable_type.map(|t| t.as_str());
        let core_str = input.variable_core.map(|c| c.as_str());
        let role_bound: Option<&str> = input.variable_role.flatten().map(|r| r.as_str());
        let clear_role = input.variable_role.is_some();

        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };

        let role_expr = if clear_role {
            "$4".to_string()
        } else {
            "COALESCE($4, variable_role)".to_string()
        };

        let sql = format!(
            "UPDATE sdtm_project_variable_overrides SET
                variable_type = COALESCE($2, variable_type),
                variable_core = COALESCE($3, variable_core),
                descriptions  = COALESCE($5, descriptions),
                variable_role = {role}
             WHERE variable_id = $1
             RETURNING id, variable_id, variable_type, variable_core, variable_role,
                       descriptions, created_at, updated_at",
            role = role_expr,
        );

        let row: SdtmProjectVariableOverrideRow =
            sqlx::query_as::<_, SdtmProjectVariableOverrideRow>(AssertSqlSafe(sql))
                .bind(input.variable_id)
                .bind(type_str)
                .bind(core_str)
                .bind(role_bound)
                .bind(descriptions_json)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| map_variable_override_err(e, 0))?
                .ok_or(DomainError::SdtmProjectVariableOverrideNotFound(
                    input.variable_id,
                ))?;
        row.into_override()
    }

    async fn delete(&self, variable_id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_project_variable_overrides WHERE variable_id = $1")
            .bind(variable_id)
            .execute(&self.pool)
            .await
            .map_err(|e| map_variable_override_err(e, 0))?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmProjectVariableOverrideNotFound(
                variable_id,
            ));
        }
        Ok(())
    }
}

/// Map a driver error to a domain error.
///
/// `parent_id` is the parent the statement referenced. A foreign-key
/// violation can only arise from an INSERT, so the read and update paths
/// pass 0; the create path passes the id the caller actually used.
fn map_variable_override_err(err: sqlx::Error, parent_id: i64) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                // Unique violation on (variable_id): the variable
                // already holds an override.
                return DomainError::DuplicateSdtmProjectVariableOverride { variable_id: 0 };
            }
            if db.code().as_deref() == Some("23503") {
                // FK violation: the parent variable is gone.
                return DomainError::FkSdtmProjectVariableNotFound(parent_id);
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
    fn migration_file_declares_variable_override_table() {
        let sql = include_str!(
            "../../../../migrations/20261103135416_create_sdtm_project_variable_overrides.sql"
        );
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_variable_overrides"));
        assert!(sql.contains(
            "CONSTRAINT sdtm_project_variable_overrides_variable_unique UNIQUE (variable_id)"
        ));
        assert!(sql.contains("sdtm_project_variable_overrides_type_check"));
        assert!(sql.contains("sdtm_project_variable_overrides_core_check"));
        assert!(sql.contains("sdtm_project_variable_overrides_role_check"));
        assert!(sql.contains("sdtm_project_variable_overrides_updated_at"));
    }
}
