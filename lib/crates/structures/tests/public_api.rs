//! Compile-only guard over the crate's documented public surface.
//!
//! Names every consumer import, pins the constructor chain as function
//! pointers, and asserts the trait bounds the usecase relies on. The
//! safety net for re-export / trait-bound refactors.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use structures::{
    AddNode, CreateStructure, DeleteNode, DomainError, MergedNode, MergedTree, Migration, Node,
    NodeAttributes, NodeRepository, NodeView, OwnNodeView, ProjectLookup, ProjectLookupImpl,
    RenameStructure, Structure, StructureFilter, StructureLevel, StructureMetaView, StructureNew,
    StructureRepository, StructureServiceImpl, StructureSummaryView, StructureTreeView,
    StructureView, UpdateNode, UsecaseError,
};

#[test]
fn the_documented_imports_resolve() {
    // Every name above must exist; this test exists to fail at compile
    // time when a re-export is dropped.
    let _: Option<StructureLevel> = None;
    let _: Option<Migration> = None;
    let _: Option<Node> = None;
    let _: Option<NodeAttributes> = None;
    let _: Option<Structure> = None;
    let _: Option<StructureNew> = None;
    let _: Option<StructureFilter> = None;
    let _: Option<MergedNode> = None;
    let _: Option<MergedTree> = None;
    let _: Option<NodeView> = None;
    let _: Option<OwnNodeView> = None;
    let _: Option<StructureView> = None;
    let _: Option<StructureSummaryView> = None;
    let _: Option<StructureMetaView> = None;
    let _: Option<StructureTreeView> = None;
    let _: Option<CreateStructure> = None;
    let _: Option<AddNode> = None;
    let _: Option<UpdateNode> = None;
    let _: Option<DeleteNode> = None;
    let _: Option<RenameStructure> = None;
    let _: Option<UsecaseError> = None;
    let _: Option<DomainError> = None;
}

#[test]
fn the_repositories_are_object_safe() {
    fn assert_object_safe<T: ?Sized>() {}
    assert_object_safe::<dyn StructureRepository>();
    assert_object_safe::<dyn NodeRepository>();
    assert_object_safe::<dyn ProjectLookup>();
}

#[test]
fn the_constructor_chain_is_pinned() {
    // The Postgres repositories take a pool.
    let _: fn(PgPool) -> structures::StructureRepoPg = structures::StructureRepoPg::new;
    let _: fn(PgPool) -> structures::NodeRepoPg = structures::NodeRepoPg::new;
    // The lookup takes the project service.
    let _: fn(Arc<dyn apis::project::ProjectService>) -> ProjectLookupImpl = ProjectLookupImpl::new;
}

// ---- in-memory doubles, to pin the trait bounds ----

#[derive(Default)]
struct FakeStructures;

#[async_trait]
impl StructureRepository for FakeStructures {
    async fn create(&self, _input: StructureNew) -> Result<Structure, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _id: i64) -> Result<Structure, DomainError> {
        unimplemented!()
    }
    async fn list(&self, _filter: StructureFilter) -> Result<Vec<Structure>, DomainError> {
        unimplemented!()
    }
    async fn rename(&self, _id: i64, _name: &str) -> Result<Structure, DomainError> {
        unimplemented!()
    }
    async fn delete(&self, _id: i64) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn count_project_structures(&self, _project_code: &str) -> Result<i64, DomainError> {
        unimplemented!()
    }
    async fn is_template_referenced(&self, _template_id: i64) -> Result<bool, DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct FakeNodes;

#[async_trait]
impl NodeRepository for FakeNodes {
    async fn insert(&self, _structure_id: i64, _node: &Node) -> Result<Node, DomainError> {
        unimplemented!()
    }
    async fn update(&self, _structure_id: i64, _node: &Node) -> Result<Node, DomainError> {
        unimplemented!()
    }
    async fn delete(&self, _structure_id: i64, _node_id: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn find_by_id(
        &self,
        _structure_id: i64,
        _node_id: &str,
    ) -> Result<Option<Node>, DomainError> {
        unimplemented!()
    }
    async fn list(&self, _structure_id: i64) -> Result<Vec<Node>, DomainError> {
        unimplemented!()
    }
    async fn find_by_path(
        &self,
        _structure_id: i64,
        _path: &str,
    ) -> Result<Option<Node>, DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct FakeProjects;

#[async_trait]
impl ProjectLookup for FakeProjects {
    async fn get_by_code(&self, _code: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

#[test]
fn the_usecase_and_the_facade_accept_any_port_implementations() {
    fn assert_usecase<S: StructureRepository, N: NodeRepository, P: ProjectLookup>() {}
    assert_usecase::<FakeStructures, FakeNodes, FakeProjects>();

    fn assert_facade<S: StructureRepository, N: NodeRepository, P: ProjectLookup>() {}
    assert_facade::<FakeStructures, FakeNodes, FakeProjects>();

    // And the facade is usable as the api port.
    fn assert_api_port<T: apis::structures::StructureService>() {}
    assert_api_port::<StructureServiceImpl<FakeStructures, FakeNodes, FakeProjects>>();
}
