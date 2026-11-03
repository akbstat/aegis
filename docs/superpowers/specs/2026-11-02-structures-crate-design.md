# Structures Crate Design

> **Revision — 2026-11-03.** A node's `path` was renamed to `entry_name`,
> and the value it holds changed from a path relative to the structure root
> to the node's **own** name — one folder or one file name. The `parent`
> link carries the nesting; the full path of a node is the chain of names
> above it. Uniqueness follows from that: a name is unique among the
> node's **siblings** (`(structure_id, parent_id, entry_name)`), so the
> same name may appear under two different parents. The `path` column
> became `entry_name`, `structure_nodes_unique_path` became the
> `structure_nodes_unique_name_per_parent` index, and the errors became
> `InvalidName` / `DuplicateName` (`duplicate_name` on the wire).

## Goal

Add `lib/crates/structures` as a reusable Rust library that owns the
**directory structure** of a clinical project: the set of folders and files
that should exist on disk, and the rule that lets a project *extend* a
shared template of that layout.

The crate owns:

- The schema for two tables (`structures`, `structure_nodes`) — two migrations.
- The domain model for a `Structure`, its `Node`s and a node's `Migration`.
- The **merge algorithm**: the pure function that combines a template's nodes
  with a project structure's extension records into one tree.
- A `StructureUsecase` that owns CRUD over structures and over individual
  nodes, and that resolves the write-time rules (which node a write refers
  to, when a write overrides an inherited node).
- A `StructureServiceImpl` facade that adapts the usecase to a new
  `apis::structures::StructureService` outbound port.

The crate does **not** touch the filesystem. It records what a structure
*should* look like; materialising it — creating directories, copying
migration files — is the desktop app's job and is out of scope here.

The crate does **not** own projects. Project existence is checked through the
`apis::project::ProjectService` port, exactly as `sdtm-project` does.

## Scope

| In | Out |
| --- | --- |
| The `structures` lib crate (domain / usecase / adapter) | Creating directories or copying files on disk |
| The `apis::structures::StructureService` port | Any desktop change (`apps/desktop/aegis-desktop` is untouched) |
| The server's HTTP routes for structures | Seeding a built-in template |
| Live-DB integration tests for the new tables | Pagination on the list endpoint |
| | Materialising a structure from the tree |

## Terminology

- **Template structure** — a global, shared layout. Many may exist. Nobody
  "owns" one; they are written by admins only.
- **Project structure** — the layout of exactly one project. It **must** point
  at a template structure and records only the *difference* from that
  template.
- **Extension record** — a row in `structure_nodes` belonging to a project
  structure. It is a difference from the template: an addition, an override,
  or a hide.
- **Effective structure** — the merge of a template's nodes with a project
  structure's extension records. It is what the caller sees.
- **Node name** (`entry_name`) — a node's *own* name: one folder or one file
  name, not a path. The nesting comes from the `parent` link, and the full
  path of a node is the chain of names above it. A name is unique among the
  node's **siblings** — the same name may appear under two different
  parents, but never twice under the same one.

## Architecture

Ports-and-adapters DDD structure, mirroring
[`lib/crates/sdtm-project/`](../../lib/crates/sdtm-project/):

- `domain` — value objects (`StructureLevel`, `Migration`), the aggregates
  (`Structure`, `Node`), the **merge algorithm** (`merge.rs`), the ports
  (`StructureRepository`, `NodeRepository`, `ProjectLookup`), and
  `DomainError`. No I/O, no `sqlx`, no `tokio`.
- `usecase` — `StructureUsecase` orchestrates CRUD across the three ports
  and projects aggregates into view DTOs. Command DTOs in `usecase/commands.rs`;
  views in `usecase/views.rs`; `UsecaseError` in `usecase/error.rs`.
- `adapter` — concrete implementations:
  - `adapter::persistence::postgres` — `StructureRepoPg`, `NodeRepoPg` (SQLx
    runtime API).
  - `adapter::service::project::ProjectLookupImpl` — bridges
    `apis::project::ProjectService` to the domain `ProjectLookup`.
  - `adapter::facade::in_memory::StructureServiceImpl` — adapts
    `StructureUsecase` to `apis::structures::StructureService`.

### Module tree

```
lib/crates/structures/
├── Cargo.toml
├── README.md
├── migrations/
│   ├── 0001_create_structures.sql
│   └── 0002_create_structure_nodes.sql
├── src/
│   ├── lib.rs
│   ├── domain.rs
│   ├── domain/
│   │   ├── structure_level.rs
│   │   ├── migration.rs
│   │   ├── node.rs
│   │   ├── structure.rs
│   │   ├── merge.rs
│   │   ├── repository.rs
│   │   ├── project_lookup.rs
│   │   ├── error.rs
│   │   └── tests.rs
│   ├── usecase.rs
│   ├── usecase/
│   │   ├── commands.rs
│   │   ├── error.rs
│   │   ├── views.rs
│   │   ├── structure_usecase.rs
│   │   └── tests.rs
│   └── adapter.rs
│   └── adapter/
│       ├── persistence.rs
│       ├── persistence/postgres.rs
│       ├── persistence/postgres/
│       │   ├── structure_repo.rs
│       │   └── node_repo.rs
│       ├── service.rs
│       ├── service/project.rs
│       ├── facade.rs
│       └── facade/in_memory.rs
│       └── facade/in_memory/
│           ├── service.rs
│           └── tests.rs
└── tests/
    ├── public_api.rs
    └── integration_persistence.rs
```

## Data Model

