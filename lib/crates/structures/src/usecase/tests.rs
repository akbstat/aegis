//! In-memory fakes for the three ports, plus the usecase's own tests.
//!
//! The fakes live here so the usecase tests and the facade tests can
//! share one implementation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::{
    DomainError, Node, NodeRepository, ProjectLookup, Structure, StructureFilter, StructureLevel,
    StructureNew, StructureRepository,
};
use crate::usecase::{
    AddNode, CreateStructure, DeleteNode, RenameStructure, StructureUsecase,
    StructureUsecaseConfig, StructureView, UpdateNode, UsecaseError,
};

// ---- fake structure repository ----

#[derive(Clone, Default)]
pub(crate) struct FakeStructureRepo {
    inner: Arc<Mutex<FakeStructureState>>,
}

#[derive(Default)]
struct FakeStructureState {
    structures: HashMap<i64, Structure>,
    next_id: AtomicI64,
}

impl FakeStructureRepo {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Insert a structure directly, bypassing the usecase.
    pub(crate) fn put(&self, mut structure: Structure) -> Structure {
        let mut state = self.inner.lock().unwrap();
        if structure.id == 0 {
            // Skip ids already taken by a hand-placed structure.
            let mut candidate = state.next_id.fetch_add(1, Ordering::SeqCst) + 1;
            while state.structures.contains_key(&candidate) {
                candidate = state.next_id.fetch_add(1, Ordering::SeqCst) + 1;
            }
            structure.id = candidate;
        }
        state.structures.insert(structure.id, structure.clone());
        structure
    }

    pub(crate) fn get(&self, id: i64) -> Option<Structure> {
        self.inner.lock().unwrap().structures.get(&id).cloned()
    }

    pub(crate) fn count(&self) -> usize {
        self.inner.lock().unwrap().structures.len()
    }
}

#[async_trait]
impl StructureRepository for FakeStructureRepo {
    async fn create(&self, input: StructureNew) -> Result<Structure, DomainError> {
        let structure = Structure::new(
            input.name,
            input.rootpath,
            input.level,
            input.project_code,
            input.template_id,
        )?;
        Ok(self.put(structure))
    }

    async fn find_by_id(&self, id: i64) -> Result<Structure, DomainError> {
        self.get(id).ok_or(DomainError::StructureNotFound(id))
    }

    async fn list(&self, filter: StructureFilter) -> Result<Vec<Structure>, DomainError> {
        let state = self.inner.lock().unwrap();
        let mut out: Vec<Structure> = state
            .structures
            .values()
            .filter(|s| filter.level.is_none_or(|l| s.level == l))
            .filter(|s| {
                filter
                    .project_code
                    .as_ref()
                    .is_none_or(|code| s.project_code.as_ref() == Some(code))
            })
            .cloned()
            .collect();
        out.sort_by_key(|s| s.id);
        Ok(out)
    }

    async fn rename(&self, id: i64, name: &str) -> Result<Structure, DomainError> {
        let mut state = self.inner.lock().unwrap();
        let structure = state
            .structures
            .get_mut(&id)
            .ok_or(DomainError::StructureNotFound(id))?;
        structure.name = name.to_string();
        Ok(structure.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut state = self.inner.lock().unwrap();
        state
            .structures
            .remove(&id)
            .map(|_| ())
            .ok_or(DomainError::StructureNotFound(id))
    }

    async fn count_project_structures(&self, project_code: &str) -> Result<i64, DomainError> {
        let state = self.inner.lock().unwrap();
        Ok(state
            .structures
            .values()
            .filter(|s| s.level == StructureLevel::Project)
            .filter(|s| s.project_code.as_deref() == Some(project_code))
            .count() as i64)
    }

    async fn is_template_referenced(&self, template_id: i64) -> Result<bool, DomainError> {
        let state = self.inner.lock().unwrap();
        Ok(state
            .structures
            .values()
            .any(|s| s.template_id == Some(template_id)))
    }
}

// ---- fake node repository ----

#[derive(Clone, Default)]
pub(crate) struct FakeNodeRepo {
    inner: Arc<Mutex<HashMap<(i64, String), Node>>>,
}

impl FakeNodeRepo {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn put(&self, structure_id: i64, node: Node) {
        self.inner
            .lock()
            .unwrap()
            .insert((structure_id, node.node_id.clone()), node);
    }

