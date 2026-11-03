//! Adapts `apis::project::ProjectService` to the domain's
//! `ProjectLookup` port.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{DomainError, ProjectLookup};

/// Looks a project up through the `apis::project::ProjectService`
/// port, so the `structures` crate does not depend on the `project`
/// crate directly.
pub struct ProjectLookupImpl {
    project: Arc<dyn apis::project::ProjectService>,
}

impl ProjectLookupImpl {
    pub fn new(project: Arc<dyn apis::project::ProjectService>) -> Self {
        Self { project }
    }
}

#[async_trait]
impl ProjectLookup for ProjectLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        self.project
            .get_project_by_code(code)
            .await
            .map(|_| ())
            .map_err(|err| match err {
                apis::project::ProjectApiError::NotFound => {
                    DomainError::ProjectNotFound(code.to_string())
                }
                other => DomainError::Repository(other.to_string()),
            })
    }
}