```rust
// domain/structure_level.rs
/// Which kind of structure this is.
///
/// `Template` structures are global and shared; `Project` structures belong
/// to one project and extend exactly one template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureLevel {
    Template,
    Project,
}

impl StructureLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            StructureLevel::Template => "template",
            StructureLevel::Project => "project",
        }
    }
}

// domain/migration.rs
/// What to copy into a directory once it has been created.
///
/// `directory` is an absolute folder path; `filter` holds the target
/// suffixes — only files whose name ends with one of them are copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub directory: String,
    pub filter: Vec<String>,
}

// domain/node.rs
/// One entry in a structure — a folder or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Unique inside one structure. **Shared identity space**: a project
    /// structure's extension record for an inherited node carries the
    /// template node's `node_id`.
    pub node_id: String,
    /// Free-text label. May be empty.
    pub description: String,
    /// The node's own name — a folder or a file name, NOT a path. The
    /// nesting comes from `parent`.
    pub entry_name: String,
    /// When true the node is filtered out of the effective structure.
    /// Only meaningful at project level.
    pub hidden: Option<bool>,
    /// Whether this node is a directory.
    pub directory: bool,
    /// The `node_id` of the containing node, if any.
    pub parent: Option<String>,
    /// Absolute file path. Only meaningful when `directory` is false.
    pub reference: Option<String>,
    /// What to copy into this directory. Only meaningful when `directory`
    /// is true.
    pub migration: Option<Migration>,
}

// domain/structure.rs
/// A directory layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    /// The owning project. Set at project level, absent at template level.
    pub project_code: Option<String>,
    /// The template this structure extends. Set at project level, absent at
    /// template level. **Immutable** once the structure is created.
    pub template_id: Option<i64>,
    /// The structure's own rows. At project level these are the *extension
    /// records* — the difference from the template, not the whole tree.
    pub nodes: Vec<Node>,
}
```

### `node_id` is a shared identity space

A project structure's extension record and the template node it overrides
carry the **same** `node_id`. This is what makes the merge a join on
`node_id` rather than on `entry_name`, and it is what lets a newly added node
point its `parent` at an inherited template node.

The client never picks a `node_id`. The system mints one (a 21-character
`nanoid`) when a node is genuinely new, and reuses the existing node's
`node_id` when the write refers to a node that already exists.

## The merge algorithm

`domain/merge.rs` holds a pure function — no I/O, fully unit-testable:

```rust
/// A node in the merged tree: its effective attributes plus its
/// effective children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedNode {
    pub node: Node,
    pub children: Vec<MergedNode>,
}

/// The merged result: the effective tree's roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedTree {
    pub roots: Vec<MergedNode>,
}

/// Combine a template's nodes with a project structure's extension records.
pub fn merge(template: &[Node], own: &[Node]) -> MergedTree;
```

The merge is **total** — it never fails. Malformed input (a `parent` naming a
node that is not in the merged set, a cycle through `parent`) is resolved
defensively rather than rejected: a node whose parent is absent is treated as
a root, and a node already visited during the recursion is skipped. The
usecase validates those shapes before the merge ever sees them.

### Rules

1. **The join key is `node_id`.** The merge builds one global map from
   `node_id` to the *effective record* for that node: the template's node
   first, then the extension record with the same `node_id` replaces it. An
   extension record whose `node_id` the template does not know is an
   **addition**.
2. **An override is a wholesale replacement.** The extension record's
   `description`, `entry_name`, `directory`, `reference` and `migration` replace
   the template node's — including an empty description and an absent
   reference or migration. Nothing is inherited field by field.
3. **Each `node_id` appears exactly once in the merged tree**, under its
   effective record's `parent`. A node re-parented by the project therefore
   moves; it is never listed under two parents.
4. **Overriding a node drops the template's children.** The effective
   children of an overridden node are exactly the extension records that
   name it as `parent`. The template's own children under it disappear —
   and so does everything below them, because those nodes are no longer
   reachable from any root. An extension record whose parent is invisible
   is dangling and is not rendered either.
5. **Adding under a template node keeps the template's other children.** If
   a node is *not* overridden, its effective children are the template's
   children plus the extension records under it. When an extension record
   and a template child share a `node_id`, the extension record wins and the
   child appears once.
6. **Hidden nodes are filtered out**, together with their whole subtree.
7. **Ordering is deterministic**: children and roots are sorted by
   `entry_name` ascending, then by `node_id` so that two nodes with the same
   name — which only a later template edit can produce — still have a
   stable order.

### The merge renders; it does not look up

`merge()` is a **pure renderer**. It produces the tree a caller sees.

The usecase's *lookups* — which node does this `POST` for name `n` under
parent `p` refer to, which node does this `PUT` at `node_id` `n` refer to —
run over a different index: **the effective records**, the template's nodes
with this structure's own records layered on top, the own record winning.
Hidden own records are in the index. The rendered tree excludes hidden
nodes, so a hidden node is not in the tree, but it *is* still addressable by
`node_id`. That is what makes a hide reversible.

Because the own record wins, a node the project has renamed does **not**
also answer to its template name: the template's name is free again.

The two indexes are deliberately different. The tree answers "what does
this structure look like"; the index answers "what does this write refer
to".

### Worked example

Template `T` (template structure `1`):

| node_id | entry_name | directory | parent |
| --- | --- | --- | --- |
| `t_root` | `raw` | true | — |
| `t_file` | `a.txt` | false | `t_root` |
| `t_doc` | `docs` | true | — |

Project structure `2` extends template `1` with these extension records:

| node_id | entry_name | directory | parent | note |
| --- | --- | --- | --- | --- |
| `t_root` | `raw` | true | — | overrides the template's `raw` |
| `n_new` | `b.txt` | false | `t_root` | new file inside the overridden dir |
| `t_doc` | `docs` | true | — | hides the template's `docs` |