    pub(crate) fn get(&self, structure_id: i64, node_id: &str) -> Option<Node> {
        self.inner
            .lock()
            .unwrap()
            .get(&(structure_id, node_id.to_string()))
            .cloned()
    }

    pub(crate) fn all(&self, structure_id: i64) -> Vec<Node> {
        let state = self.inner.lock().unwrap();
        let mut out: Vec<Node> = state
            .iter()
            .filter(|((sid, _), _)| *sid == structure_id)
            .map(|(_, node)| node.clone())
            .collect();
        out.sort_by(|a, b| a.entry_name.cmp(&b.entry_name));
        out
    }
}

#[async_trait]
impl NodeRepository for FakeNodeRepo {
    async fn insert(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError> {
        self.put(structure_id, node.clone());
        Ok(node.clone())
    }

    async fn update(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError> {
        let mut state = self.inner.lock().unwrap();
        let key = (structure_id, node.node_id.clone());
        if !state.contains_key(&key) {
            return Err(DomainError::NodeNotFound(node.node_id.clone()));
        }
        state.insert(key, node.clone());
        Ok(node.clone())
    }

    async fn delete(&self, structure_id: i64, node_id: &str) -> Result<(), DomainError> {
        let mut state = self.inner.lock().unwrap();
        state
            .remove(&(structure_id, node_id.to_string()))
            .map(|_| ())
            .ok_or_else(|| DomainError::NodeNotFound(node_id.to_string()))
    }

    async fn find_by_id(
        &self,
        structure_id: i64,
        node_id: &str,
    ) -> Result<Option<Node>, DomainError> {
        Ok(self.get(structure_id, node_id))
    }

    async fn list(&self, structure_id: i64) -> Result<Vec<Node>, DomainError> {
        Ok(self.all(structure_id))
    }

    async fn find_by_name(
        &self,
        structure_id: i64,
        parent_id: Option<&str>,
        name: &str,
    ) -> Result<Option<Node>, DomainError> {
        Ok(self
            .all(structure_id)
            .into_iter()
            .find(|n| n.parent.as_deref() == parent_id && n.entry_name == name))
    }
}

// ---- fake project lookup ----

#[derive(Clone, Default)]
pub(crate) struct FakeProjectLookup {
    codes: Arc<Mutex<Vec<String>>>,
}

impl FakeProjectLookup {
    pub(crate) fn new(codes: &[&str]) -> Self {
        Self {
            codes: Arc::new(Mutex::new(codes.iter().map(|c| c.to_string()).collect())),
        }
    }
}

#[async_trait]
impl ProjectLookup for FakeProjectLookup {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if self.codes.lock().unwrap().iter().any(|c| c == code) {
            Ok(())
        } else {
            Err(DomainError::ProjectNotFound(code.to_string()))
        }
    }
}

// ---- helpers ----

/// A usecase wired to the in-memory fakes, plus the fakes themselves so
/// a test can inspect what was stored.
pub(crate) struct Harness {
    pub usecase: StructureUsecase<FakeStructureRepo, FakeNodeRepo, FakeProjectLookup>,
    pub structures: FakeStructureRepo,
    pub nodes: FakeNodeRepo,
    pub projects: FakeProjectLookup,
}

pub(crate) fn harness(project_codes: &[&str]) -> Harness {
    let structures = FakeStructureRepo::new();
    let nodes = FakeNodeRepo::new();
    let projects = FakeProjectLookup::new(project_codes);
    let usecase = StructureUsecase::new(StructureUsecaseConfig {
        structures: structures.clone(),
        nodes: nodes.clone(),
        projects: Arc::new(projects.clone()),
    });
    Harness {
        usecase,
        structures,
        nodes,
        projects,
    }
}

/// A node with only the fields a test cares about.
pub(crate) fn node(node_id: &str, entry_name: &str, directory: bool, parent: Option<&str>) -> Node {
    Node {
        node_id: node_id.to_string(),
        description: String::new(),
        entry_name: entry_name.to_string(),
        hidden: None,
        directory,
        parent: parent.map(str::to_string),
        reference: None,
        migration: None,
    }
}

/// A template structure with one node under it.
pub(crate) fn template_with_child() -> Structure {
    let mut structure = Structure::new(
        "Standard".into(),
        "/abs/standard".into(),
        StructureLevel::Template,
        None,
        None,
    )
    .unwrap();
    structure.id = 1;
    structure.nodes = vec![node("t_root", "raw", true, None)];
    structure
}

// ---- tests ----

#[tokio::test]
async fn creating_a_template_structure_stores_it_with_no_owner() {
    let h = harness(&[]);
    let view = h
        .usecase
        .create_structure(CreateStructure {
            name: "Standard".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Template,
            project_code: None,
            template_id: None,
        })
        .await
        .unwrap();
    assert_eq!(view.level, StructureLevel::Template);
    assert_eq!(view.project_code, None);
    assert_eq!(view.template_id, None);
    assert_eq!(view.rootpath, "/abs/root");
    assert_eq!(h.structures.count(), 1);
}

#[tokio::test]
async fn creating_a_project_structure_without_a_template_is_rejected() {
    let h = harness(&["ABC"]);
    let err = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::MissingTemplate)
    ));
}

