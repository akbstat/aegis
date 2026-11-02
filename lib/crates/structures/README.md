# structures

Records the **directory structure** of a clinical project: the set of folders and
files that should exist on disk, and the rule that lets a project *extend* a
shared template of that layout.

The crate performs **no filesystem I/O**. It records what a structure should look
like; materialising it — creating directories, copying migration files — is the
desktop app's job.

## The model

- A **template** structure is global and shared. Many may exist.
- A **project** structure belongs to exactly one project and extends exactly one
  template. It records only the *difference* from that template.
- A **node** is a folder or a file. Its `node_id` is unique inside one structure
  and is a **shared identity space**: a project structure's record for an
  inherited node carries the template node's `node_id`.
- A **migration** says what to copy into a directory once it has been created.

Reading a project structure returns the **effective tree**: the template's nodes
merged with the project structure's extension records. The merge joins on
`node_id`, never on `path` — which is why renaming an inherited node works and
the template's original does not reappear.

## `src/` tree

```
src/
├── lib.rs
├── domain.rs
├── domain/
│   ├── structure_level.rs     # Template | Project
│   ├── migration.rs           # { directory, filter }
│   ├── node.rs                # Node, NodeAttributes, the path rules
│   ├── structure.rs           # the Structure aggregate
│   ├── merge.rs               # the merge algorithm (pure)
│   ├── repository.rs          # StructureRepository
│   ├── node_repository.rs     # NodeRepository
│   ├── project_lookup.rs      # ProjectLookup
│   ├── error.rs               # DomainError
│   └── tests.rs
├── usecase.rs
├── usecase/
│   ├── commands.rs            # CreateStructure, AddNode, UpdateNode, …
│   ├── views.rs               # StructureView, NodeView, OwnNodeView, …
│   ├── error.rs               # UsecaseError
│   ├── structure_usecase.rs   # the write rules
│   └── tests.rs
└── adapter.rs
└── adapter/
    ├── persistence.rs
    ├── persistence/postgres.rs
    ├── persistence/postgres/
    │   ├── structure_repo.rs  # StructureRepoPg
    │   └── node_repo.rs       # NodeRepoPg
    ├── service.rs
    ├── service/project.rs
    ├── service/project/project_lookup_impl.rs
    ├── facade.rs
    └── facade/in_memory.rs
    └── facade/in_memory/
        ├── service.rs         # StructureServiceImpl
        └── tests.rs
```

## Database setup

The crate owns two tables: `structures` and `structure_nodes`.

```bash
sqlx migrate run --source lib/crates/structures/migrations
```

against the database named by `AEGIS_DATABASE_URL` for the server, or against a
database of its own for the live tests.

> **The URL must point at a dedicated database.** SQLx keeps a single
> `_sqlx_migrations` table *per database*, shared by every crate that migrates
> it. The live test below drops that table, so pointing it at the server's
> database would erase the applied-migration record for every other crate.

## Constructor snippet

```rust
use std::sync::Arc;

use structures::{
    ProjectLookupImpl, StructureRepoPg, StructureServiceImpl, StructureUsecase,
    StructureUsecaseConfig, NodeRepoPg,
};

let structures = StructureRepoPg::new(pool.clone());
let nodes = NodeRepoPg::new(pool);
let projects = Arc::new(ProjectLookupImpl::new(project_service));
let usecase = StructureUsecase::new(StructureUsecaseConfig {
    structures,
    nodes,
    projects,
});
let service = Arc::new(StructureServiceImpl::new(usecase));
```

## Running the tests

```bash
cargo test -p structures                                  # unit + compile-only
cargo test -p structures -- --ignored --test-threads=1    # live PostgreSQL
```

The live tests need `AEGIS_STRUCTURES_DATABASE_URL` set (or present in `.env` at
the workspace root) and a **dedicated** database. Each run drops the live tables
and the `_sqlx_migrations` table before re-applying the migrations — destructive
against a real database on purpose.

## Further reading

The workspace conventions this crate follows are in
[docs/guidelines/lib-crate-development.md](../../../docs/guidelines/lib-crate-development.md).