Effective tree:

- `raw` — overridden, so its children are the extension records under it:
  - `b.txt`
  - (`a.txt` is gone — it was a *template* child of an overridden node)
- `docs` — hidden, filtered out with its subtree.

Had the project **not** overridden `raw`, `a.txt` would still be there and
`b.txt` would sit beside it.

### Why the join key is `node_id` and not `entry_name`

With `entry_name` as the join key, renaming an inherited node breaks the
merge: the record stops matching its template node, so the template's
original reappears alongside the renamed copy. With `node_id` as the join
key the rename simply wins. The `entry_name` is still what the *system* uses
at write time to decide whether a write refers to an existing node — that
decision is made by the server, never by the client.

## Write semantics

### Which node does a write refer to?

| Operation | Addressing | Target |
| --- | --- | --- |
| `POST …/nodes` | by `entry_name` under `parent` | the node with that name under that parent in the *effective records* |
| `PUT …/nodes/{node_id}` | by `node_id` | that node in the *effective records* |
| `DELETE …/nodes/{node_id}` | by `node_id` | that node in the *effective records* |

**`POST` — place a node under a parent.**

1. The system looks the `entry_name` up among the effective records whose
   `parent` is the requested parent.
2. **No node with that name under that parent** → the system mints a
   `node_id` and inserts a new record. `201`.
3. **A node with that name under that parent, recorded by this structure** →
   `409` `duplicate_name`. The record already exists; the client should
   `PUT` it.
4. **A node with that name under that parent, inherited from the template**
   → the system writes an **override record** reusing that node's `node_id`.
   The effective structure now shows the project's version of it. `201`.

Case 4 is the "the system decides the override" rule: the client asked for a
node with a name that already exists under that parent, and the system bound
the write to the existing node rather than creating a duplicate beside it.

**`PUT` — update a node.**

The `node_id` must exist in the effective records — including a node this
structure recorded and then hid, which is how a hide is undone.

- **Own record** → the row is updated. `200`.
- **Inherited template node** → an override record is created carrying the
  template's `node_id`, and the supplied attributes replace the template
  node's wholesale. `200`.
- **Unknown `node_id`** → `404`.

**`DELETE` — remove a node.**

`DELETE` has one meaning in every case: **the node and everything under it
are not part of this structure.** For the target and every own record
beneath it:

- **the record has a template counterpart** (its `node_id` exists in the
  template) → the record is set to `hidden = true`. The node drops out of
  the effective structure.
- **the record has no template counterpart** → the row is deleted.

`204` either way.

Hiding rather than deleting an override matters: deleting an override's row
would revert the node to the *template's* version and put it straight back
into the tree.

### `hidden`

`hidden` is only meaningful at project level. Writing `hidden = true`
against a **template** structure is rejected with `400`.

A hidden node is filtered out of the effective structure along with its
subtree. Writing `hidden = true` on a node that overrides nothing — i.e. a
brand-new node — is rejected with `400`: it would be invisible, which is
never what the caller meant.

`hidden = true` on a node the project recorded itself keeps the row, so the
hide can be undone with a `PUT` carrying `hidden = false`. The row is
reachable through the own-records endpoint, which is where a client learns
the `node_id` of a hidden node in the first place.

### `parent`

`parent` names the `node_id` of the containing node. It may name:

- a node this structure recorded, or
- a node inherited from the template.

The parent must exist in own records ∪ template nodes and must be a
**directory**. A node cannot be its own parent, and a node cannot be
re-parented under one of its own descendants — both are `400` (cycle).

`parent` is **always explicit** in a `POST` or `PUT` body: `null` means
"this node is at the root". Both operations are full replacements, so an
omitted `parent` must never silently re-parent a node.

### `entry_name`

- Non-blank, and a **single segment**: it names one folder or one file, so
  no `/`, no `\`, and neither `.` nor `..`.
- **Unique among the siblings.** No other node under the *same* parent may
  carry that name — a name already used by a different node under the same
  parent is `409` `duplicate_name`. The same name under two *different*
  parents is fine.

The name is the node's own — it says nothing about where the node sits. The
tree is defined by the `parent` links; the full path of a node is the chain
of names above it, and the desktop app owns composing it. A directory
rename therefore does not force a rename of the descendants.

A name collision produced by a *later template edit* — the template adds a
node with a name a project already uses under the same parent — is accepted
rather than rejected: the template repository cannot see project structures
through the current ports. The merge renders both nodes and sorts by
`(entry_name, node_id)` so the order stays deterministic.

## Validation

| Rule | Error |
| --- | --- |
| `name` non-blank | `DomainError::EmptyName` |
| `entry_name` non-blank and a single segment | `DomainError::InvalidName` |
| `entry_name` unique among the siblings | `DomainError::DuplicateName` |
| project-level structure has a `project_code` | `DomainError::MissingProjectCode` |
| project-level structure has a `template_id` | `DomainError::MissingTemplate` |
| template-level structure has neither | `DomainError::UnexpectedOwner` |
| the referenced project exists | `DomainError::ProjectNotFound` |
| the referenced template exists and is a template | `DomainError::TemplateNotFound` / `DomainError::NotATemplate` |
| one project structure per project | `DomainError::DuplicateProjectStructure` |
| `hidden` only at project level | `DomainError::HiddenNotAllowed` |
| `hidden` only on a node that overrides something | `DomainError::NothingToHide` |
| `reference` only on a file (`directory == false`) | `DomainError::ReferenceOnDirectory` |
| `migration` only on a directory (`directory == true`) | `DomainError::MigrationOnFile` |
| parent exists and is a directory | `DomainError::ParentNotFound` / `DomainError::ParentNotADirectory` |
| no cycles through `parent` | `DomainError::Cycle` |

## Schema

```sql
-- 0001_create_structures.sql
CREATE TABLE structures (
  id           BIGSERIAL PRIMARY KEY,
  name         TEXT NOT NULL,
  level        TEXT NOT NULL CHECK (level IN ('template', 'project')),
  project_code TEXT,
  template_id  BIGINT REFERENCES structures(id) ON DELETE RESTRICT,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT structures_level_shape CHECK (
    (level = 'template' AND project_code IS NULL AND template_id IS NULL) OR
    (level = 'project'  AND project_code IS NOT NULL AND template_id IS NOT NULL)
  )
);