#[tokio::test]
async fn creating_a_project_structure_without_a_project_code_is_rejected() {
    let h = harness(&["ABC"]);
    let err = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: None,
            template_id: Some(1),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::MissingProjectCode)
    ));
}

#[tokio::test]
async fn creating_a_template_structure_with_an_owner_is_rejected() {
    let h = harness(&[]);
    let err = h
        .usecase
        .create_structure(CreateStructure {
            name: "Standard".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Template,
            project_code: Some("ABC".into()),
            template_id: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::UnexpectedOwner)
    ));
}

#[tokio::test]
async fn creating_a_project_structure_for_an_unknown_project_is_rejected() {
    let h = harness(&[]);
    let err = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("NOPE".into()),
            template_id: Some(1),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::ProjectNotFound(code)) if code == "NOPE"
    ));
}

#[tokio::test]
async fn creating_a_project_structure_pointing_at_a_missing_template_is_rejected() {
    let h = harness(&["ABC"]);
    let err = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(404),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::TemplateNotFound(404))
    ));
}

#[tokio::test]
async fn creating_a_project_structure_pointing_at_a_project_structure_is_rejected() {
    let h = harness(&["ABC", "XYZ"]);
    let template = h.structures.put(template_with_child());
    let project = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let err = h
        .usecase
        .create_structure(CreateStructure {
            name: "Other".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("XYZ".into()),
            template_id: Some(project.id),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::NotATemplate(_))
    ));
}

#[tokio::test]
async fn creating_a_second_project_structure_for_the_same_project_is_rejected() {
    let h = harness(&["ABC"]);
    let template = h.structures.put(template_with_child());
    h.usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let err = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study again".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::DuplicateProjectStructure(code)) if code == "ABC"
    ));
}

#[tokio::test]
async fn renaming_a_structure_changes_only_its_name() {
    let h = harness(&["ABC"]);
    let template = h.structures.put(template_with_child());
    let project = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let renamed = h
        .usecase
        .rename_structure(RenameStructure {
            id: project.id,
            name: "Study renamed".into(),
        })
        .await
        .unwrap();
    assert_eq!(renamed.name, "Study renamed");
    assert_eq!(renamed.level, StructureLevel::Project);
    assert_eq!(renamed.project_code.as_deref(), Some("ABC"));
    assert_eq!(renamed.template_id, Some(template.id));
}

