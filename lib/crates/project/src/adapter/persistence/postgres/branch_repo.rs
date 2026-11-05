use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, ProjectBranch, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate,
};

use super::row::ProjectBranchRow;

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

/// PostgreSQL SQLSTATE for foreign-key violation.
const SQLSTATE_FOREIGN_KEY_VIOLATION: &str = "23503";

/// The name of the constraint that enforces branch-name uniqueness
/// within a project.
const BRANCH_UNIQUE_CONSTRAINT: &str = "project_branches_branch_unique";

/// PostgreSQL-backed implementation of `ProjectBranchRepository`.
pub struct ProjectBranchRepo {
    pool: PgPool,
}

impl ProjectBranchRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectBranchRepository for ProjectBranchRepo {
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError> {
        let row: ProjectBranchRow =
            sqlx::QueryBuilder::new("INSERT INTO project_branches (project_id, branch) VALUES (")
                .push_bind(input.project_id)
                .push(", ")
                .push_bind(&input.branch)
                .push(") RETURNING id, project_id, branch, created_at, updated_at")
                .build_query_as::<ProjectBranchRow>()
                .fetch_one(&self.pool)
                .await
                .map_err(|e| map_write_error(e, input.project_id, Some(&input.branch)))?;

        row.try_into()
    }

    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError> {
        let row: ProjectBranchRow = sqlx::QueryBuilder::new(
            "SELECT id, project_id, branch, created_at, updated_at \
             FROM project_branches WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<ProjectBranchRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::BranchNotFound(id))?;
        row.try_into()
    }

    async fn list_by_project(&self, project_id: i32) -> Result<Vec<ProjectBranch>, DomainError> {
        let rows: Vec<ProjectBranchRow> = sqlx::QueryBuilder::new(
            "SELECT id, project_id, branch, created_at, updated_at \
             FROM project_branches WHERE project_id = ",
        )
        .push_bind(project_id)
        .push(" ORDER BY id")
        .build_query_as::<ProjectBranchRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(|row| row.try_into()).collect()
    }

    async fn update(&self, input: ProjectBranchUpdate) -> Result<ProjectBranch, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // The write is scoped to the owning project: a branch of
        // another project is reported as not found rather than
        // updated.
        let owner: Option<(i32,)> =
            sqlx::QueryBuilder::new("SELECT project_id FROM project_branches WHERE id = ")
                .push_bind(input.id)
                .build_query_as::<(i32,)>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        match owner {
            None => return Err(DomainError::BranchNotFound(input.id)),
            Some((project_id,)) if project_id != input.project_id => {
                return Err(DomainError::BranchNotFound(input.id));
            }
            Some(_) => {}
        }

        if let Some(ref branch) = input.branch {
            let row: ProjectBranchRow =
                sqlx::QueryBuilder::new("UPDATE project_branches SET branch = ")
                    .push_bind(branch)
                    .push(" WHERE id = ")
                    .push_bind(input.id)
                    .push(" AND project_id = ")
                    .push_bind(input.project_id)
                    .push(" RETURNING id, project_id, branch, created_at, updated_at")
                    .build_query_as::<ProjectBranchRow>()
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| map_write_error(e, input.project_id, input.branch.as_deref()))?
                    .ok_or(DomainError::BranchNotFound(input.id))?;
            tx.commit().await.map_err(map_db_error)?;
            return row.try_into();
        }

        // Nothing to change: return the branch as it stands.
        tx.commit().await.map_err(map_db_error)?;
        self.find_by_id(input.id).await
    }
}

/// Translate a driver error into a domain error.
///
/// A unique violation on the branch-name constraint is a duplicate
/// branch; any other unique violation keeps the generic
/// `DuplicateCode` behaviour.
fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                DomainError::DuplicateCode(format!(
                    "(constraint {})",
                    db_err.constraint().unwrap_or("code")
                ))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}

/// Translate a driver error from a branch write into a domain error.
///
/// Two cases get a message that names what the caller asked for,
/// rather than the driver's wording:
///
/// - a unique violation on the branch-name constraint becomes
///   [`DomainError::DuplicateBranch`] carrying the attempted name -
///   the driver's own message only names the constraint;
/// - a foreign-key violation becomes
///   [`DomainError::BranchProjectNotFound`] carrying the project id.
fn map_write_error(err: sqlx::Error, project_id: i32, branch: Option<&str>) -> DomainError {
    if let sqlx::Error::Database(ref db_err) = err {
        if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION)
            && db_err.constraint() == Some(BRANCH_UNIQUE_CONSTRAINT)
        {
            return DomainError::DuplicateBranch(branch.unwrap_or_default().to_string());
        }
        if db_err.code().as_deref() == Some(SQLSTATE_FOREIGN_KEY_VIOLATION) {
            return DomainError::BranchProjectNotFound(project_id);
        }
    }
    map_db_error(err)
}
