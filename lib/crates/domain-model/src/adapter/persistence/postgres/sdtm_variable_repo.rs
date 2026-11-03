use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{AssertSqlSafe, FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmRole, SdtmVariable, SdtmVariableControlled, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableNew, SdtmVariableRepository, SdtmVariableType,
    SdtmVariableUpdate,
};

#[derive(FromRow)]
struct SdtmVariableRow {
    id: i64,
    domain_id: i64,
    name: String,
    variable_controlled: Option<JsonValue>,
    variable_type: String,
    variable_core: String,
    variable_role: Option<String>,
    variable_sequence: i64,
    descriptions: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmVariableRow {
    fn into_var(self) -> Result<SdtmVariable, DomainError> {
        let variable_type = SdtmVariableType::try_from(self.variable_type.as_str())?;
        let variable_core = SdtmVariableCore::try_from(self.variable_core.as_str())?;
        let variable_role = match self.variable_role.as_deref() {
            None => None,
            Some(s) => Some(SdtmRole::try_from(s)?),
        };
        let variable_controlled: Option<SdtmVariableControlled> = match self.variable_controlled {
            None => None,
            // A JSON `null` in the column is not a valid
            // SdtmVariableControlled, but it means "no controlled
            // terminology" just as SQL NULL does.
            Some(v) if v.is_null() => None,
            Some(v) => Some(
                serde_json::from_value(v).map_err(|e| DomainError::Repository(e.to_string()))?,
            ),
        };
        let descriptions: Vec<SdtmVariableDescription> = serde_json::from_value(self.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmVariable::for_repository(
            self.id,
            self.domain_id,
            self.name,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            self.variable_sequence,
            descriptions,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmVariableRepoPg {
    pool: PgPool,
}

/// Builds the UPDATE statement for `sdtm_variables`.
///
/// The two nullable columns (`variable_controlled`, `variable_role`)
/// each carry a three-state contract:
///
/// - outer `None` — "don't change" — needs `COALESCE($n, column)`, so
///   the current value survives the write.
/// - outer `Some(_)` — "clear" or "replace" — writes the bind
///   directly with `$n`. Those two states share one fragment because
///   the *bound value* tells them apart: SQL `NULL` clears the column,
///   a value replaces it.
///
/// `variable_controlled` binds a JSONB document, `variable_role` binds
/// a text role name.
fn variable_update_sql(clear_controlled: bool, clear_role: bool) -> String {
    let ctrl_expr = if clear_controlled {
        "$7".to_string()
    } else {
        "COALESCE($7, variable_controlled)".to_string()
    };
    let role_expr = if clear_role {
        "$8".to_string()
    } else {
        "COALESCE($8, variable_role)".to_string()
    };

    format!(
        "UPDATE sdtm_variables SET
            name                = COALESCE($2, name),
            variable_type       = COALESCE($3, variable_type),
            variable_core       = COALESCE($4, variable_core),
            variable_sequence   = COALESCE($5, variable_sequence),
            descriptions        = COALESCE($6, descriptions),
            variable_controlled = {ctrl},
            variable_role       = {role}
         WHERE id = $1
         RETURNING id, domain_id, name, variable_controlled,
                   variable_type, variable_core, variable_role,
                   variable_sequence, descriptions,
                   created_at, updated_at",
        ctrl = ctrl_expr,
        role = role_expr,
    )
}

impl SdtmVariableRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmVariableRepository for SdtmVariableRepoPg {
    async fn create(&self, input: SdtmVariableNew) -> Result<SdtmVariable, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let variable_controlled_json: Option<JsonValue> = match &input.variable_controlled {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let type_str = input.variable_type.as_str();
        let core_str = input.variable_core.as_str();
        let role_str = input.variable_role.map(|r| r.as_str());
        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(
            "INSERT INTO sdtm_variables
                (domain_id, name, variable_controlled, variable_type,
                 variable_core, variable_role, variable_sequence,
                 descriptions)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, domain_id, name, variable_controlled,
                       variable_type, variable_core, variable_role,
                       variable_sequence, descriptions,
                       created_at, updated_at",
        )
        .bind(input.domain_id)
        .bind(&input.name)
        .bind(variable_controlled_json)
        .bind(type_str)
        .bind(core_str)
        .bind(role_str)
        .bind(input.variable_sequence)
        .bind(descriptions_json)
        .fetch_one(&self.pool)
        .await
        .map_err(map_variable_err)?;
        row.into_var()
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmVariable, DomainError> {
        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(
            "SELECT id, domain_id, name, variable_controlled,
                    variable_type, variable_core, variable_role,
                    variable_sequence, descriptions,
                    created_at, updated_at
             FROM sdtm_variables WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_variable_err)?
        .ok_or(DomainError::SdtmVariableNotFound(id))?;
        row.into_var()
    }

    async fn list_by_domain(&self, domain_id: i64) -> Result<Vec<SdtmVariable>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmVariableRow>(
            "SELECT id, domain_id, name, variable_controlled,
                    variable_type, variable_core, variable_role,
                    variable_sequence, descriptions,
                    created_at, updated_at
             FROM sdtm_variables
             WHERE domain_id = $1
             ORDER BY variable_sequence ASC, id ASC",
        )
        .bind(domain_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_variable_err)?;
        rows.into_iter().map(SdtmVariableRow::into_var).collect()
    }

    async fn update(&self, input: SdtmVariableUpdate) -> Result<SdtmVariable, DomainError> {
        let name = input.name.as_deref();
        let variable_type = input.variable_type.map(|t| t.as_str());
        let variable_core = input.variable_core.map(|c| c.as_str());
        let variable_sequence = input.variable_sequence;

        let variable_controlled_bound: Option<JsonValue> = match &input.variable_controlled {
            None => None,
            Some(None) => None,
            Some(Some(v)) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let clear_controlled = input.variable_controlled.is_some();

        let variable_role_bound: Option<&str> = match &input.variable_role {
            None => None,
            Some(None) => None,
            Some(Some(r)) => Some(r.as_str()),
        };
        let clear_role = input.variable_role.is_some();

        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };

        let sql = variable_update_sql(clear_controlled, clear_role);

        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(AssertSqlSafe(sql))
            .bind(input.id)
            .bind(name)
            .bind(variable_type)
            .bind(variable_core)
            .bind(variable_sequence)
            .bind(descriptions_json)
            .bind(variable_controlled_bound)
            .bind(variable_role_bound)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_variable_err)?
            .ok_or(DomainError::SdtmVariableNotFound(input.id))?;
        row.into_var()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_variables WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_variable_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmVariableNotFound(id));
        }
        Ok(())
    }
}

fn map_variable_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                return DomainError::DuplicateSdtmVariable {
                    domain_id: 0,
                    name: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                return DomainError::FkSdtmDomainNotFound(0);
            }
            DomainError::Repository(err.to_string())
        }
        E::RowNotFound => DomainError::NotFound,
        _ => DomainError::Repository(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collapse runs of whitespace so the SQL assertions do not depend
    /// on the statement's indentation.
    fn normalize_whitespace(sql: &str) -> String {
        sql.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn migration_file_is_present_and_idempotent() {
        let sql = include_str!("../../../../migrations/20261103135402_create_sdtm_variables.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_variables"));
        assert!(sql.contains("descriptions         JSONB"));
        assert!(sql.contains("sdtm_variables_type_check"));
        assert!(sql.contains("sdtm_variables_core_check"));
        assert!(sql.contains("sdtm_variables_role_check"));
    }

    #[test]
    fn variable_controlled_migration_converts_the_column_to_jsonb() {
        let sql = include_str!(
            "../../../../migrations/20261103135403_alter_sdtm_variables_variable_controlled_to_jsonb.sql"
        );
        assert!(sql.contains("ALTER TABLE sdtm_variables"));
        // The column that changes must be named, and the conversion
        // must spell out `USING` — there is no implicit `text` ->
        // `jsonb` assignment cast, so `USING NULL::jsonb` is what
        // makes the statement legal.
        assert!(sql.contains("variable_controlled"));
        assert!(sql.contains("TYPE JSONB"));
        assert!(sql.contains("USING NULL::jsonb"));
    }

    /// The three-state semantics of the two nullable columns live in
    /// the SQL fragment, not in the bound value alone: "don't change"
    /// needs `COALESCE`, while "clear" and "replace" share the plain
    /// fragment and are told apart by what is bound (NULL clears, a
    /// value replaces). This pins the fragment selection without a
    /// database.
    #[test]
    fn update_sql_picks_the_write_fragment_per_state() {
        let unchanged = normalize_whitespace(&variable_update_sql(false, false));
        assert!(
            unchanged.contains("variable_controlled = COALESCE($7, variable_controlled)"),
            "an untouched field must fall back to the current column: {unchanged}"
        );
        assert!(unchanged.contains("variable_role = COALESCE($8, variable_role)"));

        let cleared = normalize_whitespace(&variable_update_sql(true, true));
        assert!(
            cleared.contains("variable_controlled = $7"),
            "a cleared field must be written straight from the bind: {cleared}"
        );
        assert!(cleared.contains("variable_role = $8"));
        assert!(
            !cleared.contains("variable_controlled = COALESCE"),
            "a cleared field must not fall back to the current column: {cleared}"
        );
        assert!(!cleared.contains("variable_role = COALESCE"));
    }

    #[test]
    fn row_with_json_null_variable_controlled_decodes_as_absent() {
        let row = SdtmVariableRow {
            id: 1,
            domain_id: 1,
            name: "AESEV".into(),
            variable_controlled: Some(serde_json::Value::Null),
            variable_type: "Character".into(),
            variable_core: "Req".into(),
            variable_role: Some("Topic".into()),
            variable_sequence: 1,
            descriptions: serde_json::json!([]),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let var = row.into_var().unwrap();
        assert_eq!(var.variable_controlled, None);
    }

    #[test]
    fn row_with_malformed_variable_controlled_is_a_repository_error() {
        let row = SdtmVariableRow {
            id: 1,
            domain_id: 1,
            name: "AESEV".into(),
            variable_controlled: Some(serde_json::json!({ "code": "C66731" })),
            variable_type: "Character".into(),
            variable_core: "Req".into(),
            variable_role: Some("Topic".into()),
            variable_sequence: 1,
            descriptions: serde_json::json!([]),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let err = row.into_var().unwrap_err();
        assert!(matches!(err, DomainError::Repository(_)), "got {err:?}");
    }
}