#[tokio::test]
async fn renaming_a_structure_with_a_blank_name_is_rejected() {
    let h = harness(&[]);
    let template = h.structures.put(template_with_child());
    let err = h
        .usecase
        .rename_structure(RenameStructure {
            id: template.id,
            name: "  ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyName)
    ));
}

#[tokio::test]
async fn deleting_a_template_that_is_still_referenced_is_rejected() {
    let h = harness(&["ABC"]);
    let template = h.structures.put(template_with_child());
    h.usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let err = h.usecase.delete_structure(template.id).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::TemplateInUse(id)) if id == template.id
    ));
}

#[tokio::test]
async fn deleting_an_unreferenced_template_succeeds() {
    let h = harness(&[]);
    let template = h.structures.put(template_with_child());
    h.usecase.delete_structure(template.id).await.unwrap();
    assert_eq!(h.structures.count(), 0);
}

#[tokio::test]
async fn building_a_template_structure_returns_its_own_nodes_as_a_tree() {
    let h = harness(&[]);
    let template = h.structures.put(template_with_child());
    h.nodes.put(template.id, node("t_root", "raw", true, None));
    let tree = h.usecase.build_structure(template.id).await.unwrap();
    assert_eq!(tree.structure_id, template.id);
    assert_eq!(tree.nodes.len(), 1);
    assert_eq!(tree.nodes[0].entry_name, "raw");
    assert!(tree.nodes[0].children.is_empty());
}

#[tokio::test]
async fn building_a_project_structure_returns_the_merged_tree() {
    let h = harness(&["ABC"]);
    let template = h.structures.put(template_with_child());
    h.nodes.put(template.id, node("t_root", "raw", true, None));
    h.nodes
        .put(template.id, node("t_file", "a.txt", false, Some("t_root")));
    let project = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    h.nodes
        .put(project.id, node("n_new", "b.txt", false, Some("t_root")));
    let tree = h.usecase.build_structure(project.id).await.unwrap();
    assert_eq!(tree.nodes.len(), 1);
    let root = &tree.nodes[0];
    assert_eq!(root.entry_name, "raw");
    // the template's child and the project's own addition are both there
    let mut children: Vec<String> = root.children.iter().map(|c| c.entry_name.clone()).collect();
    children.sort();
    assert_eq!(children, vec!["a.txt".to_string(), "b.txt".to_string()]);
}

#[tokio::test]
async fn getting_a_structure_returns_its_effective_tree() {
    let h = harness(&["ABC"]);
    let template = h.structures.put(template_with_child());
    h.nodes.put(template.id, node("t_root", "raw", true, None));
    let project = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let view = h.usecase.get_structure(project.id).await.unwrap();
    assert_eq!(view.id, project.id);
    assert_eq!(view.nodes.len(), 1);
    assert_eq!(view.nodes[0].node_id, "t_root");
}

#[tokio::test]
async fn getting_a_structure_meta_returns_the_level_without_the_tree() {
    let h = harness(&["ABC"]);
    let template = h.structures.put(template_with_child());
    let project = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let meta = h.usecase.get_structure_meta(project.id).await.unwrap();
    assert_eq!(meta.level, StructureLevel::Project);
    assert_eq!(meta.template_id, Some(template.id));
    assert_eq!(meta.project_code.as_deref(), Some("ABC"));
}

#[tokio::test]
async fn getting_a_missing_structure_is_rejected() {
    let h = harness(&[]);
    let err = h.usecase.get_structure(404).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::StructureNotFound(404))
    ));
}

