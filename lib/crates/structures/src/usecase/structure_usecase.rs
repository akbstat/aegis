use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::domain::{
    DomainError, MergedNode, MergedTree, Node, NodeAttributes, NodeRepository, ProjectLookup,
    Structure, StructureFilter, StructureLevel, StructureNew, StructureRepository,
    effective_records, merge,
};
use crate::usecase::{
    AddNode, CreateStructure, DeleteNode, NodeView, OwnNodeView, RenameStructure,
    StructureMetaView, StructureSummaryView, StructureTreeView, StructureView, UpdateNode,
    UsecaseError,
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
            cmd.rootpath.clone(),
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
                return Err(UsecaseError::Validation(DomainError::NotATemplate(
                    template_id,
                )));
            }
            if self
                .structures
                .count_project_structures(project_code)
                .await?
                > 0
            {
                return Err(UsecaseError::Repository(
                    DomainError::DuplicateProjectStructure(project_code.to_string()),
                ));
            }
        }

        let structure = self
            .structures
            .create(StructureNew {
                name: cmd.name,
                rootpath: cmd.rootpath,
                level: cmd.level,
                project_code: cmd.project_code,
                template_id: cmd.template_id,
            })
            .await?;
        Ok(StructureView {
            id: structure.id,
            name: structure.name,
            rootpath: structure.rootpath,
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
            rootpath: effective.structure.rootpath,
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
    pub async fn get_structure_meta(&self, id: i64) -> Result<StructureMetaView, UsecaseError> {
        let structure = self.structures.find_by_id(id).await?;
        Ok(StructureMetaView {
            id: structure.id,
            name: structure.name,
            rootpath: structure.rootpath,
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
                rootpath: s.rootpath,
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

    /// Place a node under a parent.
    ///
    /// The system decides what the write refers to: a name the
    /// effective structure already uses under that parent becomes an
    /// **override** of that node, reusing its `node_id`; a name this
    /// structure already records is a conflict; a new name mints a
    /// fresh `node_id`.
    pub async fn add_node(&self, cmd: AddNode) -> Result<NodeView, UsecaseError> {
        let eff = self.effective(cmd.structure_id).await?;
        let attrs = NodeAttributes {
            description: cmd.description,
            entry_name: cmd.entry_name,
            directory: cmd.directory,
            parent: cmd.parent,
            reference: cmd.reference,
            migration: cmd.migration,
        };

        // What does this name already refer to, under this parent?
        let existing = find_sibling(&eff, attrs.parent.as_deref(), &attrs.entry_name).cloned();
        let template_ids = template_node_ids(&eff);

        let (node_id, inherited) = match existing {
            Some(node) if eff.own_nodes.iter().any(|o| o.node_id == node.node_id) => {
                // This structure already records this name under this
                // parent. The client should have used `PUT`.
                return Err(UsecaseError::Repository(DomainError::DuplicateName(
                    attrs.entry_name.clone(),
                )));
            }
            Some(node) => {
                // The template has this name under this parent: bind the
                // write to that node.
                (node.node_id.clone(), template_ids.contains(&node.node_id))
            }
            None => (nanoid::nanoid!(), false),
        };

        if cmd.hidden == Some(true) {
            if eff.structure.level == StructureLevel::Template {
                return Err(UsecaseError::Validation(DomainError::HiddenNotAllowed));
            }
            if !inherited {
                // Hiding a node that overrides nothing would leave it
                // invisible — never what the caller meant.
                return Err(UsecaseError::Validation(DomainError::NothingToHide));
            }
        }

        // The node being written is excluded from the name-uniqueness
        // check: an override deliberately reuses the name of the node
        // it overrides.
        self.validate_node_write(&eff, &attrs, Some(&node_id))?;

        let mut node = Node::new(node_id, attrs).map_err(UsecaseError::Validation)?;
        node.hidden = cmd.hidden;
        self.nodes.insert(cmd.structure_id, &node).await?;
        self.node_view(cmd.structure_id, &node.node_id).await
    }

    /// Replace a node's mutable attributes, addressed by `node_id`.
    ///
    /// A full replacement: `parent: None` moves the node to the root and
    /// `hidden: Some(false)` un-hides it. An inherited node gains an
    /// override record carrying the template's `node_id`.
    pub async fn update_node(&self, cmd: UpdateNode) -> Result<NodeView, UsecaseError> {
        let eff = self.effective(cmd.structure_id).await?;
        find_node(&eff, &cmd.node_id).ok_or_else(|| {
            UsecaseError::Repository(DomainError::NodeNotFound(cmd.node_id.clone()))
        })?;
        let recorded_here = eff.own_nodes.iter().any(|n| n.node_id == cmd.node_id);

        let attrs = NodeAttributes {
            description: cmd.description,
            entry_name: cmd.entry_name,
            directory: cmd.directory,
            parent: cmd.parent,
            reference: cmd.reference,
            migration: cmd.migration,
        };

        if cmd.hidden == Some(true) && eff.structure.level == StructureLevel::Template {
            return Err(UsecaseError::Validation(DomainError::HiddenNotAllowed));
        }

        self.validate_node_write(&eff, &attrs, Some(&cmd.node_id))?;

        let mut node = Node::new(cmd.node_id.clone(), attrs).map_err(UsecaseError::Validation)?;
        node.hidden = cmd.hidden;
        if recorded_here {
            self.nodes.update(cmd.structure_id, &node).await?;
        } else {
            self.nodes.insert(cmd.structure_id, &node).await?;
        }
        self.node_view(cmd.structure_id, &cmd.node_id).await
    }

    /// Remove a node.
    ///
    /// One meaning in every case: the node and everything under it are
    /// not part of this structure. A record that overrides a template
    /// node is **hidden** — deleting its row would revert the node to
    /// the template's version and put it straight back into the tree. A
    /// record with no template counterpart is deleted.
    pub async fn delete_node(&self, cmd: DeleteNode) -> Result<(), UsecaseError> {
        let eff = self.effective(cmd.structure_id).await?;
        let target = find_node(&eff, &cmd.node_id).cloned().ok_or_else(|| {
            UsecaseError::Repository(DomainError::NodeNotFound(cmd.node_id.clone()))
        })?;

        // The removal set: the target plus every own record beneath it.
        let mut removal: Vec<String> = vec![target.node_id.clone()];
        let mut grew = true;
        while grew {
            grew = false;
            for node in &eff.own_nodes {
                if removal.contains(&node.node_id) {
                    continue;
                }
                if let Some(parent) = node.parent.as_deref()
                    && removal.iter().any(|id| id == parent)
                {
                    removal.push(node.node_id.clone());
                    grew = true;
                }
            }
        }

        let template_ids = template_node_ids(&eff);
        for node_id in &removal {
            let own = eff
                .own_nodes
                .iter()
                .find(|n| &n.node_id == node_id)
                .cloned();
            if template_ids.contains(node_id) {
                let mut row = own.clone().unwrap_or_else(|| target.clone());
                row.hidden = Some(true);
                if own.is_some() {
                    self.nodes.update(cmd.structure_id, &row).await?;
                } else {
                    self.nodes.insert(cmd.structure_id, &row).await?;
                }
            } else {
                self.nodes.delete(cmd.structure_id, node_id).await?;
            }
        }
        Ok(())
    }

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
                entry_name: node.entry_name,
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
    pub async fn build_structure(&self, id: i64) -> Result<StructureTreeView, UsecaseError> {
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

    /// Validate a node write against the effective structure.
    ///
    /// `target_node_id` is the node being written — `None` for an add.
    /// The name-uniqueness check excludes it, so a node may keep its own
    /// name.
    fn validate_node_write(
        &self,
        eff: &Effective,
        attrs: &NodeAttributes,
        target_node_id: Option<&str>,
    ) -> Result<(), UsecaseError> {
        attrs.validate().map_err(UsecaseError::Validation)?;

        if let Some(parent_id) = attrs.parent.as_deref() {
            let parent = find_node(eff, parent_id).ok_or_else(|| {
                UsecaseError::Validation(DomainError::ParentNotFound(parent_id.to_string()))
            })?;
            if !parent.directory {
                return Err(UsecaseError::Validation(DomainError::ParentNotADirectory(
                    parent_id.to_string(),
                )));
            }
            if let Some(target_id) = target_node_id
                && would_cycle(eff, target_id, parent_id)
            {
                return Err(UsecaseError::Validation(DomainError::Cycle));
            }
        }

        // A name is unique among siblings: no other node under the same
        // parent may carry it.
        for node in records(eff).values() {
            if Some(node.node_id.as_str()) == target_node_id {
                continue;
            }
            if node.parent.as_deref() == attrs.parent.as_deref()
                && node.entry_name == attrs.entry_name
            {
                return Err(UsecaseError::Repository(DomainError::DuplicateName(
                    attrs.entry_name.clone(),
                )));
            }
        }
        Ok(())
    }

    /// The merged view of one node.
    ///
    /// A node the effective tree does not contain — a hidden one, or one
    /// whose parent is no longer reachable — is rendered on its own.
    async fn node_view(&self, structure_id: i64, node_id: &str) -> Result<NodeView, UsecaseError> {
        let eff = self.effective(structure_id).await?;
        if let Some(found) = find_in_tree(&eff.merged.roots, node_id) {
            return Ok(found.into());
        }
        let node = find_node(&eff, node_id).ok_or_else(|| {
            UsecaseError::Repository(DomainError::NodeNotFound(node_id.to_string()))
        })?;
        Ok(NodeView::from_node(node))
    }

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

/// The effective record for each `node_id`: the template's rows with
/// this structure's own rows layered on top.
///
/// A hidden own record is in here — that is how a hide is undone. The
/// *rendered* tree excludes hidden nodes, but the lookup index does
/// not: the tree answers "what does this structure look like", the
/// index answers "what does this write refer to".
fn records(eff: &Effective) -> HashMap<&str, &Node> {
    effective_records(&eff.template_nodes, &eff.own_nodes)
}

/// The effective node with that `node_id`, if the structure has one.
fn find_node<'a>(eff: &'a Effective, node_id: &str) -> Option<&'a Node> {
    records(eff)
        .values()
        .find(|n| n.node_id == node_id)
        .copied()
}

/// The effective node carrying that name under that parent, if any.
///
/// The name of a node is unique among its **siblings** — the same name
/// may appear under two different parents.
fn find_sibling<'a>(eff: &'a Effective, parent: Option<&str>, name: &str) -> Option<&'a Node> {
    records(eff)
        .values()
        .find(|n| n.parent.as_deref() == parent && n.entry_name == name)
        .copied()
}

/// The `node_id`s the template already has, so a write can tell an
/// override from an addition.
fn template_node_ids(eff: &Effective) -> HashSet<String> {
    eff.template_nodes
        .iter()
        .map(|n| n.node_id.clone())
        .collect()
}

/// Would making `new_parent_id` the parent of `target_id` create a
/// cycle?
///
/// True when `new_parent_id` is `target_id` itself, or when it sits
/// below it — walking up from the proposed parent must reach the node
/// being moved.
fn would_cycle(eff: &Effective, target_id: &str, new_parent_id: &str) -> bool {
    if new_parent_id == target_id {
        return true;
    }
    let nodes = records(eff);
    let mut current = new_parent_id.to_string();
    // The bound guards against a cycle already present in the stored
    // data, which would otherwise loop forever.
    for _ in 0..=nodes.len() {
        let Some(node) = nodes.get(current.as_str()) else {
            return false;
        };
        let Some(parent) = node.parent.as_deref() else {
            return false;
        };
        if parent == target_id {
            return true;
        }
        current = parent.to_string();
    }
    true
}

/// Find a node in a merged tree by its `node_id`.
fn find_in_tree(nodes: &[MergedNode], node_id: &str) -> Option<MergedNode> {
    for node in nodes {
        if node.node.node_id == node_id {
            return Some(node.clone());
        }
        if let Some(found) = find_in_tree(&node.children, node_id) {
            return Some(found);
        }
    }
    None
}
