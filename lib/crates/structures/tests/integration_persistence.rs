//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `AEGIS_STRUCTURES_DATABASE_URL` set:
//!
//! ```text
//! cargo test -p structures -- --ignored --test-threads=1
//! ```
//!
//! Each run drops and re-applies the migrations so the live DB stays
//! in a deterministic state. A failure to connect is reported via a
//! clear panic (never silently skipped).
//!
//! ## The URL must point at a dedicated database
//!
//! SQLx keeps a single `_sqlx_migrations` table *per database*, shared
//! by every crate that migrates it. This test drops that table, so
//! pointing it at the server's database would erase the
//! applied-migration record for auth, user, project, terminology,
//! mission, crf and sdtm-project alike. Use a database of its own.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;
use std::sync::Arc;
use structures::{
    CreateStructure, DeleteNode, DomainError, Migration, Node, NodeAttributes, NodeRepoPg,
    NodeRepository, ProjectLookup, StructureLevel, StructureRepoPg, StructureUsecase,
    StructureUsecaseConfig, UpdateNode, UpdateStructure,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_STRUCTURES_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_STRUCTURES_DATABASE_URL must be set (or present in .env \
             at the workspace root) to run --ignored tests. It must point at a \
             DEDICATED database: the test drops the _sqlx_migrations table, \
             which sqlx shares across every crate that migrates that database."
        )
    });

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_STRUCTURES_DATABASE_URL");

    // Drop in reverse FK order so cascading rules never matter. The
    // statements are literals rather than a formatted list: sqlx
    // requires `&'static str`, and there is nothing to interpolate.
    for stmt in [
        "DROP TABLE IF EXISTS structure_nodes CASCADE",
        "DROP TABLE IF EXISTS structures CASCADE",
        "DROP TABLE IF EXISTS _sqlx_migrations CASCADE",
    ] {
        sqlx::query(stmt)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("drop {stmt}: {e}"));
    }

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

    f(pool).await
}

/// A per-run unique value for every column with a `UNIQUE` constraint,
/// so concurrent runs cannot collide.
fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

/// A project lookup that accepts every code — the live test does not
/// exercise the project table.
struct AnyProject;