#[tokio::test]
async fn listing_structures_filters_by_level_and_project() {
    let h = harness(&["ABC"]);
    let template = h.structures.put(template_with_child());
    h.usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let all = h.usecase.list_structures(None, None).await.unwrap();
    assert_eq!(all.len(), 2);
    let templates = h
        .usecase
        .list_structures(Some(StructureLevel::Template), None)
        .await
        .unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].id, template.id);
    let projects = h
        .usecase
        .list_structures(Some(StructureLevel::Project), Some("ABC"))
        .await
        .unwrap();
    assert_eq!(projects.len(), 1);
}

// ---- node writes ----

/// A template holding `raw` > `raw/a.txt`, plus a project structure
/// extending it. Returns `(template, project)`.
async fn template_and_project(h: &Harness) -> (Structure, StructureView) {
    let template = h.structures.put(template_with_child());
    h.nodes.put(template.id, node("t_root", "raw", true, None));
    h.nodes
        .put(template.id, node("t_file", "a.txt", false, Some("t_root")));
    let project = h
        .usecase
        .create_structure(CreateStructure {
            name: "Study".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    (template, project)
}

/// An `AddNode` command with the boring fields filled in.
fn add(structure_id: i64, entry_name: &str, parent: Option<&str>) -> AddNode {
    AddNode {
        structure_id,
        entry_name: entry_name.to_string(),
        description: String::new(),
        directory: false,
        parent: parent.map(str::to_string),
        reference: None,
        migration: None,
        hidden: None,
    }
}

/// An `UpdateNode` command with the boring fields filled in.
fn update(structure_id: i64, node_id: &str, entry_name: &str, parent: Option<&str>) -> UpdateNode {
    UpdateNode {
        structure_id,
        node_id: node_id.to_string(),
        entry_name: entry_name.to_string(),
        description: String::new(),
        directory: false,
        parent: parent.map(str::to_string),
        reference: None,
        migration: None,
        hidden: None,
    }
}

#[tokio::test]
async fn adding_a_node_with_a_new_name_mints_a_node_id() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.usecase
        .add_node(add(p.id, "b.txt", Some("t_root")))
        .await
        .unwrap();
    let own = h.nodes.all(p.id);
    assert_eq!(own.len(), 1);
    assert!(!own[0].node_id.is_empty());
    assert_ne!(own[0].node_id, "t_root");
}

#[tokio::test]
async fn adding_a_node_with_a_name_the_template_uses_overrides_the_inherited_node() {
    // Review Focus #5
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let mut cmd = add(p.id, "raw", None);
    cmd.directory = true;
    cmd.description = "project raw".into();
    h.usecase.add_node(cmd).await.unwrap();
    let own = h.nodes.all(p.id);
    assert_eq!(own.len(), 1);
    // the template's node_id is reused, so the merge overrides it
    assert_eq!(own[0].node_id, "t_root");
    assert_eq!(own[0].description, "project raw");
}

#[tokio::test]
async fn adding_a_node_with_a_name_this_structure_already_records_is_rejected() {
    // Review Focus #5
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.nodes
        .put(p.id, node("n_own", "c.txt", false, Some("t_root")));
    let err = h
        .usecase
        .add_node(add(p.id, "c.txt", Some("t_root")))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::DuplicateName(name)) if name == "c.txt"
    ));
}

#[tokio::test]
async fn the_same_name_may_be_used_under_two_different_parents() {
    // A name is unique among the *siblings*, not across the structure:
    // the same file name may live under two different directories.
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.nodes.put(p.id, node("n_doc", "docs", true, None));
    h.usecase
        .add_node(add(p.id, "notes.txt", Some("n_doc")))
        .await
        .unwrap();
    h.usecase
        .add_node(add(p.id, "notes.txt", Some("t_root")))
        .await
        .unwrap();
    let own = h.nodes.all(p.id);
    let mut parents: Vec<String> = own
        .iter()
        .filter(|n| n.entry_name == "notes.txt")
        .map(|n| n.parent.clone().unwrap_or_default())
        .collect();
    parents.sort();
    assert_eq!(parents, vec!["n_doc".to_string(), "t_root".to_string()]);
}

