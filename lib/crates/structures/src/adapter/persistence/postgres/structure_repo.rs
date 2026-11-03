//! PostgreSQL implementation of [`StructureRepository`].
//!
//! SQLx **runtime API** (`sqlx::query_as` / `sqlx::query`) is used
//! throughout: the workspace ships no `.sqlx/` offline cache and no
//! `DATABASE_URL` at build time, so the compile-time-checked macros are
//! intentionally avoided.

use async_trait::async_trait;
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::domain::{
    DomainError, Structure, StructureFilter, StructureLevel, StructureNew, StructureRepository,
};

/// A row of the `structures` table.
#[derive(sqlx::FromRow)]
struct StructureRow {
    id: i64,
    name: String,
    level: String,
    project_code: Option<String>,
    template_id: Option<i64>,
}

impl TryFrom<StructureRow> for Structure {
    type Error = DomainError;

    fn try_from(row: StructureRow) -> Result<Self, Self::Error> {
        let level = StructureLevel::try_from(row.level.as_str())
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(Structure::for_repository(
            row.id,
            row.name,
            level,
            row.project_code,
            row.template_id,
            Vec::new(),
        ))
    }
}

/// PostgreSQL-backed `StructureRepository`.
pub struct StructureRepoPg {
    pool: PgPool,
}

impl StructureRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StructureRepository for StructureRepoPg {
    async fn create(&self, input: StructureNew) -> Result<Structure, DomainError> {
        let row = sqlx::query_as::<_, StructureRow>(
            "INSERT INTO structures (name, level, project_code, template_id)
             VALUES ($1, $2, $3, $4)
             RETURNING id, name, level, project_code, template_id",
        )
        .bind(&input.name)
        .bind(input.level.as_str())
        .bind(&input.project_code)
        .bind(input.template_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| match &err {
            // The one-per-project index fired. The driver does not
            // surface which project, but the command does.
            sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
                DomainError::DuplicateProjectStructure(
                    input.project_code.clone().unwrap_or_default(),
                )
            }
            _ => map_error(err),
        })?;
        Structure::try_from(row)
    }

    async fn find_by_id(&self, id: i64) -> Result<Structure, DomainError> {
        let row = sqlx::query_as::<_, StructureRow>(
            "SELECT id, name, level, project_code, template_id
             FROM structures WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?
        .ok_or(DomainError::StructureNotFound(id))?;
        Structure::try_from(row)
    }

    async fn list(&self, filter: StructureFilter) -> Result<Vec<Structure>, DomainError> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id, name, level, project_code, template_id FROM structures WHERE 1 = 1",
        );
        if let Some(level) = filter.level {
            builder.push(" AND level = ").push_bind(level.as_str());
        }
        if let Some(project_code) = filter.project_code {
            builder.push(" AND project_code = ").push_bind(project_code);
        }
        builder.push(" ORDER BY id ASC");
        let rows = builder
            .build_query_as::<StructureRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_error)?;
        rows.into_iter().map(Structure::try_from).collect()
    }

    async fn rename(&self, id: i64, name: &str) -> Result<Structure, DomainError> {
        let row = sqlx::query_as::<_, StructureRow>(
            "UPDATE structures SET name = $2 WHERE id = $1
             RETURNING id, name, level, project_code, template_id",
        )
        .bind(id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?
        .ok_or(DomainError::StructureNotFound(id))?;
        Structure::try_from(row)
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let affected = sqlx::query("DELETE FROM structures WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_error)?
            .rows_affected();
        if affected == 0 {
            return Err(DomainError::StructureNotFound(id));
        }
        Ok(())
    }

    async fn count_project_structures(&self, project_code: &str) -> Result<i64, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM structures WHERE level = 'project' AND project_code = $1",
        )
        .bind(project_code)
        .fetch_one(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(count)
    }

    async fn is_template_referenced(&self, template_id: i64) -> Result<bool, DomainError> {
        let referenced: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM structures WHERE template_id = $1)")
                .bind(template_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_error)?;
        Ok(referenced)
    }
}

/// The only place in the crate that knows about `sqlx::Error`.
///
/// The driver does not surface the natural key of a violated
/// constraint, so the `Duplicate*` and `TemplateInUse` variants carry
/// an empty value or a zero id; the usecase's own checks produce the
/// useful message, and the constraint is the race backstop.
fn map_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db) => match db.code().as_deref() {
            Some("23505") => DomainError::DuplicateProjectStructure(String::new()),
            Some("23503") => DomainError::TemplateInUse(0),
            _ => DomainError::Repository(db.to_string()),
        },
        other => DomainError::Repository(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_structures_table_keeps_its_constraints() {
        let sql = include_str!("../../../../migrations/0001_create_structures.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS structures"));
        assert!(sql.contains(
            "CONSTRAINT structures_level_check CHECK (level IN ('template', 'project'))"
        ));
        assert!(sql.contains("CONSTRAINT structures_level_shape CHECK"));
        assert!(sql.contains("CREATE UNIQUE INDEX IF NOT EXISTS structures_one_per_project"));
        assert!(sql.contains("ON structures (project_code) WHERE level = 'project'"));
        assert!(sql.contains("REFERENCES structures(id) ON DELETE RESTRICT"));
        assert!(sql.contains("BEFORE UPDATE ON structures"));
    }
}
