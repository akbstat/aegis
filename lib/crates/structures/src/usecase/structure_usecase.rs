use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::{
    merge, DomainError, MergedTree, Node, NodeRepository, ProjectLookup, Structure,
    StructureFilter, StructureLevel, StructureNew, StructureRepository,
};
use crate::usecase::{
    CreateStructure, NodeView, OwnNodeView, RenameStructure, StructureMetaView,
    StructureSummaryView, StructureTreeView, StructureView, UsecaseError,
};

/// The usecase's collaborators.
///
/// `projects` is an `Arc<P>` rather than a `P`, so the call site can
/// hand over an `Arc<ProjectLookupImpl>` directly — the `crf` /
/// `sdtm-project` shape.
pub struct StructureUsecaseConfig<S, N, P> {
    pub structures: S,
    pub nodes: N,
    pub projects: Arc<P>,
}

/// Orchestrates CRUD over structures and over individual nodes, and
/// projects the domain aggregates into view DTOs.
///
/// Generic over the three ports so it can be exercised against
/// in-memory fakes in tests and against the PostgreSQL repositories in
/// production.
pub struct StructureUsecase<S, N, P> {
    structures: S,
    nodes: N,
    projects: Arc<P>,
}

impl<S, N, P> StructureUsecase<S, N, P>
where
    S: StructureRepository,
    N: NodeRepository,
    P: ProjectLookup,
{
    /// Build a usecase from its collaborators.
    pub fn new(config: StructureUsecaseConfig<S, N, P>) -> Self {
        Self {
            structures: config.structures,
            nodes: config.nodes,
            projects: config.projects,
        }
    }

    // ---- structures ----

    /// Create a structure.
    ///
    /// A project structure must name a project that exists and a
    /// template that is a template structure; a project may have only
    /// one structure.
    pub async fn create_structure(
        &self,
        cmd: CreateStructure,
    ) -> Result<StructureView, UsecaseError> {
        // The shape first: a template carries no owner, a project
        // structure must carry both an owner and a template.
        Structure::new(
            cmd.name.clone(),
            cmd.level,
            cmd.project_code.clone(),
            cmd.template_id,
        )
        .map_err(UsecaseError::Validation)?;

        if cmd.level == StructureLevel::Project {
            let project_code = cmd.project_code.as_deref().unwrap_or_default();
            self.projects.get_by_code(project_code).await?;
            let template_id = cmd.template_id.expect("validated above");
            let template = self
                .structures
                .find_by_id(template_id)
                .await
                .map_err(|_| DomainError::TemplateNotFound(template_id))?;
            if template.level != StructureLevel::Template {
                return Err(UsecaseError::Validation(DomainError::NotATemplate(template_id)));
            }
            if self.structures.count_project_structures(project_code).await? > 0 {
                return Err(UsecaseError::Repository(
                    DomainError::DuplicateProjectStructure(project_code.to_string()),
                ));
            }
        }

        let structure = self
            .structures
            .create(StructureNew {
                name: cmd.name,
                level: cmd.level,
                project_code: cmd.project_code,
                template_id: cmd.template_id,
            })
            .await?;
        Ok(StructureView {
            id: structure.id,
            name: structure.name,
            level: structure.level,
            project_code: structure.project_code,
            template_id: structure.template_id,
            nodes: Vec::new(),
        })
    }

    /// Fetch a structure together with its effective tree.
    pub async fn get_structure(&self, id: i64) -> Result<StructureView, UsecaseError> {
        let effective = self.effective(id).await?;
        Ok(StructureView {
            id: effective.structure.id,
            name: effective.structure.name,
            level: effective.structure.level,
            project_code: effective.structure.project_code,
            template_id: effective.structure.template_id,
            nodes: effective
                .merged
                .roots
                .into_iter()
                .map(Into::into)
                .collect::<Vec<NodeView>>(),
        })
    }

    /// Fetch a structure's identity and level, without building its tree.
    pub async fn get_structure_meta(
        &self,
        id: i64,
    ) -> Result<StructureMetaView, UsecaseError> {
        let structure = self.structures.find_by_id(id).await?;
        Ok(StructureMetaView {
            id: structure.id,
            name: structure.name,
            level: structure.level,
            project_code: structure.project_code,
            template_id: structure.template_id,
        })
    }

    /// List structures. A `None` filter matches everything.
    pub async fn list_structures(
        &self,
        level: Option<StructureLevel>,
        project_code: Option<&str>,
    ) -> Result<Vec<StructureSummaryView>, UsecaseError> {
        let structures = self
            .structures
            .list(StructureFilter {
                level,
                project_code: project_code.map(str::to_string),
            })
            .await?;
        Ok(structures
            .into_iter()
            .map(|s| StructureSummaryView {
                id: s.id,
                name: s.name,
                level: s.level,
                project_code: s.project_code,
                template_id: s.template_id,
            })
            .collect())
    }

    /// Rename a structure. Only the name is mutable.
    pub async fn rename_structure(
        &self,
        cmd: RenameStructure,
    ) -> Result<StructureView, UsecaseError> {
        if cmd.name.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyName));
        }
        self.structures.rename(cmd.id, &cmd.name).await?;
        self.get_structure(cmd.id).await
    }

    /// Delete a structure.
    ///
    /// A template that a project structure still points at cannot be
    /// deleted — the project would be left pointing at nothing.
    pub async fn delete_structure(&self, id: i64) -> Result<(), UsecaseError> {
        let structure = self.structures.find_by_id(id).await?;
        if structure.level == StructureLevel::Template
            && self.structures.is_template_referenced(id).await?
        {
            return Err(UsecaseError::Repository(DomainError::TemplateInUse(id)));
        }
        self.structures.delete(id).await?;
        Ok(())
    }

    // ---- nodes ----

    /// The structure's own records — the extension records, with their
    /// `hidden` flag and whether each overrides a template node.
    ///
    /// This is where a client learns the `node_id` of a hidden node, so
    /// it can undo a hide.
    pub async fn list_own_nodes(
        &self,
        structure_id: i64,
    ) -> Result<Vec<OwnNodeView>, UsecaseError> {
        let effective = self.effective(structure_id).await?;
        let template_ids = template_node_ids(&effective);
        Ok(effective
            .own_nodes
            .into_iter()
            .map(|node| OwnNodeView {
                hidden: node.hidden.unwrap_or(false),
                overrides_template: template_ids.contains(&node.node_id),
                node_id: node.node_id,
                description: node.description,
                path: node.path,
                directory: node.directory,
                parent: node.parent,
                reference: node.reference,
                migration: node.migration.map(Into::into),
            })
            .collect())
    }

    // ---- build ----

    /// The effective tree: the template's nodes merged with this
    /// structure's extension records.
    pub async fn build_structure(
        &self,
        id: i64,
    ) -> Result<StructureTreeView, UsecaseError> {
        let effective = self.effective(id).await?;
        Ok(StructureTreeView {
            structure_id: effective.structure.id,
            nodes: effective
                .merged
                .roots
                .into_iter()
                .map(Into::into)
                .collect::<Vec<NodeView>>(),
        })
    }

    // ---- internals ----

    /// The three node sets a read or a write is resolved against.
    async fn effective(&self, structure_id: i64) -> Result<Effective, UsecaseError> {
        let structure = self.structures.find_by_id(structure_id).await?;
        let template_nodes = match structure.template_id {
            Some(template_id) => {
                let template = self
                    .structures
                    .find_by_id(template_id)
                    .await
                    .map_err(|_| DomainError::TemplateNotFound(template_id))?;
                self.nodes.list(template.id).await?
            }
            None => Vec::new(),
        };
        let own_nodes = self.nodes.list(structure.id).await?;
        let merged = merge(&template_nodes, &own_nodes);
        Ok(Effective {
            structure,
            template_nodes,
            own_nodes,
            merged,
        })
    }
}

/// The three node sets a read or a write is resolved against.
struct Effective {
    structure: Structure,
    /// The template's nodes. Empty for a template-level structure.
    template_nodes: Vec<Node>,
    /// This structure's own rows — the extension records at project level.
    own_nodes: Vec<Node>,
    /// The merged result — what the caller sees.
    merged: MergedTree,
}

/// Every node a write may refer to: this structure's own records plus
/// the template's nodes.
///
/// A hidden own record is in here — that is how a hide is undone. The
/// *rendered* tree excludes hidden nodes, but the lookup index does
/// not: the tree answers "what does this structure look like", the
/// index answers "what does this write refer to".
// The node-write methods land in the next task; until then nothing calls
// this, and the dead-code lint would fire.
#[allow(dead_code)]
fn lookup_nodes(eff: &Effective) -> Vec<&Node> {
    eff.own_nodes
        .iter()
        .chain(eff.template_nodes.iter())
        .collect()
}

/// The `node_id`s the template already has, so a write can tell an
/// override from an addition.
fn template_node_ids(eff: &Effective) -> HashSet<String> {
    eff.template_nodes
        .iter()
        .map(|n| n.node_id.clone())
        .collect()
}
