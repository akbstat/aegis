//! `StructureService` adapter.
//!
//! Hosts `StructureServiceImpl` — the implementation of
//! `apis::structures::StructureService` that adapts
//! `structures::usecase::StructureUsecase` to the API contract.
//! Translation between `apis::structures::*` and `structures::*` happens
//! inline in each trait method.

use async_trait::async_trait;

use apis::structures::{
    AddNodeRequest, CreateStructureRequest, DeleteNodeRequest, ListStructuresRequest,
    MigrationView, NodeView, OwnNodeList, OwnNodeView, RenameStructureRequest, StructureApiError,
    StructureLevel, StructureList, StructureMetaView, StructureService, StructureSummaryView,
    StructureTreeView, StructureView, UpdateNodeRequest,
};

use crate::domain::{
    DomainError, Migration, NodeRepository, ProjectLookup, StructureLevel as DomainLevel,
    StructureRepository,
};
use crate::usecase::{
    AddNode, CreateStructure, DeleteNode, NodeView as InternalNodeView, RenameStructure,
    StructureMetaView as InternalMetaView, StructureSummaryView as InternalSummaryView,
    StructureTreeView as InternalTreeView, StructureUsecase,
    StructureView as InternalStructureView, UpdateNode, UsecaseError,
};

/// Adapter that implements [`StructureService`] on top of a
/// [`StructureUsecase`].
///
/// Generic over the three persistence ports so the adapter can be
/// exercised against in-memory fakes in tests and against the
/// PostgreSQL-backed repositories in production.
pub struct StructureServiceImpl<S, N, P>
where
    S: StructureRepository,
    N: NodeRepository,
    P: ProjectLookup,
{
    usecase: StructureUsecase<S, N, P>,
}

impl<S, N, P> StructureServiceImpl<S, N, P>
where
    S: StructureRepository,
    N: NodeRepository,
    P: ProjectLookup,
{
    /// Build a new `StructureServiceImpl` wrapping the supplied
    /// usecase.
    pub fn new(usecase: StructureUsecase<S, N, P>) -> Self {
        Self { usecase }
    }
}

#[async_trait]
impl<S, N, P> StructureService for StructureServiceImpl<S, N, P>
where
    S: StructureRepository,
    N: NodeRepository,
    P: ProjectLookup,
{
    async fn create_structure(
        &self,
        req: CreateStructureRequest,
    ) -> Result<StructureView, StructureApiError> {
        let view = self
            .usecase
            .create_structure(CreateStructure {
                name: req.name,
                level: to_domain_level(req.level),
                project_code: req.project_code,
                template_id: req.template_id,
            })
            .await?;
        Ok(view.into())
    }

    async fn get_structure(&self, id: i64) -> Result<StructureView, StructureApiError> {
        let view = self.usecase.get_structure(id).await?;
        Ok(view.into())
    }

    async fn get_structure_meta(&self, id: i64) -> Result<StructureMetaView, StructureApiError> {
        let meta = self.usecase.get_structure_meta(id).await?;
        Ok(meta.into())
    }

    async fn list_structures(
        &self,
        req: ListStructuresRequest,
    ) -> Result<StructureList, StructureApiError> {
        let summaries = self
            .usecase
            .list_structures(req.level.map(to_domain_level), req.project_code.as_deref())
            .await?;
        Ok(StructureList {
            structures: summaries.into_iter().map(Into::into).collect(),
        })
    }

    async fn rename_structure(
        &self,
        req: RenameStructureRequest,
    ) -> Result<StructureView, StructureApiError> {
        let view = self
            .usecase
            .rename_structure(RenameStructure {
                id: req.id,
                name: req.name,
            })
            .await?;
        Ok(view.into())
    }

    async fn delete_structure(&self, id: i64) -> Result<(), StructureApiError> {
        self.usecase.delete_structure(id).await?;
        Ok(())
    }

    async fn add_node(&self, req: AddNodeRequest) -> Result<NodeView, StructureApiError> {
        let view = self
            .usecase
            .add_node(AddNode {
                structure_id: req.structure_id,
                entry_name: req.entry_name,
                description: req.description,
                directory: req.directory,
                parent: req.parent,
                reference: req.reference,
                migration: req.migration.map(to_domain_migration),
                hidden: req.hidden,
            })
            .await?;
        Ok(view.into())
    }

    async fn update_node(&self, req: UpdateNodeRequest) -> Result<NodeView, StructureApiError> {
        let view = self
            .usecase
            .update_node(UpdateNode {
                structure_id: req.structure_id,
                node_id: req.node_id,
                entry_name: req.entry_name,
                description: req.description,
                directory: req.directory,
                parent: req.parent,
                reference: req.reference,
                migration: req.migration.map(to_domain_migration),
                hidden: req.hidden,
            })
            .await?;
        Ok(view.into())
    }

    async fn delete_node(&self, req: DeleteNodeRequest) -> Result<(), StructureApiError> {
        self.usecase
            .delete_node(DeleteNode {
                structure_id: req.structure_id,
                node_id: req.node_id,
            })
            .await?;
        Ok(())
    }

    async fn list_own_nodes(&self, structure_id: i64) -> Result<OwnNodeList, StructureApiError> {
        let own = self.usecase.list_own_nodes(structure_id).await?;
        Ok(OwnNodeList {
            nodes: own.into_iter().map(Into::into).collect(),
        })
    }

    async fn build_structure(&self, id: i64) -> Result<StructureTreeView, StructureApiError> {
        let tree = self.usecase.build_structure(id).await?;
        Ok(tree.into())
    }
}

