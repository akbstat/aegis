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
- A node's **`entry_name`** is its own name — one folder or one file name, *not*
  a path. The nesting comes from the `parent` link, and a node's path relative
  to the structure's root path is the chain of names above it. A name is unique
  among the node's **siblings**: the same name may appear under two different
  parents, but never twice under the same one.
- A structure's **`rootpath`** is the absolute base directory its tree is rooted
  at. Every structure has one, at every level; it is set when the structure is
  created and can be changed afterwards. A node's full path is the root path
  followed by the chain of ancestor names. Structures created before the column
  existed read back with an empty root path until one is set.
- A **migration** says what to copy into a directory once it has been created.

> **The desktop client needs a follow-up.** `POST /api/structures` now requires
> a `rootpath`, and the desktop's `CreateStructureRequest` does not send one, so
> the desktop's create-structure call is rejected with a 422 until it is
> updated. The desktop's *reads* are unaffected (its wire DTOs ignore unknown
> fields) and its update call is unaffected (the root path is optional there).
> Three places need the field: the desktop's Rust `CreateStructureRequest` in
> `src-tauri/src/http/structures.rs`, its TypeScript `CreateStructureInput`
> mirror in `src/shared/api/types.ts`, and the create form in
> `src/features/templates/components/StructureDrawer.tsx`.

Reading a project structure returns the **effective tree**: the template's nodes
merged with the project structure's extension records. The merge joins on
`node_id`, never on `entry_name` — which is why renaming an inherited node works
and the template's original does not reappear.

## `src/` tree

```
src/
├── lib.rs
├── domain.rs
├── domain/
│   ├── structure_level.rs     # Template | Project
│   ├── migration.rs           # { directory, filter }
│   ├── node.rs                # Node, NodeAttributes, the name rules
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
    └── facade/internal.rs
    └── facade/internal/
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

> **Apply the migrations before deploying a binary that reads the new
> columns.** The server does not auto-migrate. Every structures query selects
> `structures.rootpath`, so a binary deployed ahead of the migration fails
> every structures request with "column rootpath does not exist".

> **The URL must point at a disposable test database.** SQLx keeps a single
> `_sqlx_migrations` table *per database*, shared by every crate that migrates
> it. The live test below drops that table, so pointing it at the server's
> database would erase the applied-migration record for every other crate.

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/structures/migrations --timestamp <description>
```

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

The live tests need `AEGIS_TEST_DATABASE_URL` set (or present in `.env` at
the workspace root) and a **disposable** database, shared with the other crates'
live-DB tests. Each run drops the live tables
and the `_sqlx_migrations` table before re-applying the migrations — destructive
against a real database on purpose.

## Further reading

The workspace conventions this crate follows are in
[docs/guidelines/lib-crate-development.md](../../../docs/guidelines/lib-crate-development.md).