-- Exactly one project-level structure per project.
CREATE UNIQUE INDEX structures_one_per_project
  ON structures (project_code) WHERE level = 'project';

CREATE TRIGGER structures_set_updated_at BEFORE UPDATE ON structures
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- 0002_create_structure_nodes.sql
CREATE TABLE structure_nodes (
  id                 BIGSERIAL PRIMARY KEY,
  structure_id       BIGINT NOT NULL REFERENCES structures(id) ON DELETE CASCADE,
  node_id            TEXT NOT NULL,
  description        TEXT NOT NULL DEFAULT '',
  entry_name        TEXT NOT NULL,
  directory          BOOLEAN NOT NULL DEFAULT TRUE,
  parent_id          TEXT,
  hidden             BOOLEAN NOT NULL DEFAULT FALSE,
  reference_path     TEXT,
  migration_directory TEXT,
  migration_filter   JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT structure_nodes_unique_node UNIQUE (structure_id, node_id),
  CONSTRAINT structure_nodes_migration_filter_array
    CHECK (jsonb_typeof(migration_filter) = 'array'),
  CONSTRAINT structure_nodes_reference_only_on_file
    CHECK (reference_path IS NULL OR directory = FALSE),
  CONSTRAINT structure_nodes_migration_only_on_directory
    CHECK (migration_directory IS NULL OR directory = TRUE)
);

-- A name is unique among the node's siblings. COALESCE folds the root
-- (parent_id IS NULL) into one bucket, so two roots may not share a name
-- either -- a plain UNIQUE constraint would let them through, because
-- NULLs are distinct.
CREATE UNIQUE INDEX structure_nodes_unique_name_per_parent
  ON structure_nodes (structure_id, COALESCE(parent_id, ''), entry_name);

CREATE INDEX structure_nodes_by_parent ON structure_nodes (structure_id, parent_id);

CREATE TRIGGER structure_nodes_set_updated_at BEFORE UPDATE ON structure_nodes
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
```

Notes:

- `parent_id` is a **soft** reference. It may name a node that lives in the
  *template* structure, so no foreign key is possible. The usecase
  validates it against the effective structure.
- `migration_directory` + `migration_filter` are two columns rather than one
  JSONB object, so a migration is inspectable in SQL. `migration_filter` is
  an array of target suffixes.
- Each table defines its **own** trigger function —
  `structures_set_updated_at()` and `structure_nodes_set_updated_at()`. There
  is no shared helper in this workspace; every crate defines a per-table
  function.
- `ON DELETE RESTRICT` on `template_id` is the backstop for "a template that
  a project structure still points at cannot be deleted". The usecase checks
  this first and answers `409 template_in_use`; the FK is the race backstop.
- Each table gets its **own** trigger function — `structures_set_updated_at()`
  and `structure_nodes_set_updated_at()`. There is no shared helper in this
  workspace; every crate defines a per-table function
  (`sdtm_project_versions_set_updated_at()`, `assignees_set_updated_at()`, …).

## Persistence (sqlx, runtime API)

SQLx runtime API (`sqlx::query_as`, `sqlx::query`, `FromRow`) throughout. The
workspace ships no `.sqlx/` offline cache and no `DATABASE_URL` at build
time, so the compile-time-checked macros are intentionally avoided. Each
repo file carries the module-level comment explaining this, as `sdtm-project`
does.

`map_*_error` in each repo is the only place that knows about `sqlx::Error`:

| Driver error | Domain error |
| --- | --- |
| `RowNotFound` | `DomainError::NotFound` |
| `Database` with SQLSTATE `23505` | `DomainError::DuplicateName` / `DuplicateProjectStructure` |
| `Database` with SQLSTATE `23503` | `DomainError::TemplateInUse` / `DomainError::NotFound` |
| anything else | `DomainError::Repository(msg)` |

## Public API surface at the crate root

```rust
// lib/crates/structures/src/lib.rs
pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::StructureServiceImpl;
pub use adapter::persistence::postgres::{NodeRepoPg, StructureRepoPg};
pub use adapter::service::project::ProjectLookupImpl;