#[async_trait::async_trait]
impl ProjectLookup for AnyProject {
    async fn get_by_code(&self, _code: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

fn node(node_id: &str, entry_name: &str, directory: bool, parent: Option<&str>) -> Node {
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

/// The usecase wired to the live Postgres repositories.
fn usecase(pool: PgPool) -> StructureUsecase<StructureRepoPg, NodeRepoPg, AnyProject> {
    StructureUsecase::new(StructureUsecaseConfig {
        structures: StructureRepoPg::new(pool.clone()),
        nodes: NodeRepoPg::new(pool),
        projects: Arc::new(AnyProject),
    })
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn a_template_structure_round_trips_through_postgres() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let view = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        assert!(view.id > 0);

        let nodes = NodeRepoPg::new(pool.clone());
        nodes
            .insert(
                view.id,
                &Node::new(
                    "t_root".into(),
                    NodeAttributes {
                        description: "the raw folder".into(),
                        entry_name: "raw".into(),
                        directory: true,
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let fetched = uc.get_structure(view.id).await.unwrap();
        assert_eq!(fetched.nodes.len(), 1);
        assert_eq!(fetched.nodes[0].node_id, "t_root");
        assert_eq!(fetched.nodes[0].entry_name, "raw");
        assert_eq!(fetched.nodes[0].description, "the raw folder");
        assert_eq!(fetched.rootpath, "/abs/root");
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn a_migration_and_a_reference_survive_the_jsonb_round_trip() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let view = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        let nodes = NodeRepoPg::new(pool.clone());
        nodes
            .insert(
                view.id,
                &Node::new(
                    "t_stats".into(),
                    NodeAttributes {
                        description: String::new(),
                        entry_name: "stats".into(),
                        directory: true,
                        migration: Some(Migration {
                            directory: "/abs/references".into(),
                            filter: vec![".sas".into(), ".xlsx".into()],
                        }),
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
            .await
            .unwrap();
        nodes
            .insert(
                view.id,
                &Node::new(
                    "t_file".into(),
                    NodeAttributes {
                        description: String::new(),
                        entry_name: "a.txt".into(),
                        directory: false,
                        reference: Some("/abs/a.txt".into()),
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let fetched = uc.get_structure(view.id).await.unwrap();
        let stats = fetched
            .nodes
            .iter()
            .find(|n| n.node_id == "t_stats")
            .unwrap();
        let migration = stats.migration.as_ref().unwrap();
        assert_eq!(migration.directory, "/abs/references");
        assert_eq!(
            migration.filter,
            vec![".sas".to_string(), ".xlsx".to_string()]
        );
        let file = fetched
            .nodes
            .iter()
            .find(|n| n.node_id == "t_file")
            .unwrap();
        assert_eq!(file.reference.as_deref(), Some("/abs/a.txt"));
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn a_project_structure_merges_over_its_template() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let template = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        let nodes = NodeRepoPg::new(pool.clone());
        nodes
            .insert(template.id, &node("t_root", "raw", true, None))
            .await
            .unwrap();
        nodes
            .insert(template.id, &node("t_file", "a.txt", false, Some("t_root")))
            .await
            .unwrap();

        let project_code = unique("ABC");
        let project = uc
            .create_structure(CreateStructure {
                name: unique("project"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Project,
                project_code: Some(project_code.clone()),
                template_id: Some(template.id),
            })
            .await
            .unwrap();

        // A new node under the template's directory.
        nodes
            .insert(project.id, &node("n_new", "b.txt", false, Some("t_root")))
            .await
            .unwrap();

        let tree = uc.build_structure(project.id).await.unwrap();
        assert_eq!(tree.nodes.len(), 1);
        let mut children: Vec<String> = tree.nodes[0]
            .children
            .iter()
            .map(|c| c.entry_name.clone())
            .collect();
        children.sort();
        assert_eq!(children, vec!["a.txt".to_string(), "b.txt".to_string()]);
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn overriding_a_template_node_drops_the_template_children() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let template = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        let nodes = NodeRepoPg::new(pool.clone());
        nodes
            .insert(template.id, &node("t_root", "raw", true, None))
            .await
            .unwrap();
        nodes
            .insert(template.id, &node("t_file", "a.txt", false, Some("t_root")))
            .await
            .unwrap();

        let project = uc
            .create_structure(CreateStructure {
                name: unique("project"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Project,
                project_code: Some(unique("ABC")),
                template_id: Some(template.id),
            })
            .await
            .unwrap();

        // The project overrides `raw`, reusing the template's node_id.
        uc.update_node(UpdateNode {
            structure_id: project.id,
            node_id: "t_root".into(),
            entry_name: "raw".into(),
            description: "project raw".into(),
            directory: true,
            parent: None,
            reference: None,
            migration: None,
            hidden: None,
        })
        .await
        .unwrap();

        let tree = uc.build_structure(project.id).await.unwrap();
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0].description, "project raw");
        // the template's child is gone
        assert!(tree.nodes[0].children.is_empty());
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn hiding_a_template_node_removes_it_from_the_tree() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let template = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        let nodes = NodeRepoPg::new(pool.clone());
        nodes
            .insert(template.id, &node("t_doc", "docs", true, None))
            .await
            .unwrap();

        let project = uc
            .create_structure(CreateStructure {
                name: unique("project"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Project,
                project_code: Some(unique("ABC")),
                template_id: Some(template.id),
            })
            .await
            .unwrap();

        uc.delete_node(DeleteNode {
            structure_id: project.id,
            node_id: "t_doc".into(),
        })
        .await
        .unwrap();

        let tree = uc.build_structure(project.id).await.unwrap();
        assert!(tree.nodes.is_empty());

        // and the hide is visible through the own-records list
        let own = uc.list_own_nodes(project.id).await.unwrap();
        assert_eq!(own.len(), 1);
        assert!(own[0].hidden);
        assert!(own[0].overrides_template);
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn deleting_a_structure_cascades_to_its_nodes() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let view = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        let nodes = NodeRepoPg::new(pool.clone());
        nodes
            .insert(view.id, &node("t_root", "raw", true, None))
            .await
            .unwrap();

        uc.delete_structure(view.id).await.unwrap();
        assert!(nodes.list(view.id).await.unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn the_database_refuses_two_siblings_with_the_same_name() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let view = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        let nodes = NodeRepoPg::new(pool.clone());
        nodes
            .insert(view.id, &node("t_root", "raw", true, None))
            .await
            .unwrap();
        nodes
            .insert(view.id, &node("t_other", "other", true, None))
            .await
            .unwrap();

        // The same name under two *different* parents is fine.
        nodes
            .insert(view.id, &node("n_a", "same.txt", false, Some("t_root")))
            .await
            .unwrap();
        nodes
            .insert(view.id, &node("n_b", "same.txt", false, Some("t_other")))
            .await
            .unwrap();

        // The same name under the *same* parent is not.
        let err = nodes
            .insert(view.id, &node("n_c", "same.txt", false, Some("t_root")))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::DuplicateName(_)), "got {err:?}");

        // Two roots may not share a name either — a plain UNIQUE
        // constraint would let them through, because NULLs are
        // distinct.
        nodes
            .insert(view.id, &node("n_d", "root.txt", true, None))
            .await
            .unwrap();
        let err = nodes
            .insert(view.id, &node("n_e", "root.txt", true, None))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::DuplicateName(_)), "got {err:?}");
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn a_project_can_only_have_one_structure() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let template = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();
        let project_code = unique("ABC");
        uc.create_structure(CreateStructure {
            name: unique("project"),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Project,
            project_code: Some(project_code.clone()),
            template_id: Some(template.id),
        })
        .await
        .unwrap();

        let err = uc
            .create_structure(CreateStructure {
                name: unique("project"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Project,
                project_code: Some(project_code.clone()),
                template_id: Some(template.id),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            structures::UsecaseError::Repository(DomainError::DuplicateProjectStructure(code))
                if code == project_code
        ));
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn a_structure_written_before_the_rootpath_reads_back_empty() {
    with_pool(|pool| async move {
        // A row that predates the rootpath column: the column is NOT
        // NULL with a default, so a direct insert without it lands an
        // empty root path.
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO structures (name, rootpath, level) VALUES ($1, '', 'template') RETURNING id",
        )
        .bind(unique("template"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let uc = usecase(pool.clone());
        let view = uc.get_structure(id).await.unwrap();
        assert_eq!(view.rootpath, "");
    })
    .await;
}

#[tokio::test]
#[ignore = "needs AEGIS_STRUCTURES_DATABASE_URL"]
async fn updating_a_structure_stores_a_new_rootpath_and_keeps_the_old_one() {
    with_pool(|pool| async move {
        let uc = usecase(pool.clone());
        let view = uc
            .create_structure(CreateStructure {
                name: unique("template"),
                rootpath: "/abs/root".into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap();

        // A supplied root path replaces the stored one.
        let moved = uc
            .update_structure(UpdateStructure {
                id: view.id,
                name: "Moved".into(),
                rootpath: Some("/abs/other".into()),
            })
            .await
            .unwrap();
        assert_eq!(moved.rootpath, "/abs/other");
        assert_eq!(moved.name, "Moved");

        // An absent root path leaves the stored one alone -- this is
        // the `COALESCE` in the UPDATE statement, and only a live
        // database can show it.
        let kept = uc
            .update_structure(UpdateStructure {
                id: view.id,
                name: "Kept".into(),
                rootpath: None,
            })
            .await
            .unwrap();
        assert_eq!(kept.rootpath, "/abs/other");
        assert_eq!(kept.name, "Kept");

        // A blank root path is rejected, and the name is not changed
        // either -- the whole update is rejected.
        let err = uc
            .update_structure(UpdateStructure {
                id: view.id,
                name: "Rejected".into(),
                rootpath: Some("   ".into()),
            })
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                structures::UsecaseError::Validation(DomainError::EmptyRootpath)
            ),
            "got {err:?}"
        );
        let unchanged = uc.get_structure(view.id).await.unwrap();
        assert_eq!(unchanged.name, "Kept");
        assert_eq!(unchanged.rootpath, "/abs/other");
    })
    .await;
}
