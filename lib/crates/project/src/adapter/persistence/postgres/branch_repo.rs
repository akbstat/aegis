use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, ProjectBranch, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate,
};

use super::row::ProjectBranchRow;

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

/// The name of the constraint that enforces branch-name uniqueness
/// within a project.
const BRANCH_UNIQUE_CONSTRAINT: &str = "project_branches_branch_unique";

/// PostgreSQL-backed implementation of `ProjectBranchRepository`.
///
/// The switch rule — at most one active branch per project — is
/// enforced here, inside a transaction: the project's other active
/// branches are cleared before a new branch is activated. The
/// `project_branches_one_active_per_project` partial unique index is
/// the database-level backstop.
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
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // The switch: a branch created as active deactivates the
        // project's other active branches first, in the same
        // transaction.
        if input.active {
            sqlx::query(
                "UPDATE project_branches SET active = FALSE \
                 WHERE project_id = $1 AND active",
            )
            .bind(input.project_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let row: ProjectBranchRow = sqlx::QueryBuilder::new(
            "INSERT INTO project_branches (project_id, branch, active) VALUES (",
        )
        .push_bind(input.project_id)
        .push(", ")
        .push_bind(&input.branch)
        .push(", ")
        .push_bind(input.active)
        .push(") RETURNING id, project_id, branch, active, created_at, updated_at")
        .build_query_as::<ProjectBranchRow>()
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        tx.commit().await.map_err(map_db_error)?;

        row.try_into()
    }

    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError> {
        let row: ProjectBranchRow = sqlx::QueryBuilder::new(
            "SELECT id, project_id, branch, active, created_at, updated_at \
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
            "SELECT id, project_id, branch, active, created_at, updated_at \
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

        // The switch: activating this branch deactivates the
        // project's other active branches, in the same transaction.
        if input.active == Some(true) {
            sqlx::query(
                "UPDATE project_branches SET active = FALSE \
                 WHERE project_id = $1 AND id <> $2 AND active",
            )
            .bind(input.project_id)
            .bind(input.id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let mut qb = sqlx::QueryBuilder::new("UPDATE project_branches SET ");
        let mut first = true;
        if let Some(ref branch) = input.branch {
            qb.push("branch = ").push_bind(branch);
            first = false;
        }
        if let Some(active) = input.active {
            if !first {
                qb.push(", ");
            }
            qb.push("active = ").push_bind(active);
            first = false;
        }
        if !first {
            qb.push(" WHERE id = ")
                .push_bind(input.id)
                .push(" AND project_id = ")
                .push_bind(input.project_id)
                .push(" RETURNING id, project_id, branch, active, created_at, updated_at");
            let row: ProjectBranchRow = qb
                .build_query_as::<ProjectBranchRow>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?
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
                if db_err.constraint() == Some(BRANCH_UNIQUE_CONSTRAINT) {
                    DomainError::DuplicateBranch(db_err.message().to_string())
                } else {
                    DomainError::DuplicateCode(format!(
                        "(constraint {})",
                        db_err.constraint().unwrap_or("code")
                    ))
                }
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}