pub use domain::{
    DomainError, MergedNode, MergedTree, Migration, Node, NodeAttributes, NodeRepository,
    ProjectLookup, Structure, StructureFilter, StructureLevel, StructureNew, StructureRepository,
};
pub use usecase::{
    AddNode, CreateStructure, DeleteNode, MigrationView, NodeView, OwnNodeView, RenameStructure,
    StructureMetaView, StructureSummaryView, StructureTreeView, StructureUsecase,
    StructureUsecaseConfig, StructureView, UpdateNode, UsecaseError,
};
```

## `DomainError` variants

```rust
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    // ---- validation (constructor-time) ----
    #[error("empty name")]
    EmptyName,
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("missing project code")]
    MissingProjectCode,
    #[error("missing template")]
    MissingTemplate,
    #[error("unexpected owner on a template structure")]
    UnexpectedOwner,
    #[error("hidden is not allowed on a template structure")]
    HiddenNotAllowed,
    #[error("nothing to hide: the node overrides no template node")]
    NothingToHide,
    #[error("a reference is only allowed on a file node")]
    ReferenceOnDirectory,
    #[error("a migration is only allowed on a directory node")]
    MigrationOnFile,
    #[error("parent node not found: {0}")]
    ParentNotFound(String),
    #[error("parent node is not a directory: {0}")]
    ParentNotADirectory(String),
    #[error("the node hierarchy contains a cycle")]
    Cycle,

    // ---- existence / duplicate (runtime) ----
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("structure not found: {0}")]
    StructureNotFound(i64),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("template structure not found: {0}")]
    TemplateNotFound(i64),
    #[error("structure {0} is not a template structure")]
    NotATemplate(i64),
    #[error("name already used under the same parent: {0}")]
    DuplicateName(String),
    #[error("project {0} already has a structure")]
    DuplicateProjectStructure(String),
    #[error("template structure {0} is still in use")]
    TemplateInUse(i64),

    #[error("not found")]
    NotFound,
    #[error("repository error: {0}")]
    Repository(String),
}
```

## Usecase

```rust
pub struct StructureUsecaseConfig<S, N, P> {
    pub structures: S,
    pub nodes: N,
    /// `Arc<P>` rather than `P`, so the call site can hand over an
    /// `Arc<ProjectLookupImpl>` directly — the `crf` / `sdtm-project` shape.
    pub projects: Arc<P>,
}

pub struct StructureUsecase<S, N, P> {
    structures: S,
    nodes: N,
    projects: Arc<P>,
}
```

`S: StructureRepository`, `N: NodeRepository`, `P: ProjectLookup`.

### Methods

```rust
// ---- structures ----
async fn create_structure(&self, cmd: CreateStructure) -> Result<StructureView, UsecaseError>;
async fn get_structure(&self, id: i64) -> Result<StructureView, UsecaseError>;
async fn get_structure_meta(&self, id: i64) -> Result<StructureMetaView, UsecaseError>;
async fn list_structures(
    &self,
    level: Option<StructureLevel>,
    project_code: Option<&str>,
) -> Result<Vec<StructureSummaryView>, UsecaseError>;
async fn rename_structure(&self, cmd: RenameStructure) -> Result<StructureView, UsecaseError>;
async fn delete_structure(&self, id: i64) -> Result<(), UsecaseError>;

// ---- nodes ----
async fn add_node(&self, cmd: AddNode) -> Result<NodeView, UsecaseError>;
async fn update_node(&self, cmd: UpdateNode) -> Result<NodeView, UsecaseError>;
async fn delete_node(&self, cmd: DeleteNode) -> Result<(), UsecaseError>;
async fn list_own_nodes(&self, structure_id: i64) -> Result<Vec<OwnNodeView>, UsecaseError>;

// ---- build ----
async fn build_structure(&self, id: i64) -> Result<StructureTreeView, UsecaseError>;
```

`create_structure` validates the project through `ProjectLookup` **before**
inserting, and validates that `template_id` names an existing
**template-level** structure. A project structure without a template is
`400`; a template that a project structure points at cannot be deleted
(`409 template_in_use`).

`rename_structure` changes the `name` only. `level`, `project_code` and
`template_id` are immutable after creation — re-pointing a project structure
at a different template would invalidate every extension record's `node_id`
binding, so the template pointer is fixed for the structure's lifetime.

### The effective structure

A private helper resolves the three node sets every write needs:

```rust
/// The nodes a write is resolved against.
struct Effective {
    /// The structure being written to.
    structure: Structure,
    /// The template's nodes. Empty for a template-level structure.
    template_nodes: Vec<Node>,
    /// This structure's own rows. The extension records, at project level.
    own_nodes: Vec<Node>,
    /// The merged result.
    merged: MergedTree,
}
```

`Effective` is built by loading the structure, then its template (if any),
then its own nodes, and running `domain::merge::merge`.

### Views

```rust
pub struct StructureView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

pub struct StructureSummaryView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// The structure's identity and level, without the tree.
pub struct StructureMetaView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// One of the structure's own records.
pub struct OwnNodeView {
    pub node_id: String,
    pub description: String,
    pub entry_name: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    /// Whether this record hides the node.
    pub hidden: bool,
    /// Whether this record overrides a node the template already has.
    pub overrides_template: bool,
}

pub struct StructureTreeView {
    pub structure_id: i64,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

pub struct NodeView {
    pub node_id: String,
    pub description: String,
    pub entry_name: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    /// The node's effective children.
    pub children: Vec<NodeView>,
}

pub struct MigrationView {
    pub directory: String,
    pub filter: Vec<String>,
}
```

`NodeView` deliberately carries **no `hidden` field**: the effective tree
never contains a hidden node, so the field would always be `false`.

Each view has a `From<…>` impl in `usecase/views.rs`.

## `apis::structures::StructureService` (the public-facing port)

New module `lib/crates/apis/src/structures.rs`, registered in `apis/src/lib.rs`.
The `apis` crate stays dependency-free and re-declares the value objects it
needs (`StructureLevel`, `MigrationView`, `NodeView`, …) exactly as
`apis/src/project.rs` does.

The trait mirrors the usecase's eleven methods:

```rust
#[async_trait]
pub trait StructureService: Send + Sync {
    async fn create_structure(&self, req: CreateStructureRequest) -> Result<StructureView, StructureApiError>;
    async fn get_structure(&self, id: i64) -> Result<StructureView, StructureApiError>;
    /// The structure's identity and level, without building the effective
    /// tree. The auth gate needs the level on every write, and building
    /// the whole tree to read one enum would be wasteful.
    async fn get_structure_meta(&self, id: i64) -> Result<StructureMetaView, StructureApiError>;
    async fn list_structures(&self, req: ListStructuresRequest) -> Result<StructureList, StructureApiError>;
    async fn rename_structure(&self, req: RenameStructureRequest) -> Result<StructureView, StructureApiError>;
    async fn delete_structure(&self, id: i64) -> Result<(), StructureApiError>;