// ---- translations ----

fn to_domain_level(level: StructureLevel) -> DomainLevel {
    match level {
        StructureLevel::Template => DomainLevel::Template,
        StructureLevel::Project => DomainLevel::Project,
    }
}

fn to_api_level(level: DomainLevel) -> StructureLevel {
    match level {
        DomainLevel::Template => StructureLevel::Template,
        DomainLevel::Project => StructureLevel::Project,
    }
}

fn to_domain_migration(migration: MigrationView) -> Migration {
    Migration {
        directory: migration.directory,
        filter: migration.filter,
    }
}

fn to_api_migration(migration: crate::usecase::MigrationView) -> MigrationView {
    MigrationView {
        directory: migration.directory,
        filter: migration.filter,
    }
}

impl From<InternalStructureView> for StructureView {
    fn from(v: InternalStructureView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            level: to_api_level(v.level),
            project_code: v.project_code,
            template_id: v.template_id,
            nodes: v.nodes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<InternalSummaryView> for StructureSummaryView {
    fn from(v: InternalSummaryView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            level: to_api_level(v.level),
            project_code: v.project_code,
            template_id: v.template_id,
        }
    }
}

impl From<InternalMetaView> for StructureMetaView {
    fn from(v: InternalMetaView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            level: to_api_level(v.level),
            project_code: v.project_code,
            template_id: v.template_id,
        }
    }
}

impl From<InternalTreeView> for StructureTreeView {
    fn from(v: InternalTreeView) -> Self {
        Self {
            structure_id: v.structure_id,
            nodes: v.nodes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<InternalNodeView> for NodeView {
    fn from(v: InternalNodeView) -> Self {
        Self {
            node_id: v.node_id,
            description: v.description,
            entry_name: v.entry_name,
            directory: v.directory,
            parent: v.parent,
            reference: v.reference,
            migration: v.migration.map(to_api_migration),
            children: v.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::OwnNodeView> for OwnNodeView {
    fn from(v: crate::usecase::OwnNodeView) -> Self {
        Self {
            node_id: v.node_id,
            description: v.description,
            entry_name: v.entry_name,
            directory: v.directory,
            parent: v.parent,
            reference: v.reference,
            migration: v.migration.map(to_api_migration),
            hidden: v.hidden,
            overrides_template: v.overrides_template,
        }
    }
}

/// The usecase error → api error mapping.
///
/// The domain's validation-only variants become `Validation`; the
/// runtime variants keep their own identity so the HTTP layer can pick
/// a status code. Anything unmapped becomes a `Repository` error — a
/// 500, never a panic.
impl From<UsecaseError> for StructureApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => StructureApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => StructureApiError::NotFound,
                DomainError::StructureNotFound(id) => StructureApiError::StructureNotFound(id),
                DomainError::NodeNotFound(node_id) => StructureApiError::NodeNotFound(node_id),
                DomainError::TemplateNotFound(id) => StructureApiError::TemplateNotFound(id),
                DomainError::ProjectNotFound(code) => StructureApiError::ProjectNotFound(code),
                DomainError::DuplicateName(name) => StructureApiError::DuplicateName(name),
                DomainError::DuplicateProjectStructure(code) => {
                    StructureApiError::DuplicateProjectStructure(code)
                }
                DomainError::TemplateInUse(id) => StructureApiError::TemplateInUse(id),
                other => StructureApiError::Repository(other.to_string()),
            },
        }
    }
}