#[tokio::test]
async fn a_name_the_project_renamed_away_is_free_again() {
    // The project renamed the inherited `raw`, so the template's
    // original name is not in the effective tree any more — and may be
    // used again.
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let mut cmd = update(p.id, "t_root", "renamed", None);
    cmd.directory = true;
    h.usecase.update_node(cmd).await.unwrap();
    h.usecase.add_node(add(p.id, "raw", None)).await.unwrap();
    let own = h.nodes.all(p.id);
    assert_eq!(own.len(), 2);
    let mut names: Vec<String> = own.iter().map(|n| n.entry_name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["raw".to_string(), "renamed".to_string()]);
}

#[tokio::test]
async fn adding_a_node_with_an_unknown_parent_is_rejected() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let err = h
        .usecase
        .add_node(add(p.id, "b.txt", Some("t_missing")))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::ParentNotFound(id)) if id == "t_missing"
    ));
}

#[tokio::test]
async fn adding_a_node_whose_parent_is_a_file_is_rejected() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let err = h
        .usecase
        .add_node(add(p.id, "b.txt", Some("t_file")))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::ParentNotADirectory(id)) if id == "t_file"
    ));
}

#[tokio::test]
async fn hiding_a_node_that_overrides_nothing_is_rejected() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let mut cmd = add(p.id, "b.txt", Some("t_root"));
    cmd.hidden = Some(true);
    let err = h.usecase.add_node(cmd).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::NothingToHide)
    ));
}

#[tokio::test]
async fn hiding_a_node_on_a_template_structure_is_rejected() {
    let h = harness(&[]);
    let template = h.structures.put(template_with_child());
    let mut cmd = add(template.id, "b.txt", None);
    cmd.hidden = Some(true);
    let err = h.usecase.add_node(cmd).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::HiddenNotAllowed)
    ));
}

#[tokio::test]
async fn updating_an_inherited_node_creates_an_override_record() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let mut cmd = update(p.id, "t_root", "raw", None);
    cmd.directory = true;
    cmd.description = "project raw".into();
    h.usecase.update_node(cmd).await.unwrap();
    let own = h.nodes.all(p.id);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].node_id, "t_root");
    assert_eq!(own[0].description, "project raw");
}

#[tokio::test]
async fn updating_an_unknown_node_is_rejected() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let err = h
        .usecase
        .update_node(update(p.id, "t_missing", "x", None))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NodeNotFound(id)) if id == "t_missing"
    ));
}

#[tokio::test]
async fn updating_a_node_to_a_name_another_node_uses_is_rejected() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.nodes
        .put(p.id, node("n_own", "c.txt", false, Some("t_root")));
    let err = h
        .usecase
        .update_node(update(p.id, "n_own", "a.txt", Some("t_root")))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::DuplicateName(name)) if name == "a.txt"
    ));
}

#[tokio::test]
async fn updating_a_node_under_its_own_descendant_is_rejected() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.nodes
        .put(p.id, node("n_child", "sub", true, Some("t_root")));
    h.nodes
        .put(p.id, node("n_grand", "deep.txt", false, Some("n_child")));
    // `t_root` cannot become a child of its own descendant `n_child`
    let err = h
        .usecase
        .update_node(update(p.id, "t_root", "raw", Some("n_child")))
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Validation(DomainError::Cycle)));
}

#[tokio::test]
async fn renaming_an_inherited_node_does_not_resurrect_the_template_original() {
    // Review Focus #1
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let mut cmd = update(p.id, "t_root", "renamed", None);
    cmd.directory = true;
    h.usecase.update_node(cmd).await.unwrap();
    let tree = h.usecase.build_structure(p.id).await.unwrap();
    assert_eq!(tree.nodes.len(), 1);
    assert_eq!(tree.nodes[0].entry_name, "renamed");
}