    async fn add_node(&self, req: AddNodeRequest) -> Result<NodeView, StructureApiError>;
    async fn update_node(&self, req: UpdateNodeRequest) -> Result<NodeView, StructureApiError>;
    async fn delete_node(&self, req: DeleteNodeRequest) -> Result<(), StructureApiError>;

    /// This structure's own records — the extension records, with their
    /// `hidden` flag and whether each overrides a template node. This is
    /// where a client learns the `node_id` of a hidden node, so it can
    /// undo a hide.
    async fn list_own_nodes(&self, structure_id: i64) -> Result<OwnNodeList, StructureApiError>;

    async fn build_structure(&self, id: i64) -> Result<StructureTreeView, StructureApiError>;
}
```

`StructureApiError` mirrors `SdtmProjectApiError`'s shape: `Validation`,
`NotFound`, `StructureNotFound`, `NodeNotFound`, `TemplateNotFound`,
`ProjectNotFound`, `DuplicateName`, `DuplicateProjectStructure`,
`TemplateInUse`, `Repository`.

## Server HTTP surface

New module `apps/server/aegis-server/src/transport/http/structures/` with
`router.rs` and `handlers.rs`, following the `sdtm_project` module's layout
(one `routes!` call per HTTP verb, because `utoipa-axum` 0.2 panics when a
single `routes!` receives two handlers sharing a method).

The router is nested under `/api/structures` in `transport/http/router.rs`.

### Routes

| Method | Path | Handler | Success |
| --- | --- | --- | --- |
| POST | `/api/structures` | `create_structure` | 201 |
| GET | `/api/structures` | `list_structures` | 200 |
| GET | `/api/structures/{id}` | `get_structure` | 200 |
| GET | `/api/structures/{id}/meta` | `get_structure_meta` | 200 |
| PUT | `/api/structures/{id}` | `rename_structure` | 200 |
| DELETE | `/api/structures/{id}` | `delete_structure` | 204 |
| GET | `/api/structures/{id}/tree` | `build_structure` | 200 |
| GET | `/api/structures/{id}/nodes` | `list_own_nodes` | 200 |
| POST | `/api/structures/{id}/nodes` | `add_node` | 201 |
| PUT | `/api/structures/{id}/nodes/{node_id}` | `update_node` | 200 |
| DELETE | `/api/structures/{id}/nodes/{node_id}` | `delete_node` | 204 |

`GET /api/structures` takes optional `level` and `projectCode` query
parameters. With neither, every structure is listed. The response is wrapped
in a `structures` array so the shape can grow.

`GET /api/structures/{id}` returns the structure **and its effective tree** —
the caller never needs to know which nodes came from the template and which
from the project.

### Wire DTOs

`apps/server/aegis-server/src/transport/http/dto.rs` gains the `structures`
wire types, all `#[serde(rename_all = "camelCase")]` and
`#[derive(ToSchema)]`:

- `StructureLevelResponse` — `"template"` / `"project"`.
- `StructureViewResponse` — `id`, `name`, `level`, `projectCode`, `templateId`,
  `nodes`.
- `StructureSummaryResponse` — `id`, `name`, `level`, `projectCode`,
  `templateId`.
- `StructureMetaResponse` — `id`, `name`, `level`, `projectCode`,
  `templateId`.
- `StructureListResponse` — `{ "structures": [...] }`.
- `NodeViewResponse` — `nodeId`, `description`, `entryName`, `directory`,
  `parent`, `reference`, `migration`, `children`.
- `MigrationViewResponse` — `directory`, `filter`.
- `StructureTreeResponse` — `structureId`, `nodes`.
- `OwnNodeResponse` — `nodeId`, `description`, `entryName`, `directory`,
  `parent`, `reference`, `migration`, `hidden`, `overridesTemplate`.
- `OwnNodeListResponse` — `{ "nodes": [...] }`.
- `CreateStructureRequest` — `name`, `level`, `projectCode?`, `templateId?`.
- `RenameStructureRequest` — `name`.
- `AddNodeRequest` — `entryName`, `description`, `directory`, `parent`,
  `reference?`, `migration?`, `hidden?`.
- `UpdateNodeRequest` — `entryName`, `description`, `directory`, `parent`,
  `reference?`, `migration?`, `hidden?`.
- `DeleteNodeRequest` — `structureId`, `nodeId`.
- `ListStructuresQuery` — `level?`, `projectCode?`.

`AddNodeRequest` and `UpdateNodeRequest` are **full replacements**: `PUT`
overwrites the node's mutable attributes, so `parent: null` moves the node to
the root and `hidden: false` un-hides it. `hidden` defaults to `false` when
omitted.

`parent` is **not optional** on those two requests — it is `Option<String>` on
the wire but always present, and `null` means "at the root". A missing
`parent` is a malformed request, not an implicit "keep the current parent".

### Error mapping

`transport/http/error.rs` gains:

```rust
#[error("{0}")]
Structures(#[from] apis::structures::StructureApiError),
```

plus the `status()` / `code()` dispatch arms:

| `StructureApiError` | HTTP | `code` |
| --- | --- | --- |
| `Validation(_)` | 400 | `validation_failed` |
| `NotFound` | 404 | `not_found` |
| `StructureNotFound(_)` | 404 | `structure_not_found` |
| `NodeNotFound(_)` | 404 | `node_not_found` |
| `TemplateNotFound(_)` | 404 | `template_not_found` |
| `ProjectNotFound(_)` | 404 | `project_not_found` |
| `DuplicateName(_)` | 409 | `duplicate_name` |
| `DuplicateProjectStructure(_)` | 409 | `duplicate_project_structure` |
| `TemplateInUse(_)` | 409 | `template_in_use` |
| `Repository(_)` | 500 | `repository_error` |

### Auth policy

**Reads are open to any authenticated user.** Writes are gated by the
structure's level:

- A write against a **template** structure requires admin/root
  (`require_admin_or_root`, as the `domain-model` module does). Templates
  are global and affect every project.
- A write against a **project** structure is open to any authenticated user
  (the `crf` / `sdtm-project` precedent).

The handler resolves the structure's level before the write, so the gate is
per-target rather than per-route. It reads the level through
`get_structure_meta`, which returns the structure's identity without
building the effective tree — building the whole merged tree to read one
enum would be wasteful on every write.

Because a write can target either level, **every write route advertises a
`403` response** in its `#[utoipa::path]` block, and no read route does.
`POST /api/structures` has no target structure yet, so it gates on the
*requested* level: creating a template requires admin/root, creating a
project structure does not.

Project-membership gating is **not** implemented — deciding who may shape a
project's directory layout is a separate question, and no other crate in the
workspace gates on it.

### Server wiring

- `AppState` gains `pub structures: Arc<dyn apis::structures::StructureService>`.
  `AppState`'s test-support module gains a null double for it, so the other
  modules' test doubles stay compiling.
- `run.rs` gains `build_structure_service(pool, project)`:

```rust
fn build_structure_service(
    pool: PgPool,
    project: Arc<dyn apis::project::ProjectService>,
) -> Arc<dyn apis::structures::StructureService> {
    let structures = StructureRepoPg::new(pool.clone());
    let nodes = NodeRepoPg::new(pool);
    let projects = Arc::new(ProjectLookupImpl::new(project));
    let usecase = StructureUsecase::new(StructureUsecaseConfig {
        structures, nodes, projects,
    });
    Arc::new(StructureServiceImpl::new(usecase))
}
```

