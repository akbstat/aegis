//! The facade wired on top of the in-memory ports.

use std::sync::Arc;

use apis::structures::{
    AddNodeRequest, CreateStructureRequest, DeleteNodeRequest, ListStructuresRequest,
    RenameStructureRequest, StructureApiError, StructureLevel, StructureService, UpdateNodeRequest,
};

use crate::adapter::facade::internal::service::StructureServiceImpl;
use crate::domain::StructureLevel as DomainLevel;
use crate::usecase::StructureUsecase;
use crate::usecase::tests::{Harness, harness, node};

fn service(
    h: &Harness,
) -> StructureServiceImpl<
    crate::usecase::tests::FakeStructureRepo,
    crate::usecase::tests::FakeNodeRepo,
    crate::usecase::tests::FakeProjectLookup,
> {
    StructureServiceImpl::new(StructureUsecase::new(
        crate::usecase::StructureUsecaseConfig {
            structures: h.structures.clone(),
            nodes: h.nodes.clone(),
            projects: Arc::new(h.projects.clone()),
        },
    ))
}

#[tokio::test]
async fn the_facade_round_trips_a_template_structure() {
    let h = harness(&[]);
    let svc = service(&h);
    let created = svc
        .create_structure(CreateStructureRequest {
            name: "Standard".into(),
            level: StructureLevel::Template,
            project_code: None,
            template_id: None,
        })
        .await
        .unwrap();
    assert_eq!(created.level, StructureLevel::Template);
    assert!(created.nodes.is_empty());

    let fetched = svc.get_structure(created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "Standard");
}

#[tokio::test]
async fn the_facade_translates_the_structure_level_both_ways() {
    let h = harness(&["ABC"]);
    let svc = service(&h);
    let created = svc
        .create_structure(CreateStructureRequest {
            name: "Study".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(1),
        })
        .await;
    // the template id 1 does not exist, so this is rejected — but the
    // level reached the usecase as a project structure.
    assert!(created.is_err());

    let template = h
        .structures
        .put(crate::usecase::tests::template_with_child());
    let project = svc
        .create_structure(CreateStructureRequest {
            name: "Study".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    assert_eq!(project.level, StructureLevel::Project);
    assert_eq!(project.template_id, Some(template.id));

    let meta = svc.get_structure_meta(project.id).await.unwrap();
    assert_eq!(meta.level, StructureLevel::Project);
}

#[tokio::test]
async fn the_facade_maps_a_duplicate_name_to_the_duplicate_name_error() {
    let h = harness(&["ABC"]);
    let svc = service(&h);
    let template = h
        .structures
        .put(crate::usecase::tests::template_with_child());
    h.nodes.put(template.id, node("t_root", "raw", true, None));
    let project = svc
        .create_structure(CreateStructureRequest {
            name: "Study".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    h.nodes
        .put(project.id, node("n_own", "c.txt", false, Some("t_root")));
    let err = svc
        .add_node(AddNodeRequest {
            structure_id: project.id,
            entry_name: "c.txt".into(),
            description: String::new(),
            directory: false,
            parent: Some("t_root".into()),
            reference: None,
            migration: None,
            hidden: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StructureApiError::DuplicateName(name) if name == "c.txt"
    ));
}

#[tokio::test]
async fn the_facade_maps_a_missing_node_to_the_node_not_found_error() {
    let h = harness(&["ABC"]);
    let svc = service(&h);
    let template = h
        .structures
        .put(crate::usecase::tests::template_with_child());
    let project = svc
        .create_structure(CreateStructureRequest {
            name: "Study".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    let err = svc
        .update_node(UpdateNodeRequest {
            structure_id: project.id,
            node_id: "t_missing".into(),
            entry_name: "x".into(),
            description: String::new(),
            directory: false,
            parent: None,
            reference: None,
            migration: None,
            hidden: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StructureApiError::NodeNotFound(id) if id == "t_missing"
    ));
}

#[tokio::test]
async fn the_facade_maps_a_validation_failure_to_the_validation_error() {
    let h = harness(&[]);
    let svc = service(&h);
    let err = svc
        .create_structure(CreateStructureRequest {
            name: "  ".into(),
            level: StructureLevel::Template,
            project_code: None,
            template_id: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, StructureApiError::Validation(_)));
}

#[tokio::test]
async fn the_facade_lists_own_nodes_with_the_hidden_and_override_flags() {
    let h = harness(&["ABC"]);
    let svc = service(&h);
    let template = h
        .structures
        .put(crate::usecase::tests::template_with_child());
    h.nodes.put(template.id, node("t_root", "raw", true, None));
    let project = svc
        .create_structure(CreateStructureRequest {
            name: "Study".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    h.nodes.put(project.id, node("t_root", "raw", true, None));
    h.nodes
        .put(project.id, node("n_new", "b.txt", false, Some("t_root")));
    let own = svc.list_own_nodes(project.id).await.unwrap();
    assert_eq!(own.nodes.len(), 2);
    let root = own.nodes.iter().find(|n| n.node_id == "t_root").unwrap();
    assert!(root.overrides_template);
    assert!(!root.hidden);
}

#[tokio::test]
async fn the_facade_deletes_a_node_and_lists_structures() {
    let h = harness(&["ABC"]);
    let svc = service(&h);
    let template = h
        .structures
        .put(crate::usecase::tests::template_with_child());
    h.nodes.put(template.id, node("t_root", "raw", true, None));
    let project = svc
        .create_structure(CreateStructureRequest {
            name: "Study".into(),
            level: StructureLevel::Project,
            project_code: Some("ABC".into()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();
    svc.delete_node(DeleteNodeRequest {
        structure_id: project.id,
        node_id: "t_root".into(),
    })
    .await
    .unwrap();
    let list = svc
        .list_structures(ListStructuresRequest {
            level: Some(StructureLevel::Project),
            project_code: Some("ABC".into()),
        })
        .await
        .unwrap();
    assert_eq!(list.structures.len(), 1);

    svc.rename_structure(RenameStructureRequest {
        id: project.id,
        name: "Renamed".into(),
    })
    .await
    .unwrap();
    let renamed = svc.get_structure(project.id).await.unwrap();
    assert_eq!(renamed.name, "Renamed");
    assert_eq!(renamed.level, StructureLevel::Project);
    let _ = DomainLevel::Template;
}