#[tokio::test]
async fn deleting_a_recorded_node_removes_it_and_its_descendants() {
    // Review Focus #3
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    // the project records `raw` (an override) and a file under it
    h.nodes.put(p.id, node("t_root", "raw", true, None));
    h.nodes
        .put(p.id, node("n_new", "b.txt", false, Some("t_root")));
    h.usecase
        .delete_node(DeleteNode {
            structure_id: p.id,
            node_id: "t_root".into(),
        })
        .await
        .unwrap();
    let own = h.nodes.all(p.id);
    // the override row survives, hidden; the plain addition is gone
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].node_id, "t_root");
    assert_eq!(own[0].hidden, Some(true));
    assert!(h.nodes.get(p.id, "n_new").is_none());
    // and the node is gone from the effective tree
    assert!(
        h.usecase
            .build_structure(p.id)
            .await
            .unwrap()
            .nodes
            .is_empty()
    );
}

#[tokio::test]
async fn deleting_an_inherited_node_hides_it() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.usecase
        .delete_node(DeleteNode {
            structure_id: p.id,
            node_id: "t_file".into(),
        })
        .await
        .unwrap();
    let own = h.nodes.all(p.id);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].node_id, "t_file");
    assert_eq!(own[0].hidden, Some(true));
    let tree = h.usecase.build_structure(p.id).await.unwrap();
    assert!(tree.nodes[0].children.is_empty());
}

#[tokio::test]
async fn deleting_an_overridden_node_hides_it_rather_than_reverting_it() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.nodes.put(p.id, node("t_root", "raw", true, None));
    h.usecase
        .delete_node(DeleteNode {
            structure_id: p.id,
            node_id: "t_root".into(),
        })
        .await
        .unwrap();
    // the row is hidden, not deleted — deleting it would bring the
    // template's version straight back
    let own = h.nodes.all(p.id);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].hidden, Some(true));
    assert!(
        h.usecase
            .build_structure(p.id)
            .await
            .unwrap()
            .nodes
            .is_empty()
    );
}

#[tokio::test]
async fn a_hidden_record_is_still_addressable_so_a_hide_can_be_undone() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.usecase
        .delete_node(DeleteNode {
            structure_id: p.id,
            node_id: "t_root".into(),
        })
        .await
        .unwrap();
    // undo the hide
    let mut cmd = update(p.id, "t_root", "raw", None);
    cmd.directory = true;
    cmd.hidden = Some(false);
    h.usecase.update_node(cmd).await.unwrap();
    let own = h.nodes.all(p.id);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].hidden, Some(false));
    let tree = h.usecase.build_structure(p.id).await.unwrap();
    assert_eq!(tree.nodes.len(), 1);
    assert_eq!(tree.nodes[0].entry_name, "raw");
}

#[tokio::test]
async fn deleting_an_unknown_node_is_rejected() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    let err = h
        .usecase
        .delete_node(DeleteNode {
            structure_id: p.id,
            node_id: "t_missing".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NodeNotFound(id)) if id == "t_missing"
    ));
}

#[tokio::test]
async fn listing_own_nodes_reports_the_hidden_flag_and_the_override_flag() {
    let h = harness(&["ABC"]);
    let (_t, p) = template_and_project(&h).await;
    h.nodes.put(p.id, node("t_root", "raw", true, None));
    h.nodes
        .put(p.id, node("n_new", "b.txt", false, Some("t_root")));
    let own = h.usecase.list_own_nodes(p.id).await.unwrap();
    assert_eq!(own.len(), 2);
    let root = own.iter().find(|n| n.node_id == "t_root").unwrap();
    assert!(root.overrides_template);
    assert!(!root.hidden);
    let added = own.iter().find(|n| n.node_id == "n_new").unwrap();
    assert!(!added.overrides_template);
}