  The `project` service is already built in `run.rs` and is passed in, so the
  project-existence check reuses the canonical `projects` table.
- `transport/http/openapi.rs` registers the new schemas.
- `transport/http/router.rs` nests the new router under `/api/structures`.

## Tests

Per the workspace guideline (§9), in this order:

1. **Domain unit tests** — `src/domain/tests.rs`: the merge algorithm's five
   rules, worked end to end; `StructureLevel::as_str`; name normalisation;
   the validating constructors rejecting blank names and malformed names.
2. **Adapter unit tests** — `src/adapter/persistence/postgres/*_repo.rs`, each
   with a `#[cfg(test)] mod tests` that reads its migration file with
   `include_str!` and asserts the table, the unique constraints, the CHECK
   constraints and the `updated_at` trigger, so the schema cannot regress
   silently.
3. **Usecase tests** — `src/usecase/tests.rs`: in-memory fakes for both
   repositories plus a fake `ProjectLookup`, exercising each write rule
   (add / override / hide / duplicate name / cycle), the merge round trip and
   each validation failure.
4. **Facade tests** — `src/adapter/facade/in_memory/tests.rs`: the facade
   wired on top of in-memory ports, exercising the api-level round trip and
   the `UsecaseError → StructureApiError` mapping.
5. **`tests/public_api.rs`** — compile-only. Names every documented consumer
   import, pins the constructor chain (`fn(PgPool) -> _`, `fn(R) -> _` as
   function pointers) and asserts the trait bounds the usecase relies on.
6. **`tests/integration_persistence.rs`** — live-database round trips,
   `#[ignore]`-gated.

### Live-DB test harness

- Env var: `AEGIS_STRUCTURES_DATABASE_URL` (loaded via `dotenvy`).
- The test applies the crate's migrations, then **drops the live tables and
  the `_sqlx_migrations` bookkeeping table** so each run starts clean.
  Destructive against a real database on purpose.
- **The URL must point at a dedicated database, not the server's.** SQLx
  keeps a single `_sqlx_migrations` table *per database*, shared by every
  `--source` migration run.
- Run with `cargo test -p structures -- --ignored --test-threads=1`.

### Migrating the server's database

The server does not auto-migrate. To create the two `structures*` tables in
the server's database:

```bash
sqlx migrate run --source lib/crates/structures/migrations
```

run against `AEGIS_DATABASE_URL`.

## Dependencies (`Cargo.toml`)

```toml
[package]
name = "structures"
version = "0.1.0"
edition = "2024"

[dependencies]
sqlx = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
# `serde` + `serde_json` are required for the JSONB round-trip of the
# migration filter list.
serde = { workspace = true }
serde_json = { workspace = true }
# `nanoid` mints the `node_id` of a genuinely new node. The default
# 21-character length gives ~10^36 ids, so a collision needs no
# coordination — which matters because a node_id is unique across a
# template and every project structure that extends it.
nanoid = { workspace = true }
# `apis` provides the outbound `StructureService` port the facade
# implements. Path-dep because both crates share the workspace.
apis = { path = "../apis" }

[dev-dependencies]
dotenvy = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
```

`chrono` is **not** a dependency. Both tables carry `created_at` /
`updated_at` per the workspace convention, but a structure's timestamps
are audit metadata that no caller reads, so the domain model and the wire
do not surface them and the repositories never construct a
`DateTime<Utc>`.

## Workspace wiring

`Cargo.toml` `[workspace].members` gains `"lib/crates/structures"`.

The server's `Cargo.toml` gains
`structures = { path = "../../../lib/crates/structures" }`.

The `apis` crate gains no new dependency (the port is self-contained).

## `README.md`

`lib/crates/structures/README.md` covers: one-sentence purpose; the `src/`
tree; database setup (`sqlx migrate run --source lib/crates/structures/migrations`,
the `AEGIS_STRUCTURES_DATABASE_URL` env var, a constructor snippet); how to
run the ignored tests; and a back-link to
`docs/guidelines/lib-crate-development.md`.

The README must state prominently that `AEGIS_STRUCTURES_DATABASE_URL` has
to point at a **dedicated** database, because the live test drops
`_sqlx_migrations` — a table SQLx shares across every crate that migrates
that database.

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p structures --all-targets --all-features -- -D warnings
cargo test -p structures
cargo doc -p structures --no-deps
cargo clippy -p apis --all-targets --all-features -- -D warnings
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test -p structures -- --ignored --test-threads=1   # when AEGIS_STRUCTURES_DATABASE_URL is set
```

## Out of scope

- **The desktop app.** `apps/desktop/aegis-desktop` is not touched by this
  change — no Tauri commands, no TS feature module, no wire-DTO mirror in
  `src/shared/api/types.ts`. The desktop will consume this API in a later
  change, which is also where the structure gets materialised on disk.
- **Materialising a structure.** Creating directories, copying migration
  files and creating reference files is not this crate's job.
- **Seeding a built-in template.** Templates are created through the CRUD
  endpoints. The crate ships with an empty `structures` table.
- **Any UI.**
- **Pagination.** The list endpoint returns the full set, wrapped in a
  `structures` array so the response shape can grow later.
- **Re-pointing a project structure at a different template.** The template
  pointer is immutable after creation.
- **Project-membership gating on writes.**

## Open questions resolved with the requester

1. **Structure identity and ownership** — every structure carries its own
   `id`; a project-level structure carries the `project_code` of its owner;
   template structures are global.
2. **The join key is `node_id`, not `entry_name`** — so renaming an inherited
   node works and the template's original does not reappear. The *system*
   resolves which node a write refers to, from the name and the parent; the
   client never picks a `node_id`.
3. **`node_id` is a shared identity space** across a template and the project
   structures that extend it, so a new node can point its `parent` at an
   inherited template node.
4. **Overriding a node drops the template's children**; the effective
   children are the extension records under it. Adding a node under a
   template node that is *not* overridden leaves the template's other
   children intact.
5. **The crate does not touch the filesystem.** It records structures and
   describes what they should look like; the desktop materialises them
   later, in a separate change.
6. **Exactly one project-level structure per project.**
7. **Template structures stay editable** after a project structure extends
   them. Because the join key is `node_id`, an edit to the template changes
   what every extending project sees; an extension record whose `node_id` no
   longer exists in the template becomes an ordinary new node.
8. **Node mutations are node-level**, not whole-structure replacement.
9. **The read model is the merged tree, with the origin invisible.** Every
   node in it is addressable by `node_id`; mutating an inherited node makes
   the system create the override automatically.
10. **`DELETE` on an inherited node hides it** (writes a `hidden` override);
    `DELETE` on a recorded node removes the row.
11. **Auth** — template writes are admin/root only; project-structure writes
    are open to any authenticated user; reads are open to any authenticated
    user.

## Decisions made by the implementer

1. `created_at` / `updated_at` on both tables, `DEFAULT NOW()` plus a
   `BEFORE UPDATE` trigger — the workspace convention.
2. `node_id` is a 21-character `nanoid`; `nanoid` is already a workspace
   dependency.
3. `parent_id` is a soft reference (no FK), because it may name a node that
   lives in the template structure.
4. `migration` is stored as two columns (`migration_directory` TEXT,
   `migration_filter` JSONB) rather than one JSONB object, so it is
   inspectable in SQL.
5. A node's `entry_name` is its own name — one folder or one file name —
   and the tree is defined by the `parent` links. The full path of a node is
   the chain of names above it, and the desktop owns composing it. A
   directory rename therefore does not force a rename of the descendants.
6. `PUT` on a node is a **full replacement** of its mutable attributes, so
   `parent: null` moves the node to the root and `hidden: false` un-hides
   it.
7. `NodeView` carries no `hidden` field — the effective tree never contains
   a hidden node.
8. Writing `hidden = true` on a template structure's node is rejected
   (`400`), and writing it on a brand-new node is rejected (`400`) because it
   would be invisible.
9. `GET /api/structures/{id}` returns the structure *and* its effective
   tree, so the caller never needs to know where a node came from. A
   separate `GET …/tree` returns the tree alone.
10. Every **write** route advertises a `403` response and no read route
    does, because a write can target either level and the gate is resolved
    per target at run time. `POST /api/structures` gates on the requested
    level, so creating a template requires admin/root.
11. `parent` is always present in a `POST` / `PUT` node body; `null` means
    "at the root". Both operations are full replacements, so an omitted
    `parent` must never silently re-parent a node.
12. `GET /api/structures/{id}/nodes` returns the structure's own records
    with a `hidden` flag and an `overridesTemplate` marker, and
    `GET /api/structures/{id}/meta` returns the structure's identity and
    level without the tree. Together they make a hide reversible and keep
    the auth gate cheap.
13. The migrations are numbered `0001_` / `0002_`, matching every other
    crate. SQLx derives a migration's version from the filename's integer
    prefix and keeps one `_sqlx_migrations` table per database, so several
    crates' `--source` directories collide on version 1 in a shared
    database. That collision is pre-existing and workspace-wide; fixing it
    means renumbering eight crates and is out of scope here. The README
    states the dedicated-database requirement the other crates state.
