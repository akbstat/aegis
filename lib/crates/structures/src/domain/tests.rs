use super::error::DomainError;
use super::migration::Migration;
use super::node::{Node, NodeAttributes};
use super::structure::Structure;
use super::structure_level::StructureLevel;

#[test]
fn structure_level_round_trips_through_its_str_form() {
    assert_eq!(StructureLevel::Template.as_str(), "template");
    assert_eq!(StructureLevel::Project.as_str(), "project");
    assert_eq!(
        StructureLevel::try_from("template").unwrap(),
        StructureLevel::Template
    );
    assert_eq!(
        StructureLevel::try_from("project").unwrap(),
        StructureLevel::Project
    );
    assert!(StructureLevel::try_from("Template").is_err());
}

#[test]
fn node_new_rejects_a_blank_path() {
    let attrs = NodeAttributes {
        path: "   ".into(),
        ..Default::default()
    };
    assert!(matches!(
        Node::new("n1".into(), attrs),
        Err(DomainError::InvalidPath(_))
    ));
}

#[test]
fn node_new_rejects_an_unnormalised_path() {
    // leading slash, trailing slash, empty segment, "." segment, ".."
    // segment and a backslash are each rejected.
    for bad in ["/a", "a/", "a//b", "a/./b", "a/../b", "a\\b"] {
        let attrs = NodeAttributes {
            path: bad.into(),
            ..Default::default()
        };
        assert!(
            matches!(
                Node::new("n1".into(), attrs),
                Err(DomainError::InvalidPath(_))
            ),
            "path {bad} should be rejected"
        );
    }
}

#[test]
fn node_new_rejects_a_reference_on_a_directory() {
    let attrs = NodeAttributes {
        path: "a".into(),
        directory: true,
        reference: Some("/abs/a.txt".into()),
        ..Default::default()
    };
    assert!(matches!(
        Node::new("n1".into(), attrs),
        Err(DomainError::ReferenceOnDirectory)
    ));
}

#[test]
fn node_new_rejects_a_migration_on_a_file() {
    let attrs = NodeAttributes {
        path: "a.txt".into(),
        directory: false,
        migration: Some(Migration {
            directory: "/src".into(),
            filter: vec![".sas".into()],
        }),
        ..Default::default()
    };
    assert!(matches!(
        Node::new("n1".into(), attrs),
        Err(DomainError::MigrationOnFile)
    ));
}

#[test]
fn node_new_accepts_a_well_formed_node() {
    let attrs = NodeAttributes {
        description: "the raw folder".into(),
        path: "raw/b.txt".into(),
        directory: false,
        parent: Some("t_root".into()),
        reference: Some("/abs/b.txt".into()),
        migration: None,
    };
    let node = Node::new("n1".into(), attrs).unwrap();
    assert_eq!(node.node_id, "n1");
    assert_eq!(node.path, "raw/b.txt");
    assert_eq!(node.hidden, None);
}

#[test]
fn node_new_accepts_a_directory_with_a_migration() {
    let attrs = NodeAttributes {
        description: "the stats folder".into(),
        path: "stats".into(),
        directory: true,
        parent: None,
        reference: None,
        migration: Some(Migration {
            directory: "/abs/refs".into(),
            filter: vec![".sas".into()],
        }),
    };
    let node = Node::new("n1".into(), attrs).unwrap();
    assert!(node.directory);
    assert_eq!(node.migration.unwrap().filter, vec![".sas".to_string()]);
}

#[test]
fn structure_new_accepts_a_template_with_no_owner() {
    let structure = Structure::new(
        "Standard".into(),
        StructureLevel::Template,
        None,
        None,
    )
    .unwrap();
    assert_eq!(structure.level, StructureLevel::Template);
    assert_eq!(structure.project_code, None);
    assert_eq!(structure.template_id, None);
    assert_eq!(structure.id, 0);
}

#[test]
fn structure_new_accepts_a_project_with_an_owner_and_a_template() {
    let structure = Structure::new(
        "Study ABC".into(),
        StructureLevel::Project,
        Some("ABC".into()),
        Some(7),
    )
    .unwrap();
    assert_eq!(structure.project_code.as_deref(), Some("ABC"));
    assert_eq!(structure.template_id, Some(7));
}

#[test]
fn structure_new_rejects_a_blank_name() {
    let err = Structure::new("   ".into(), StructureLevel::Template, None, None).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn structure_new_rejects_a_template_with_an_owner() {
    let err = Structure::new(
        "Standard".into(),
        StructureLevel::Template,
        Some("ABC".into()),
        None,
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::UnexpectedOwner));
}

#[test]
fn structure_new_rejects_a_project_without_a_project_code() {
    let err = Structure::new("Study".into(), StructureLevel::Project, None, Some(7)).unwrap_err();
    assert!(matches!(err, DomainError::MissingProjectCode));
}

#[test]
fn structure_new_rejects_a_project_without_a_template() {
    let err = Structure::new("Study".into(), StructureLevel::Project, Some("ABC".into()), None)
        .unwrap_err();
    assert!(matches!(err, DomainError::MissingTemplate));
}
