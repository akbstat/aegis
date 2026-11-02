# Structures Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `lib/crates/structures` — a business library that records the directory layout of a clinical project, merges a project structure's extension records over its template, and exposes the result through a new `apis` port and the server's HTTP API.

**Architecture:** A new DDD-layered Cargo crate (`domain` → `usecase` → `adapter`) following the workspace guideline. The heart of it is a pure `domain::merge::merge(template, own)` function that joins a template's nodes and a project structure's extension records on `node_id` — never on `path`. The usecase owns the write rules (which node a write refers to, when a write overrides an inherited node); the adapters are a Postgres persistence pair, a project-existence lookup, and a facade implementing the new `apis::structures::StructureService` port.

**Tech Stack:** Rust 2024 edition, Cargo workspace, SQLx 0.9 (runtime API, PostgreSQL), axum 0.8 + utoipa-axum 0.2 (server), async-trait, thiserror, serde/serde_json, chrono, nanoid 0.5.

**Spec:** [docs/superpowers/specs/2026-11-02-structures-crate-design.md](../specs/2026-11-02-structures-crate-design.md) — read it before starting; the plan argues from it and the spec carries the intent behind every decision below.

## Global Constraints

- The crate is named `structures`, lives at `lib/crates/structures`, and is registered in the root `Cargo.toml` `[workspace].members`.
- `edition = "2024"`, `resolver = "3"` (already set at the workspace root — do not change it).
- Every shared dependency is inherited with `{ workspace = true }`. Every non-obvious dependency carries a one-line `why` comment.
- Layer layout is `src/domain.rs` + `src/domain/`, `src/usecase.rs` + `src/usecase/`, `src/adapter.rs` + `src/adapter/`. **No `mod.rs`** — the 2024 edition deprecates it.
- Each layer module re-exports its public surface with `pub use …;` at the bottom. Consumers write `use crate::Foo`, never a nested path.
- Errors: one enum per layer (`DomainError`, `UsecaseError`), `#[derive(thiserror::Error)]`, every wrapped inner error carries `#[source]`, and `From<DomainError> for UsecaseError` exists so the usecase can `?` straight through.
- Aggregate constructors: `Foo::new(…)` validates and returns `Result<Self, DomainError>`; `Foo::for_repository(…)` is `pub(crate)` and skips validation.
- Ports that must be object-safe get `#[async_trait]` and a `Send + Sync` bound. Concrete types use plain `pub async fn`.
- SQLx **runtime API** (`sqlx::query_as`, `sqlx::query`, `FromRow`) — the workspace ships no `.sqlx/` cache and no build-time `DATABASE_URL`, so the compile-time macros are intentionally avoided. Every repo file carries a module-level comment saying so.
- Migrations live in `lib/crates/structures/migrations/`, one file per schema change, applied via `sqlx::migrate!("./migrations")`.
- `created_at` / `updated_at` use `DEFAULT NOW()` plus a per-table `BEFORE UPDATE` trigger that refreshes `updated_at`.
- Live-DB tests are `#[ignore]`-gated, load `.env` via `dotenvy`, read `AEGIS_STRUCTURES_DATABASE_URL`, and **drop the live tables and `_sqlx_migrations` before each run** — destructive on purpose.
- The `apis` crate stays dependency-free: it declares its own DTOs and never imports another workspace crate.
- Wire DTOs live in `apps/server/aegis-server/src/transport/http/dto.rs`, are `#[serde(rename_all = "camelCase")]` and `#[derive(ToSchema)]`, and mirror the Rust DTOs 1:1 (TS identifiers camelCase, wire snake_case is the server's business — the rename happens at the serde boundary).
- **The desktop app is not touched.** No Tauri command, no TS feature module, no change to `apps/desktop/aegis-desktop/src/shared/api/types.ts`.
- The crate performs **no filesystem I/O**. It records structures; it never creates directories or copies files.

## Review Focus

These are the inputs and conditions the spec implies but does not spell out. Each one gets a test in the task that owns the code.

1. **Renaming an inherited node must not resurrect the template's original.** The renamed project version wins; the template's node at the old path does not reappear alongside it. *(Task 2)*
2. **Overriding a template directory drops the template's children but keeps the project's own children under it.** The project's `raw/b.txt` survives; the template's `raw/a.txt` does not. *(Task 2)*
3. **Deleting a directory removes its whole subtree** from the effective structure, not just the directory. *(Task 5)*
4. **A template edited after a project extends it** leaves the extension record as a plain new node — it still appears, and the template's node does not come back. *(Task 2)*
5. **`POST` at a path the template already uses overrides the inherited node (201), not a duplicate; `POST` at a path this structure already records is 409.** *(Task 5)*

---

### Task 1: Crate scaffolding and the domain value objects

**Files:**
- Modify: `Cargo.toml` (workspace root) — add `"lib/crates/structures"` to `[workspace].members`
- Create: `lib/crates/structures/Cargo.toml`
- Create: `lib/crates/structures/src/lib.rs`
- Create: `lib/crates/structures/src/domain.rs`
- Create: `lib/crates/structures/src/domain/structure_level.rs`
- Create: `lib/crates/structures/src/domain/migration.rs`
- Create: `lib/crates/structures/src/domain/node.rs`
- Create: `lib/crates/structures/src/domain/error.rs`
- Create: `lib/crates/structures/src/domain/tests.rs`

**Interfaces:**
- Consumes: nothing — this is the first task.
- Produces:
  - `structures::StructureLevel` — `enum { Template, Project }` with `as_str(&self) -> &'static str` (`"template"` / `"project"`) and `TryFrom<&str>` (error `DomainError::InvalidStructureLevel`).
  - `structures::Migration` — `struct { pub directory: String, pub filter: Vec<String> }`.
  - `structures::NodeAttributes` — `struct { pub description: String, pub path: String, pub directory: bool, pub parent: Option<String>, pub reference: Option<String>, pub migration: Option<Migration> }`.
  - `structures::Node` — `struct { pub node_id: String, pub description: String, pub path: String, pub hidden: Option<bool>, pub directory: bool, pub parent: Option<String>, pub reference: Option<String>, pub migration: Option<Migration> }`.
  - `structures::Node::new(node_id: String, attrs: NodeAttributes) -> Result<Node, DomainError>` and `Node::for_repository(node_id: String, attrs: NodeAttributes) -> Node` (`pub(crate)`).
  - `structures::NodeAttributes::validate(&self) -> Result<(), DomainError>` — the single validation entry point.
  - `structures::DomainError` — the full enum from the spec's "`DomainError` variants" section.

- [ ] **Step 1: Write the failing domain tests**

Create `lib/crates/structures/src/domain/tests.rs` with these tests (they will not compile until the types exist — that is the point):

```rust
#[test]
fn structure_level_round_trips_through_its_str_form() {
    assert_eq!(StructureLevel::Template.as_str(), "template");
    assert_eq!(StructureLevel::Project.as_str(), "project");
    assert_eq!(StructureLevel::try_from("template").unwrap(), StructureLevel::Template);
    assert_eq!(StructureLevel::try_from("project").unwrap(), StructureLevel::Project);
    assert!(StructureLevel::try_from("Template").is_err());
}

#[test]
fn node_new_rejects_a_blank_path() {
    let attrs = NodeAttributes { path: "   ".into(), ..Default::default() };
    assert!(matches!(Node::new("n1".into(), attrs), Err(DomainError::InvalidPath(_))));
}

#[test]
fn node_new_rejects_an_unnormalised_path() {
    // leading slash, trailing slash, empty segment, "." segment, ".."
    // segment and a backslash are each rejected.
    for bad in ["/a", "a/", "a//b", "a/./b", "a/../b", "a\\b"] {
        let attrs = NodeAttributes { path: bad.into(), ..Default::default() };
        assert!(matches!(Node::new("n1".into(), attrs), Err(DomainError::InvalidPath(_))), "path {bad} should be rejected");
    }
}

#[test]
fn node_new_rejects_a_reference_on_a_directory() {
    let attrs = NodeAttributes {
        path: "a.txt".into(), directory: false,
        reference: Some("/abs/a.txt".into()), ..Default::default()
    };
    assert!(matches!(Node::new("n1".into(), attrs), Err(DomainError::ReferenceOnDirectory)));
}

#[test]
fn node_new_rejects_a_migration_on_a_file() {
    let attrs = NodeAttributes {
        path: "a".into(), directory: true,
        migration: Some(Migration { directory: "/src".into(), filter: vec![".sas".into()] }),
        ..Default::default()
    };
    assert!(matches!(Node::new("n1".into(), attrs), Err(DomainError::MigrationOnFile)));
}

#[test]
fn node_new_accepts_a_well_formed_node() {
    let attrs = NodeAttributes {
        description: "the raw folder".into(), path: "raw/b.txt".into(), directory: false,
        parent: Some("t_root".into()), reference: Some("/abs/b.txt".into()),
        migration: None,
    };
    let node = Node::new("n1".into(), attrs).unwrap();
    assert_eq!(node.node_id, "n1");
    assert_eq!(node.path, "raw/b.txt");
    assert_eq!(node.hidden, None);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p structures --lib 2>&1 | head -30`
Expected: FAIL — `cargo` cannot find package `structures` (the crate is not registered yet).

- [ ] **Step 3: Register the crate in the workspace**

In the root `Cargo.toml`, add `"lib/crates/structures"` to `[workspace].members`. The list is grouped two-per-line in places; keep it readable:

```toml
members = [
    "apps/desktop/aegis-desktop/src-tauri",
    "apps/server/aegis-server",
    "lib/crates/apis",
    "lib/crates/auth", "lib/crates/crf", "lib/crates/domain-model",
    "lib/crates/logging-utils", "lib/crates/mission",
    "lib/crates/project", "lib/crates/sdtm-project",
    "lib/crates/structures", "lib/crates/terminology",
    "lib/crates/user",
    "lib/crates/windows-utils",
]
```

- [ ] **Step 4: Create `lib/crates/structures/Cargo.toml`**

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

`sqlx` and `serde_json` are not used until Tasks 6 and 7. They stay in `[dependencies]` because the Postgres adapters land in this crate's own tree, not behind a feature flag; drop them only if `cargo build -p structures` proves them unused at the end.

**`chrono` is deliberately absent.** The `structures` and `structure_nodes` tables carry `created_at` / `updated_at` per the workspace convention, but the domain model and the wire do not surface them — a structure's timestamps are audit metadata no caller reads. The repositories select only the columns they map, so no `DateTime<Utc>` is ever constructed.

- [ ] **Step 5: Implement the domain types**

`src/domain/structure_level.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Which kind of structure this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureLevel {
    /// A global, shared layout. Many may exist.
    Template,
    /// The layout of exactly one project. Extends exactly one template.
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

impl TryFrom<&str> for StructureLevel {
    type Error = DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "template" => Ok(StructureLevel::Template),
            "project" => Ok(StructureLevel::Project),
            other => Err(DomainError::InvalidStructureLevel(other.to_string())),
        }
    }
}
```

`src/domain/migration.rs`:

```rust
/// What to copy into a directory once it has been created.
///
/// `directory` is an absolute folder path; `filter` holds the target
/// suffixes — only files whose name ends with one of them are copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub directory: String,
    pub filter: Vec<String>,
}
```

`src/domain/node.rs` — holds `NodeAttributes`, `Node`, and the path rules:

```rust
/// The mutable attributes of a node, as supplied by a caller.
#[derive(Debug, Clone, Default)]
pub struct NodeAttributes {
    pub description: String,
    pub path: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<Migration>,
}

/// One entry in a structure — a folder or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Unique inside one structure. **Shared identity space**: a project
    /// structure's extension record for an inherited node carries the
    /// template node's `node_id`.
    pub node_id: String,
    pub description: String,
    pub path: String,
    /// When true the node is filtered out of the effective structure.
    /// Only meaningful at project level.
    pub hidden: Option<bool>,
    pub directory: bool,
    pub parent: Option<String>,
    /// Absolute file path. Only meaningful when `directory` is false.
    pub reference: Option<String>,
    /// What to copy into this directory. Only meaningful when `directory`
    /// is true.
    pub migration: Option<Migration>,
}
```

`NodeAttributes::validate` enforces, in this order, each returning its own `DomainError` variant:

1. `path` is non-blank after trimming → else `InvalidPath(path)`.
2. `path` does not start or end with `/` → else `InvalidPath(path)`.
3. `path` contains no `\` → else `InvalidPath(path)`.
4. every `/`-separated segment is non-empty and is neither `.` nor `..` → else `InvalidPath(path)`.
5. `reference.is_some()` requires `directory == false` → else `ReferenceOnDirectory`.
6. `migration.is_some()` requires `directory == true` → else `MigrationOnFile`.

`Node::new(node_id, attrs)` calls `attrs.validate()?` and then builds the `Node` with `hidden: None`. `Node::for_repository(node_id, attrs)` skips validation and is `pub(crate)`.

- [ ] **Step 6: Implement `DomainError`**

`src/domain/error.rs` — copy the enum verbatim from the spec's "`DomainError` variants" section, and add the two variants the value objects need:

```rust
#[error("invalid structure level: {0}")]
InvalidStructureLevel(String),
```

(`InvalidPath` is already in the spec's list; it takes the offending path.)

- [ ] **Step 7: Wire the module tree**

`src/lib.rs`:

```rust
//! # structures crate
//!
//! Workspace library that records the **directory structure** of a
//! clinical project: the set of folders and files that should exist on
//! disk, and the rule that lets a project *extend* a shared template of
//! that layout.
//!
//! The crate performs no filesystem I/O. It records what a structure
//! should look like; materialising it is the desktop app's job.

pub mod domain;

pub use domain::{
    DomainError, Migration, Node, NodeAttributes, StructureLevel,
};
```

`src/domain.rs`:

```rust
mod error;
mod migration;
mod node;
mod structure_level;

#[cfg(test)]
mod tests;

pub use error::DomainError;
pub use migration::Migration;
pub use node::{Node, NodeAttributes};
pub use structure_level::StructureLevel;
```

`src/domain/tests.rs` starts with:

```rust
use super::error::DomainError;
use super::migration::Migration;
use super::node::{Node, NodeAttributes};
use super::structure_level::StructureLevel;
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p structures --lib`
Expected: PASS — 6 tests.

- [ ] **Step 9: Verify the workspace still builds**

Run: `cargo check --workspace 2>&1 | tail -20`
Expected: no errors.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml lib/crates/structures
git commit -m "feat(structures): scaffold the crate and its domain value objects

Adds lib/crates/structures with the StructureLevel / Migration / Node
value objects, the path and attribute validation rules, and the
DomainError enum.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: The merge algorithm

**Files:**
- Create: `lib/crates/structures/src/domain/merge.rs`
- Modify: `lib/crates/structures/src/domain.rs` — add `mod merge;` and re-exports
- Modify: `lib/crates/structures/src/lib.rs` — re-export `MergedNode`, `MergedTree`
- Modify: `lib/crates/structures/src/domain/tests.rs` — or a dedicated `src/domain/merge/tests.rs`

**Interfaces:**
- Consumes: `Node`, `Migration`, `DomainError` from Task 1.
- Produces:
  - `structures::MergedNode` — `struct { pub node: Node, pub children: Vec<MergedNode> }`.
  - `structures::MergedTree` — `struct { pub roots: Vec<MergedNode> }`.
  - `structures::merge(template: &[Node], own: &[Node]) -> MergedTree` — a **total** function; it never fails and never panics.

- [ ] **Step 1: Write the failing merge tests**

Create `lib/crates/structures/src/domain/merge/tests.rs` (make `merge.rs` a directory module: `src/domain/merge.rs` declares `mod tests;` and the tests live in `src/domain/merge/tests.rs`).

Build the fixture with a helper so each test reads as a scenario:

```rust
/// A node with only the fields a test cares about.
fn node(node_id: &str, path: &str, directory: bool, parent: Option<&str>) -> Node {
    Node {
        node_id: node_id.to_string(),
        description: String::new(),
        path: path.to_string(),
        hidden: None,
        directory,
        parent: parent.map(str::to_string),
        reference: None,
        migration: None,
    }
}

/// The effective `path`s of a merged subtree, depth-first, so a test can
/// assert on the shape of the tree without spelling out the nesting.
fn paths(node: &MergedNode) -> Vec<String> {
    let mut out = vec![node.node.path.clone()];
    for child in &node.children {
        out.extend(paths(child));
    }
    out
}
```

Tests:

```rust
#[test]
fn a_template_alone_merges_into_itself() {
    let template = vec![node("t_root", "raw", true, None), node("t_file", "raw/a.txt", false, Some("t_root"))];
    let merged = merge(&template, &[]);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(paths(&merged.roots[0]), vec!["raw", "raw/a.txt"]);
}

#[test]
fn an_override_replaces_the_template_node_and_drops_the_template_children() {
    // Review Focus #2
    let template = vec![node("t_root", "raw", true, None), node("t_file", "raw/a.txt", false, Some("t_root"))];
    let mut own = node("t_root", "raw", true, None);
    own.description = "project raw".into();
    let merged = merge(&template, &[own]);
    assert_eq!(merged.roots.len(), 1);
    // the template's child `raw/a.txt` is gone
    assert_eq!(paths(&merged.roots[0]), vec!["raw"]);
    assert_eq!(merged.roots[0].node.description, "project raw");
}

#[test]
fn an_override_keeps_the_projects_own_children() {
    // Review Focus #2
    let template = vec![node("t_root", "raw", true, None), node("t_file", "raw/a.txt", false, Some("t_root"))];
    let own = vec![node("t_root", "raw", true, None), node("n_new", "raw/b.txt", false, Some("t_root"))];
    let merged = merge(&template, &own);
    assert_eq!(paths(&merged.roots[0]), vec!["raw", "raw/b.txt"]);
}

#[test]
fn adding_under_a_template_node_keeps_the_templates_other_children() {
    let template = vec![node("t_root", "raw", true, None), node("t_file", "raw/a.txt", false, Some("t_root"))];
    let own = vec![node("n_new", "raw/b.txt", false, Some("t_root"))];
    let merged = merge(&template, &own);
    // both the template's child and the new one are present, sorted by path
    assert_eq!(paths(&merged.roots[0]), vec!["raw", "raw/a.txt", "raw/b.txt"]);
}

#[test]
fn a_hidden_node_and_its_subtree_are_filtered_out() {
    let template = vec![node("t_doc", "docs", true, None), node("t_sub", "docs/x", false, Some("t_doc"))];
    let mut own = node("t_doc", "docs", true, None);
    own.hidden = Some(true);
    let merged = merge(&template, &[own]);
    assert!(merged.roots.is_empty());
}

#[test]
fn renaming_an_inherited_node_wins_and_does_not_resurrect_the_template_original() {
    // Review Focus #1
    let template = vec![node("t_root", "raw", true, None)];
    let mut own = node("t_root", "renamed", true, None);
    own.path = "renamed".into();
    let merged = merge(&template, &[own]);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(merged.roots[0].node.path, "renamed");
    // the template's `raw` did not come back
    assert_eq!(paths(&merged.roots[0]), vec!["renamed"]);
}

#[test]
fn an_own_record_whose_node_id_is_unknown_to_the_template_is_a_plain_new_node() {
    // Review Focus #4 — the template was edited and the record no longer
    // matches anything in it.
    let template = vec![node("t_other", "other", true, None)];
    let own = vec![node("n_new", "raw", true, None)];
    let merged = merge(&template, &own);
    let mut all: Vec<String> = merged.roots.iter().flat_map(|r| paths(r)).collect();
    all.sort();
    assert_eq!(all, vec!["other", "raw"]);
}

#[test]
fn the_merged_tree_is_sorted_by_path() {
    let template = vec![node("t_z", "zeta", true, None), node("t_a", "alpha", true, None)];
    let merged = merge(&template, &[]);
    let all: Vec<String> = merged.roots.iter().map(|r| r.node.path.clone()).collect();
    assert_eq!(all, vec!["alpha", "zeta"]);
}

#[test]
fn a_node_whose_parent_is_absent_is_treated_as_a_root() {
    let own = vec![node("n_orphan", "orphan", true, Some("t_missing"))];
    let merged = merge(&[], &own);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(merged.roots[0].node.path, "orphan");
}

#[test]
fn a_cycle_does_not_recurse_forever() {
    let own = vec![node("a", "a", true, Some("b")), node("b", "b", true, Some("a"))];
    let merged = merge(&[], &own);
    // the merge terminates; every node appears at most once
    let mut all: Vec<String> = merged.roots.iter().flat_map(|r| paths(r)).collect();
    all.sort();
    all.dedup();
    assert!(all.len() <= 2);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p structures --lib 2>&1 | head -30`
Expected: FAIL — `merge` and `MergedNode` are not defined.

- [ ] **Step 3: Implement `merge`**

`src/domain/merge.rs`:

```rust
//! The merge algorithm.
//!
//! Joins a template's nodes with a project structure's extension
//! records on **`node_id`**, never on `path`. See the spec's "The merge
//! algorithm" section for the five rules this implements.

use std::collections::{HashMap, HashSet};

use super::node::Node;

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
```

The algorithm, in order:

1. Build `attrs: HashMap<&str, &Node>` from `template`, then overwrite each entry with the matching entry from `own`. An own record therefore wins on a shared `node_id`.
2. Build `overridden: HashSet<&str>` = the set of `node_id`s that `own` mentions. Every node in `overridden` is a node this structure has a record for.
3. Build `template_children: HashMap<&str, Vec<&Node>>` and `own_children: HashMap<&str, Vec<&Node>>`, keyed on the `parent` of each node. A node with `parent == None` is not in either map.
4. The **effective children** of a node with `node_id` `X`:
   - if `X` is in `overridden` → `own_children[X]` only (the template's children under an overridden node are dropped);
   - otherwise → `template_children[X]` followed by `own_children[X]`, deduplicated by `node_id` with the own record winning. Since an own record that overrides a template child shares that child's `node_id`, the child appears once.
5. The **roots** are the nodes in `attrs` whose `parent` is `None`, plus any node whose `parent` names an id that is not in `attrs` (treated as a root rather than silently lost).
6. Build the tree recursively from the roots. A node whose `node.hidden == Some(true)` is **skipped**, and its subtree is skipped with it. The recursion carries a `visited: HashSet<&str>`; a node already visited is skipped, so a cycle in `parent` cannot recurse forever.
7. Sort every children list and the root list by `node.path` ascending.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p structures --lib`
Expected: PASS — 16 tests.

- [ ] **Step 5: Re-export from the crate root**

`src/domain.rs` gains `mod merge;` and:

```rust
pub use merge::{merge, MergedNode, MergedTree};
```

`src/lib.rs` gains `MergedNode, MergedTree` to its `pub use domain::{…}` list.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/structures
git commit -m "feat(structures): add the merge algorithm

Pure domain::merge::merge joins a template's nodes with a project
structure's extension records on node_id. Overriding a node drops the
template's children and keeps the project's own; hidden nodes and their
subtrees are filtered out; the output is sorted by path.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: The domain ports and the usecase DTOs

**Files:**
- Create: `lib/crates/structures/src/domain/structure.rs`
- Create: `lib/crates/structures/src/domain/repository.rs`
- Create: `lib/crates/structures/src/domain/node_repository.rs`
- Create: `lib/crates/structures/src/domain/project_lookup.rs`
- Modify: `lib/crates/structures/src/domain.rs`
- Modify: `lib/crates/structures/src/lib.rs`
- Create: `lib/crates/structures/src/usecase.rs`
- Create: `lib/crates/structures/src/usecase/commands.rs`
- Create: `lib/crates/structures/src/usecase/views.rs`
- Create: `lib/crates/structures/src/usecase/error.rs`
- Modify: `lib/crates/structures/src/lib.rs` — declare `pub mod usecase;`

**Interfaces:**
- Consumes: `Node`, `NodeAttributes`, `Migration`, `StructureLevel`, `DomainError`, `MergedTree` from Tasks 1–2.
- Produces:
  - `structures::Structure` — `struct { pub id: i64, pub name: String, pub level: StructureLevel, pub project_code: Option<String>, pub template_id: Option<i64>, pub nodes: Vec<Node> }`.
  - `structures::StructureNew` — `struct { pub name: String, pub level: StructureLevel, pub project_code: Option<String>, pub template_id: Option<i64> }`.
  - `structures::StructureFilter` — `struct { pub level: Option<StructureLevel>, pub project_code: Option<String> }`.
  - `structures::StructureRepository` — trait, methods listed below.
  - `structures::NodeRepository` — trait, methods listed below.
  - `structures::ProjectLookup` — trait, one method `get_by_code`.
  - `structures::CreateStructure`, `RenameStructure`, `AddNode`, `UpdateNode`, `DeleteNode` — command DTOs.
  - `structures::StructureView`, `StructureSummaryView`, `StructureTreeView`, `NodeView`, `MigrationView` — view DTOs.
  - `structures::UsecaseError` — `enum { Validation(#[source] DomainError), Repository(#[source] DomainError) }` plus `From<DomainError>`.

- [ ] **Step 1: Define the aggregate**

`src/domain/structure.rs`:

```rust
/// A directory layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    /// The owning project. Set at project level, absent at template level.
    pub project_code: Option<String>,
    /// The template this structure extends. Set at project level, absent
    /// at template level. Immutable once the structure is created.
    pub template_id: Option<i64>,
    /// The structure's own rows. At project level these are the
    /// *extension records* — the difference from the template.
    pub nodes: Vec<Node>,
}

/// A structure that has not been persisted yet.
#[derive(Debug, Clone)]
pub struct StructureNew {
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// The filter for listing structures.
#[derive(Debug, Clone, Default)]
pub struct StructureFilter {
    pub level: Option<StructureLevel>,
    pub project_code: Option<String>,
}
```

- [ ] **Step 2: Define the ports**

`src/domain/repository.rs`:

```rust
use async_trait::async_trait;

use super::error::DomainError;
use super::structure::{Structure, StructureFilter, StructureNew};

/// Outbound port for persistence of `Structure` aggregates.
#[async_trait]
pub trait StructureRepository: Send + Sync {
    async fn create(&self, input: StructureNew) -> Result<Structure, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<Structure, DomainError>;
    async fn list(&self, filter: StructureFilter) -> Result<Vec<Structure>, DomainError>;
    async fn rename(&self, id: i64, name: &str) -> Result<Structure, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    /// How many project-level structures the given project already has.
    async fn count_project_structures(&self, project_code: &str) -> Result<i64, DomainError>;
    /// Whether any project-level structure still points at this template.
    async fn is_template_referenced(&self, template_id: i64) -> Result<bool, DomainError>;
}
```

`src/domain/node_repository.rs`:

```rust
use async_trait::async_trait;

use super::error::DomainError;
use super::node::Node;

/// Outbound port for persistence of `Node` rows.
///
/// A node row is always addressed by the `structure_id` that owns it plus
/// the node's `node_id`. `parent_id` is a **soft** reference: it may name
/// a node that lives in the *template* structure, so it cannot be a
/// database foreign key.
#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn insert(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError>;
    async fn update(&self, structure_id: i64, node: &Node) -> Result<Node, DomainError>;
    async fn delete(&self, structure_id: i64, node_id: &str) -> Result<(), DomainError>;
    /// `Ok(None)` when the structure has no row with that `node_id`.
    async fn find_by_id(&self, structure_id: i64, node_id: &str) -> Result<Option<Node>, DomainError>;
    async fn list(&self, structure_id: i64) -> Result<Vec<Node>, DomainError>;
    /// `Ok(None)` when the structure has no row with that path.
    async fn find_by_path(&self, structure_id: i64, path: &str) -> Result<Option<Node>, DomainError>;
}
```

`src/domain/project_lookup.rs` — copy `sdtm-project`'s file verbatim, changing only the doc comment to name this crate:

```rust
use async_trait::async_trait;

use super::error::DomainError;

/// Narrow cross-crate port. Returns `Ok(())` if the project exists,
/// `Err(DomainError::ProjectNotFound)` if not, or
/// `Err(DomainError::Repository(msg))` for any other failure.
#[async_trait]
pub trait ProjectLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
}
```

- [ ] **Step 3: Define the usecase command DTOs**

`src/usecase/commands.rs`:

```rust
use crate::domain::{Migration, StructureLevel};

#[derive(Debug, Clone)]
pub struct CreateStructure {
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RenameStructure {
    pub id: i64,
    pub name: String,
}

/// Place a node at a path. The usecase resolves the path against the
/// effective structure and decides whether this creates a new node or
/// overrides an inherited one.
#[derive(Debug, Clone)]
pub struct AddNode {
    pub structure_id: i64,
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<Migration>,
    pub hidden: Option<bool>,
}

/// Replace a node's mutable attributes, addressed by `node_id`.
#[derive(Debug, Clone)]
pub struct UpdateNode {
    pub structure_id: i64,
    pub node_id: String,
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<Migration>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct DeleteNode {
    pub structure_id: i64,
    pub node_id: String,
}
```

- [ ] **Step 4: Define the usecase view DTOs**

`src/usecase/views.rs`:

```rust
use crate::domain::{Migration, StructureLevel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureSummaryView {
    pub id: i64,
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTreeView {
    pub structure_id: i64,
    /// The effective tree's roots.
    pub nodes: Vec<NodeView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeView {
    pub node_id: String,
    pub description: String,
    pub path: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    /// The node's effective children.
    pub children: Vec<NodeView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationView {
    pub directory: String,
    pub filter: Vec<String>,
}
```

`NodeView` deliberately carries **no `hidden` field** — the effective tree never contains a hidden node.

- [ ] **Step 5: Define `UsecaseError`**

`src/usecase/error.rs` — mirror `sdtm-project`'s file exactly:

```rust
use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(#[source] DomainError),

    #[error("repository error: {0}")]
    Repository(#[source] DomainError),
}

impl From<DomainError> for UsecaseError {
    fn from(err: DomainError) -> Self {
        // Validation errors that originated upstream of the repository
        // already came through `UsecaseError::Validation`; everything
        // else surfaces as `Repository`.
        UsecaseError::Repository(err)
    }
}
```

- [ ] **Step 6: Wire the modules**

`src/usecase.rs`:

```rust
mod commands;
mod error;
mod views;

pub use commands::{AddNode, CreateStructure, DeleteNode, RenameStructure, UpdateNode};
pub use error::UsecaseError;
pub use views::{MigrationView, NodeView, StructureSummaryView, StructureTreeView, StructureView};
```

`src/domain.rs` gains `mod node_repository;`, `mod project_lookup;`, `mod repository;`, `mod structure;` and the re-exports:

```rust
pub use node_repository::NodeRepository;
pub use project_lookup::ProjectLookup;
pub use repository::StructureRepository;
pub use structure::{Structure, StructureFilter, StructureNew};
```

`src/lib.rs` gains `pub mod usecase;` and re-exports every new public name.

- [ ] **Step 7: Verify it compiles**

Run: `cargo build -p structures 2>&1 | tail -20`
Expected: compiles. `cargo clippy -p structures --all-targets --all-features -- -D warnings` must also be clean.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/structures
git commit -m "feat(structures): add the domain ports and the usecase DTOs

StructureRepository, NodeRepository and ProjectLookup ports; the
Structure aggregate; and the command / view DTOs the usecase will
consume and produce.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: The usecase — structure CRUD and the build

**Files:**
- Create: `lib/crates/structures/src/usecase/structure_usecase.rs`
- Create: `lib/crates/structures/src/usecase/tests.rs`
- Modify: `lib/crates/structures/src/usecase.rs`
- Modify: `lib/crates/structures/src/lib.rs`

**Interfaces:**
- Consumes: every type from Task 3.
- Produces:
  - `structures::StructureUsecase<S, N, P>` — `pub struct` with `pub fn new(StructureUsecaseConfig<S, N, P>) -> Self`.
  - `structures::StructureUsecaseConfig<S, N, P>` — `pub struct { pub structures: S, pub nodes: N, pub projects: Arc<P> }`.
  - Methods: `create_structure`, `get_structure`, `list_structures`, `rename_structure`, `delete_structure`, `build_structure`.

- [ ] **Step 1: Write the failing usecase tests**

Create `src/usecase/tests.rs` with in-memory fakes for all three ports. The fakes are the backbone of Tasks 4 and 5, so write them once, carefully:

```rust
use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::domain::{
    DomainError, Node, NodeRepository, ProjectLookup, Structure, StructureFilter, StructureLevel,
    StructureNew, StructureRepository,
};
```

`FakeStructureRepo` holds `Mutex<HashMap<i64, Structure>>` and a `Mutex<i64>` counter for ids; `FakeNodeRepo` holds `Mutex<HashMap<(i64, String), Node>>`; `FakeProjectLookup` holds a `Mutex<Vec<String>>` of existing project codes.

Then the tests:

```rust
#[tokio::test]
async fn creating_a_template_structure_stores_it_with_no_owner() { /* assert the repo's structure has level Template, project_code None, template_id None */ }

#[tokio::test]
async fn creating_a_project_structure_without_a_template_is_rejected() { /* assert Err(Validation(DomainError::MissingTemplate)) */ }

#[tokio::test]
async fn creating_a_project_structure_without_a_project_code_is_rejected() { /* assert Err(Validation(DomainError::MissingProjectCode)) */ }

#[tokio::test]
async fn creating_a_template_structure_with_an_owner_is_rejected() { /* assert Err(Validation(DomainError::UnexpectedOwner)) */ }

#[tokio::test]
async fn creating_a_project_structure_for_an_unknown_project_is_rejected() { /* assert Err(Repository(DomainError::ProjectNotFound("nope".into()))) */ }

#[tokio::test]
async fn creating_a_second_project_structure_for_the_same_project_is_rejected() { /* assert Err(Repository(DomainError::DuplicateProjectStructure(..))) */ }

#[tokio::test]
async fn renaming_a_structure_changes_only_its_name() { /* assert the repo's structure kept its level / project_code / template_id */ }

#[tokio::test]
async fn deleting_a_template_that_is_still_referenced_is_rejected() { /* assert Err(Repository(DomainError::TemplateInUse(..))) */ }

#[tokio::test]
async fn building_a_template_structure_returns_its_own_nodes_as_a_tree() { /* assert the tree's paths */ }

#[tokio::test]
async fn building_a_project_structure_returns_the_merged_tree() { /* assert the template's node and the project's addition are both present */ }
```

The `assert` on an error variant uses `matches!`:

```rust
assert!(matches!(err, UsecaseError::Validation(DomainError::MissingTemplate)));
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p structures --lib 2>&1 | head -30`
Expected: FAIL — `StructureUsecase` is not defined.

- [ ] **Step 3: Implement the usecase**

`src/usecase/structure_usecase.rs`:

```rust
use std::sync::Arc;

use crate::domain::{
    merge, DomainError, MergedTree, Node, NodeRepository, ProjectLookup, Structure,
    StructureFilter, StructureLevel, StructureNew, StructureRepository,
};
use crate::usecase::{
    CreateStructure, RenameStructure, StructureSummaryView, StructureTreeView, StructureView,
    UsecaseError,
};

pub struct StructureUsecaseConfig<S, N, P> {
    pub structures: S,
    pub nodes: N,
    /// `Arc<P>` rather than `P`, so the call site can hand over an
    /// `Arc<ProjectLookupImpl>` directly.
    pub projects: Arc<P>,
}

pub struct StructureUsecase<S, N, P> {
    structures: S,
    nodes: N,
    projects: Arc<P>,
}
```

`StructureUsecase::new(config)` moves the three fields into the usecase.

The **effective structure** helper — a private method on the usecase that every read and write needs:

```rust
/// The three node sets a read or a write is resolved against.
struct Effective {
    structure: Structure,
    /// The template's nodes. Empty for a template-level structure.
    template_nodes: Vec<Node>,
    /// This structure's own rows — the extension records at project level.
    own_nodes: Vec<crate::domain::Node>,
    /// The merged result.
    merged: MergedTree,
}
```

`async fn effective(&self, structure_id: i64) -> Result<Effective, UsecaseError>`:
1. `let structure = self.structures.find_by_id(structure_id).await?;` — a missing row surfaces as `DomainError::StructureNotFound(id)`.
2. `let template_nodes = match structure.template_id { Some(tid) => { let t = self.structures.find_by_id(tid).await.map_err(|_| DomainError::TemplateNotFound(tid))?; self.nodes.list(t.id).await? } None => vec![] };`
   — a template that no longer exists is `TemplateNotFound(tid)`, not the generic `StructureNotFound`, because the caller asked for a *template*.
3. `let own_nodes = self.nodes.list(structure.id).await?;`
4. `let merged = merge(&template_nodes, &own_nodes);`

The six methods:

- **`create_structure(cmd)`** — validate, then insert.
  1. `cmd.name.trim().is_empty()` → `Validation(DomainError::EmptyName)`.
  2. Match on `cmd.level`:
     - `Template` → `project_code.is_some()` or `template_id.is_some()` → `Validation(DomainError::UnexpectedOwner)`.
     - `Project` → `project_code` is `None` or blank → `Validation(DomainError::MissingProjectCode)`; `template_id` is `None` → `Validation(DomainError::MissingTemplate)`.
  3. At project level: `self.projects.get_by_code(project_code).await?` — the lookup's own error carries through. Then `self.structures.find_by_id(template_id).await.map_err(|_| DomainError::TemplateNotFound(template_id))?` must yield a structure whose `level` is `Template`, else `Validation(DomainError::NotATemplate(template_id))`.
  4. `self.structures.count_project_structures(project_code).await? > 0` → `Repository(DomainError::DuplicateProjectStructure(project_code))`.
  5. `self.structures.create(StructureNew { … }).await?`
  6. Return the view with an empty tree (a fresh structure has no nodes): `StructureView { id, name, level, project_code, template_id, nodes: vec![] }`.

- **`get_structure(id)`** — `let eff = self.effective(id).await?;` then build the `StructureView` from `eff.structure` and `eff.merged.roots`.

- **`list_structures(level, project_code)`** — `self.structures.list(StructureFilter { level, project_code })` → map each to a `StructureSummaryView`. **No nodes are loaded** for a list.

- **`rename_structure(cmd)`** — validate the name is non-blank, then `self.structures.rename(cmd.id, &cmd.name).await?`, then return the full view via `effective`.

- **`delete_structure(id)`** — load the structure; if its `level` is `Template` and `self.structures.is_template_referenced(id).await?` → `Repository(DomainError::TemplateInUse(id))`. Otherwise `self.structures.delete(id).await?`. The node rows go with it via `ON DELETE CASCADE`.

- **`build_structure(id)`** — `let eff = self.effective(id).await?;` then `StructureTreeView { structure_id: eff.structure.id, nodes: eff.merged.roots.into_iter().map(Into::into).collect() }`.

The `MergedNode → NodeView` conversion lives in `usecase/views.rs`:

```rust
impl From<crate::domain::MergedNode> for NodeView {
    fn from(m: crate::domain::MergedNode) -> Self {
        Self {
            node_id: m.node.node_id,
            description: m.node.description,
            path: m.node.path,
            directory: m.node.directory,
            parent: m.node.parent,
            reference: m.node.reference,
            migration: m.node.migration.map(|mig| MigrationView { directory: mig.directory, filter: mig.filter }),
            children: m.children.into_iter().map(Into::into).collect(),
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p structures --lib`
Expected: PASS — 26 tests.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/structures
git commit -m "feat(structures): add the structure usecase

CRUD over structures plus build_structure, which returns the effective
tree — the template's nodes merged with the project structure's
extension records.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: The usecase — node writes

**Files:**
- Modify: `lib/crates/structures/src/usecase/structure_usecase.rs` — add `add_node`, `update_node`, `delete_node` and their private helpers
- Modify: `lib/crates/structures/src/usecase/tests.rs` — add the node-write tests

**Interfaces:**
- Consumes: everything from Task 4.
- Produces:
  - `StructureUsecase::add_node(&self, cmd: AddNode) -> Result<NodeView, UsecaseError>`
  - `StructureUsecase::update_node(&self, cmd: UpdateNode) -> Result<NodeView, UsecaseError>`
  - `StructureUsecase::delete_node(&self, cmd: DeleteNode) -> Result<(), UsecaseError>`

- [ ] **Step 1: Write the failing node-write tests**

Add to `src/usecase/tests.rs`:

```rust
#[tokio::test]
async fn adding_a_node_at_a_new_path_mints_a_node_id() { /* assert the repo holds one node with a non-empty node_id */ }

#[tokio::test]
async fn adding_a_node_at_a_path_the_template_uses_overrides_the_inherited_node() {
    // Review Focus #5
    /* template has `raw`; POST at path `raw` → the repo holds ONE own record whose node_id equals the template's node_id */
}

#[tokio::test]
async fn adding_a_node_at_a_path_this_structure_already_records_is_rejected() {
    // Review Focus #5
    /* assert Err(Repository(DomainError::DuplicatePath(path))) */
}

#[tokio::test]
async fn adding_a_node_whose_path_is_used_by_another_node_is_rejected() { /* assert DuplicatePath */ }

#[tokio::test]
async fn adding_a_node_with_an_unknown_parent_is_rejected() { /* assert Err(Validation(DomainError::ParentNotFound(id))) */ }

#[tokio::test]
async fn adding_a_node_whose_parent_is_a_file_is_rejected() { /* assert Err(Validation(DomainError::ParentNotADirectory(id))) */ }

#[tokio::test]
async fn adding_a_node_that_would_create_a_cycle_is_rejected() { /* assert Err(Validation(DomainError::Cycle)) */ }

#[tokio::test]
async fn hiding_a_node_that_overrides_nothing_is_rejected() { /* assert Err(Validation(DomainError::NothingToHide)) */ }

#[tokio::test]
async fn hiding_a_node_on_a_template_structure_is_rejected() { /* assert Err(Validation(DomainError::HiddenNotAllowed)) */ }

#[tokio::test]
async fn updating_an_inherited_node_creates_an_override_record() { /* assert the repo holds one own record carrying the template's node_id */ }

#[tokio::test]
async fn updating_an_unknown_node_is_rejected() { /* assert Err(Repository(DomainError::NodeNotFound(id))) */ }

#[tokio::test]
async fn renaming_an_inherited_node_does_not_resurrect_the_template_original() {
    // Review Focus #1 at the usecase level
    /* assert the built tree has exactly one root and its path is the new one */
}

#[tokio::test]
async fn deleting_a_recorded_node_removes_it_and_its_descendants() {
    // Review Focus #3
    /* template `raw` > `a.txt`; the project records `raw` and `raw/b.txt`; DELETE `raw` → the repo holds no own records and the built tree is empty */
}

#[tokio::test]
async fn deleting_an_inherited_node_hides_it() { /* assert the repo holds one own record with hidden == Some(true) */ }

#[tokio::test]
async fn deleting_an_unknown_node_is_rejected() { /* assert Err(Repository(DomainError::NodeNotFound(id))) */ }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p structures --lib 2>&1 | head -30`
Expected: FAIL — `add_node` is not defined.

- [ ] **Step 3: Implement `add_node`**

`add_node(cmd)`:

1. `let eff = self.effective(cmd.structure_id).await?;`
2. If `eff.structure.level` is `Template` and `cmd.hidden == Some(true)` → `Validation(DomainError::HiddenNotAllowed)`.
3. Build the attributes and validate them: `let attrs = NodeAttributes { description: cmd.description.clone(), path: cmd.path.clone(), directory: cmd.directory, parent: cmd.parent.clone(), reference: cmd.reference.clone(), migration: cmd.migration.clone() }; attrs.validate().map_err(UsecaseError::Validation)?;`
4. **Resolve the target node by path.** Search the effective tree's flattened node list for a node whose `path` equals `cmd.path`:
   - If found **and** the structure already records it (it is in `eff.own_nodes`) → `Repository(DomainError::DuplicatePath(cmd.path))`.
   - If found **and** it is inherited → the write is an **override**: reuse that node's `node_id`.
   - If not found → mint a new id with `nanoid::nanoid!()`.
5. **Validate the parent.** If `cmd.parent` is `Some(p)`:
   - The parent must exist in the effective structure → else `Validation(DomainError::ParentNotFound(p))`.
   - The parent must be a directory → else `Validation(DomainError::ParentNotADirectory(p))`.
   - The parent must not be the node itself or one of its descendants → else `Validation(DomainError::Cycle)`. (For a new node there are no descendants; the check matters for `update_node`.)
6. **Validate the path against the rest of the effective structure.** No *other* node may already use `cmd.path` → else `Repository(DomainError::DuplicatePath(cmd.path))`. (Step 4 already handled the "this structure records it" case; this covers a path held by a *different* template node.)
7. **Validate `hidden`.** If `cmd.hidden == Some(true)` and the write did not resolve to an inherited node → `Validation(DomainError::NothingToHide)`. (A hidden node that overrides nothing would be invisible.)
8. Build the `Node` with `Node::new(node_id, attrs)?` — except that `Node::new` sets `hidden: None`, so set `node.hidden = cmd.hidden` afterwards.
9. `self.nodes.insert(cmd.structure_id, &node).await?`
10. Return the merged view of that node: re-run `effective`, find the node with `node.node_id` in the merged tree, and convert it to a `NodeView`. If it is not in the merged tree (it was hidden), return a `NodeView` built directly from the node with an empty `children` list.

- [ ] **Step 4: Implement `update_node`**

`update_node(cmd)`:

1. `let eff = self.effective(cmd.structure_id).await?;`
2. Locate the target in the effective tree by `cmd.node_id` → else `Repository(DomainError::NodeNotFound(cmd.node_id))`.
3. If the target is an own record → update that row. If it is inherited → create an override record carrying the target's `node_id`.
   - In both cases the stored `parent` is the one in the command.
4. The same validation rules as `add_node` apply (attributes, parent existence and directory-ness, cycle, path uniqueness, `hidden` rules) — except that the path-uniqueness check **excludes the node being updated**.
5. `hidden` on a template-level structure → `Validation(DomainError::HiddenNotAllowed)`.
6. `hidden == Some(true)` on a node that is **not** inherited and **not** already recorded as hidden → `Validation(DomainError::NothingToHide)`. (A node this structure recorded *can* be hidden — that is how a recorded node is taken out of the effective structure.)
7. Write the row, then return the merged view of the node.

- [ ] **Step 5: Implement `delete_node`**

`delete_node(cmd)`:

1. `let eff = self.effective(cmd.structure_id).await?;`
2. Locate the target by `cmd.node_id` → else `Repository(DomainError::NodeNotFound(cmd.node_id))`.
3. If the structure **records** it → delete that row **and every own record beneath it**. Compute the descendant set from `eff.own_nodes`: repeatedly take the own records whose `parent` is a node already in the removal set. Delete each with `self.nodes.delete(cmd.structure_id, node_id)`.
4. If it is **inherited** → write an override record that hides it: the new row carries the template node's `node_id`, `path`, `description`, `directory`, `parent`, `reference` and `migration`, with `hidden = Some(true)`. If the structure already records that node, update that row instead of inserting a second one.
5. Return `Ok(())`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p structures --lib`
Expected: PASS — 40 tests.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/structures
git commit -m "feat(structures): add the node write rules

POST places a node at a path — the usecase resolves the path against
the effective structure and decides whether it creates a node or
overrides an inherited one. PUT addresses by node_id and creates the
override record when the node was inherited. DELETE removes a recorded
row and its descendants, and hides an inherited node.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: The Postgres persistence adapters

**Files:**
- Create: `lib/crates/structures/migrations/0001_create_structures.sql`
- Create: `lib/crates/structures/migrations/0002_create_structure_nodes.sql`
- Create: `lib/crates/structures/src/adapter.rs`
- Create: `lib/crates/structures/src/adapter/persistence.rs`
- Create: `lib/crates/structures/src/adapter/persistence/postgres.rs`
- Create: `lib/crates/structures/src/adapter/persistence/postgres/structure_repo.rs`
- Create: `lib/crates/structures/src/adapter/persistence/postgres/node_repo.rs`
- Create: `lib/crates/structures/src/adapter/service.rs`
- Create: `lib/crates/structures/src/adapter/service/project.rs`
- Create: `lib/crates/structures/src/adapter/service/project/project_lookup_impl.rs`
- Modify: `lib/crates/structures/src/lib.rs`

**Interfaces:**
- Consumes: `StructureRepository`, `NodeRepository`, `ProjectLookup`, `Structure`, `StructureNew`, `StructureFilter`, `Node`, `DomainError`.
- Produces:
  - `structures::StructureRepoPg` — `pub fn new(pool: PgPool) -> Self`, implementing `StructureRepository`.
  - `structures::NodeRepoPg` — `pub fn new(pool: PgPool) -> Self`, implementing `NodeRepository`.
  - `structures::ProjectLookupImpl` — `pub fn new(project: Arc<dyn apis::project::ProjectService>) -> Self`, implementing `ProjectLookup`.

- [ ] **Step 1: Write the migrations**

`migrations/0001_create_structures.sql`:

```sql
-- structures: one row per directory layout.
-- `project_code` is a soft reference to `projects.code` from the
-- `project` crate; it is validated at the application level, not by a
-- database foreign key, so the two crates' schemas stay independent.
-- `template_id` references another row in this table and IS a foreign
-- key: a template cannot be deleted while a project structure points
-- at it.

CREATE TABLE IF NOT EXISTS structures (
    id           BIGSERIAL PRIMARY KEY,
    name         TEXT        NOT NULL,
    level        TEXT        NOT NULL,
    project_code TEXT,
    template_id  BIGINT      REFERENCES structures(id) ON DELETE RESTRICT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT structures_level_check CHECK (level IN ('template', 'project')),
    CONSTRAINT structures_level_shape CHECK (
        (level = 'template' AND project_code IS NULL AND template_id IS NULL) OR
        (level = 'project'  AND project_code IS NOT NULL AND template_id IS NOT NULL)
    )
);

-- Exactly one project-level structure per project.
CREATE UNIQUE INDEX IF NOT EXISTS structures_one_per_project
    ON structures (project_code) WHERE level = 'project';

CREATE OR REPLACE FUNCTION structures_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER structures_updated_at
BEFORE UPDATE ON structures
FOR EACH ROW EXECUTE FUNCTION structures_set_updated_at();
```

`migrations/0002_create_structure_nodes.sql`:

```sql
-- structure_nodes: one row per node of a structure.
-- At project level these rows are the *extension records* — the
-- difference from the template, not the whole tree.
--
-- `parent_id` is a SOFT reference: it may name a node that lives in the
-- template structure, so it cannot be a database foreign key.

CREATE TABLE IF NOT EXISTS structure_nodes (
    id                  BIGSERIAL PRIMARY KEY,
    structure_id        BIGINT      NOT NULL REFERENCES structures(id) ON DELETE CASCADE,
    node_id             TEXT        NOT NULL,
    description         TEXT        NOT NULL DEFAULT '',
    path                TEXT        NOT NULL,
    directory           BOOLEAN     NOT NULL DEFAULT TRUE,
    parent_id           TEXT,
    hidden              BOOLEAN     NOT NULL DEFAULT FALSE,
    reference_path      TEXT,
    migration_directory TEXT,
    migration_filter    JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT structure_nodes_unique_node UNIQUE (structure_id, node_id),
    CONSTRAINT structure_nodes_unique_path UNIQUE (structure_id, path),
    CONSTRAINT structure_nodes_migration_filter_array CHECK (jsonb_typeof(migration_filter) = 'array'),
    CONSTRAINT structure_nodes_reference_only_on_file CHECK (reference_path IS NULL OR directory = FALSE),
    CONSTRAINT structure_nodes_migration_only_on_directory CHECK (migration_directory IS NULL OR directory = TRUE)
);

CREATE INDEX IF NOT EXISTS structure_nodes_by_parent ON structure_nodes (structure_id, parent_id);

CREATE OR REPLACE FUNCTION structure_nodes_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER structure_nodes_updated_at
BEFORE UPDATE ON structure_nodes
FOR EACH ROW EXECUTE FUNCTION structure_nodes_set_updated_at();
```

- [ ] **Step 2: Write the schema regression tests**

Each repo file carries a `#[cfg(test)] mod tests` that reads its migration as a string and asserts the schema cannot regress silently:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_structures_table_keeps_its_constraints() {
        let sql = include_str!("../../../../migrations/0001_create_structures.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS structures"));
        assert!(sql.contains("CONSTRAINT structures_level_check CHECK (level IN ('template', 'project'))"));
        assert!(sql.contains("CONSTRAINT structures_level_shape CHECK"));
        assert!(sql.contains("CREATE UNIQUE INDEX IF NOT EXISTS structures_one_per_project"));
        assert!(sql.contains("BEFORE UPDATE ON structures"));
    }
}
```

The node repo's test asserts `structure_nodes_unique_node`, `structure_nodes_unique_path`, `structure_nodes_migration_filter_array`, `structure_nodes_reference_only_on_file`, `structure_nodes_migration_only_on_directory`, `ON DELETE CASCADE` and `BEFORE UPDATE ON structure_nodes`.

- [ ] **Step 3: Implement the structure repo**

`src/adapter/persistence/postgres/structure_repo.rs`:

```rust
//! PostgreSQL implementation of [`StructureRepository`].
//!
//! SQLx **runtime API** (`sqlx::query_as` / `sqlx::query`) is used
//! throughout: the workspace ships no `.sqlx/` offline cache and no
//! `DATABASE_URL` at build time, so the compile-time-checked macros are
//! intentionally avoided.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, Structure, StructureFilter, StructureLevel, StructureNew, StructureRepository,
};
```

A private `row` struct annotated `#[derive(sqlx::FromRow)]` holding the table's columns, plus a `TryFrom<row> for Structure` that maps `level` through `StructureLevel::try_from` (an unknown level → `DomainError::Repository(msg)`).

The trait impl, in the SQLx runtime style:

- `create` — `INSERT INTO structures (name, level, project_code, template_id) VALUES ($1,$2,$3,$4) RETURNING …`
- `find_by_id` — `SELECT … FROM structures WHERE id = $1`, `fetch_optional` → `Ok(None)` → `DomainError::StructureNotFound(id)`.
- `list` — build the WHERE clause with a `QueryBuilder` from the filter: `level = $n` when `filter.level` is set, `project_code = $n` when `filter.project_code` is set. Order by `id ASC`.
- `rename` — `UPDATE structures SET name = $2 WHERE id = $1 RETURNING …`; zero rows → `DomainError::StructureNotFound(id)`.
- `delete` — `DELETE FROM structures WHERE id = $1`; zero rows → `DomainError::StructureNotFound(id)`.
- `count_project_structures` — `SELECT COUNT(*) FROM structures WHERE level = 'project' AND project_code = $1`.
- `is_template_referenced` — `SELECT EXISTS(SELECT 1 FROM structures WHERE template_id = $1)`.

A private `map_error(err: sqlx::Error) -> DomainError` is the only place that knows about `sqlx::Error`:
- `sqlx::Error::RowNotFound` → `DomainError::NotFound`.
- `sqlx::Error::Database(db)` with SQLSTATE `23505` → `DomainError::DuplicateProjectStructure(String::new())` — the constraint name is not surfaced, so the crate passes the empty string and the doc comment says so. (The usecase's own `count_project_structures` check produces the useful message; the DB constraint is the race backstop.)
- `sqlx::Error::Database(db)` with SQLSTATE `23503` → `DomainError::TemplateInUse(0)` — again the id is not surfaced by the driver; the usecase checks first.
- anything else → `DomainError::Repository(msg)`.

- [ ] **Step 4: Implement the node repo**

`src/adapter/persistence/postgres/node_repo.rs`:

- `insert` — `INSERT INTO structure_nodes (structure_id, node_id, description, path, directory, parent_id, hidden, reference_path, migration_directory, migration_filter) VALUES ($1,…,$10) RETURNING …`. The `migration_filter` is bound as `serde_json::Value` (`json!(filter)`).
- `update` — `UPDATE structure_nodes SET description = $3, path = $4, directory = $5, parent_id = $6, hidden = $7, reference_path = $8, migration_directory = $9, migration_filter = $10 WHERE structure_id = $1 AND node_id = $2 RETURNING …`; zero rows → `DomainError::NodeNotFound(node_id)`.
- `delete` — `DELETE FROM structure_nodes WHERE structure_id = $1 AND node_id = $2`; zero rows → `DomainError::NodeNotFound(node_id)`.
- `find_by_id` — `SELECT … WHERE structure_id = $1 AND node_id = $2` → `Ok(None)` when absent.
- `list` — `SELECT … WHERE structure_id = $1 ORDER BY path ASC`.
- `find_by_path` — `SELECT … WHERE structure_id = $1 AND path = $2` → `Ok(None)` when absent.

The same `map_error` shape; SQLSTATE `23505` → `DomainError::DuplicatePath(String::new())`.

- [ ] **Step 5: Implement the project lookup adapter**

`src/adapter/service/project/project_lookup_impl.rs` — copy `sdtm-project`'s `ProjectLookupImpl` and change the error mapping to this crate's `DomainError`:

```rust
/// Adapts `apis::project::ProjectService` to the domain's
/// `ProjectLookup` port.
pub struct ProjectLookupImpl {
    project: Arc<dyn apis::project::ProjectService>,
}

impl ProjectLookupImpl {
    pub fn new(project: Arc<dyn apis::project::ProjectService>) -> Self {
        Self { project }
    }
}

#[async_trait]
impl ProjectLookup for ProjectLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        self.project.get_project_by_code(code).await.map(|_| ()).map_err(|e| match e {
            apis::project::ProjectApiError::NotFound => DomainError::ProjectNotFound(code.to_string()),
            other => DomainError::Repository(other.to_string()),
        })
    }
}
```

Check `apis::project::ProjectService`'s method name and error variants against `lib/crates/apis/src/project.rs` before writing this — the method may be `get_project_by_code` or `get_by_code`, and the not-found variant may be `NotFound` or `ProjectNotFound`. Match whatever that file declares.

- [ ] **Step 6: Wire the adapter modules**

`src/adapter.rs`:

```rust
//! Adapter layer.
//!
//! Hosts three sub-modules:
//! - `persistence` — SQLx/PostgreSQL implementations of the repository
//!   ports.
//! - `service` — adapters that bridge another workspace crate's port
//!   into a domain port.
//! - `facade` — the implementation of the API trait defined in
//!   `apis::structures::StructureService`.

pub mod facade;
pub mod persistence;
pub mod service;
```

`src/adapter/persistence.rs` declares `pub mod postgres;`.
`src/adapter/persistence/postgres.rs` declares `pub mod node_repo;` and `pub mod structure_repo;`.
`src/adapter/service.rs` declares `pub mod project;`.
`src/adapter/service/project.rs` declares `pub mod project_lookup_impl;`.

Create a stub `src/adapter/facade.rs` with `pub mod in_memory;` and a stub `src/adapter/facade/in_memory.rs` — Task 7 fills them in. (An empty `in_memory.rs` with just a doc comment is enough to compile.)

`src/lib.rs` gains `pub mod adapter;` and re-exports `NodeRepoPg`, `StructureRepoPg`, `ProjectLookupImpl`.

- [ ] **Step 7: Verify it compiles**

Run: `cargo build -p structures 2>&1 | tail -30`
Expected: compiles.

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p structures --lib`
Expected: PASS — 42 tests (the two new schema tests included).

- [ ] **Step 9: Commit**

```bash
git add lib/crates/structures
git commit -m "feat(structures): add the Postgres persistence adapters

The structures and structure_nodes tables with their migrations, the two
SQLx repositories, and the project-existence lookup adapter.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: The `apis` port and the facade

**Files:**
- Create: `lib/crates/apis/src/structures.rs`
- Modify: `lib/crates/apis/src/lib.rs` — add `pub mod structures;`
- Create: `lib/crates/structures/src/adapter/facade/in_memory/service.rs`
- Create: `lib/crates/structures/src/adapter/facade/in_memory/tests.rs`
- Modify: `lib/crates/structures/src/adapter/facade/in_memory.rs`
- Modify: `lib/crates/structures/src/lib.rs`

**Interfaces:**
- Consumes: the usecase from Tasks 4–5; `apis::project::ProjectService` for the lookup.
- Produces:
  - `apis::structures::StructureService` — the port trait.
  - `apis::structures::{StructureLevel, MigrationView, NodeView, StructureView, StructureSummaryView, StructureTreeView, StructureList}` — the apis view DTOs.
  - `apis::structures::{CreateStructureRequest, RenameStructureRequest, AddNodeRequest, UpdateNodeRequest, DeleteNodeRequest, ListStructuresRequest}` — the apis request DTOs.
  - `apis::structures::StructureApiError` — the error enum.
  - `structures::StructureServiceImpl` — the facade.

- [ ] **Step 1: Write the `apis` port**

`lib/crates/apis/src/structures.rs`:

```rust
//! Outbound port for the directory-structure service.
//!
//! Mirrors the surface of `structures::usecase::StructureUsecase` so
//! adapters in any backend (in-memory, PostgreSQL, …) can adapt their
//! own types to the shared contract defined here.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum StructureApiError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("structure not found: {0}")]
    StructureNotFound(i64),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("template structure not found: {0}")]
    TemplateNotFound(i64),
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("path already used: {0}")]
    DuplicatePath(String),
    #[error("project already has a structure: {0}")]
    DuplicateProjectStructure(String),
    #[error("template structure is still in use: {0}")]
    TemplateInUse(i64),
    #[error("repository error: {0}")]
    Repository(String),
}
```

`StructureLevel` is re-declared here (the `apis` crate depends on no other workspace crate):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureLevel {
    Template,
    Project,
}
```

`MigrationView`, `NodeView`, `StructureView`, `StructureSummaryView`, `StructureTreeView` and `StructureList` mirror the usecase views field for field. `StructureList` is `struct { pub structures: Vec<StructureSummaryView> }`.

The request DTOs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStructureRequest {
    pub name: String,
    pub level: StructureLevel,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameStructureRequest {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddNodeRequest {
    pub structure_id: i64,
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeRequest {
    pub structure_id: i64,
    pub node_id: String,
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationView>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteNodeRequest {
    pub structure_id: i64,
    pub node_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListStructuresRequest {
    pub level: Option<StructureLevel>,
    pub project_code: Option<String>,
}
```

The trait:

```rust
#[async_trait]
pub trait StructureService: Send + Sync {
    async fn create_structure(&self, req: CreateStructureRequest) -> Result<StructureView, StructureApiError>;
    async fn get_structure(&self, id: i64) -> Result<StructureView, StructureApiError>;
    async fn list_structures(&self, req: ListStructuresRequest) -> Result<StructureList, StructureApiError>;
    async fn rename_structure(&self, req: RenameStructureRequest) -> Result<StructureView, StructureApiError>;
    async fn delete_structure(&self, id: i64) -> Result<(), StructureApiError>;

    async fn add_node(&self, req: AddNodeRequest) -> Result<NodeView, StructureApiError>;
    async fn update_node(&self, req: UpdateNodeRequest) -> Result<NodeView, StructureApiError>;
    async fn delete_node(&self, req: DeleteNodeRequest) -> Result<(), StructureApiError>;

    async fn build_structure(&self, id: i64) -> Result<StructureTreeView, StructureApiError>;
}
```

Register it in `lib/crates/apis/src/lib.rs`:

```rust
pub mod structures;
```

- [ ] **Step 2: Write the failing facade tests**

`src/adapter/facade/in_memory/tests.rs` — reuse the fakes from `crate::usecase::tests` if they are visible; otherwise the facade test file declares its own minimal fakes. The facade test module lives in the crate, so `pub(crate)` fakes in `usecase/tests.rs` are reachable only if that module is compiled for the test — declare the fakes `pub(crate)` in `usecase/tests.rs` and re-export them from a `#[cfg(test)] pub(crate) mod fakes;` if needed. Simplest: give the facade tests their own small fakes.

Tests:

```rust
#[tokio::test]
async fn the_facade_round_trips_a_template_structure() { /* create then get */ }

#[tokio::test]
async fn the_facade_maps_a_duplicate_path_to_the_duplicate_path_error() {
    /* assert Err(StructureApiError::DuplicatePath(path)) */
}

#[tokio::test]
async fn the_facade_maps_a_missing_node_to_the_node_not_found_error() {
    /* assert Err(StructureApiError::NodeNotFound(id)) */
}

#[tokio::test]
async fn the_facade_maps_a_validation_failure_to_the_validation_error() {
    /* assert Err(StructureApiError::Validation(_)) */
}
```

- [ ] **Step 3: Implement the facade**

`src/adapter/facade/in_memory/service.rs`:

```rust
pub struct StructureServiceImpl<S, N, P>
where
    S: StructureRepository,
    N: NodeRepository,
    P: ProjectLookup,
{
    usecase: StructureUsecase<S, N, P>,
}

impl<S, N, P> StructureServiceImpl<S, N, P> where … {
    pub fn new(usecase: StructureUsecase<S, N, P>) -> Self { Self { usecase } }
}
```

Each trait method translates the apis DTO into the usecase command, calls the usecase, and translates the usecase view into the apis view. The `StructureLevel` translation is a matching pair of `match`es between the apis copy and the crate's own copy.

The error translation:

```rust
impl From<UsecaseError> for StructureApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => StructureApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => StructureApiError::NotFound,
                DomainError::StructureNotFound(id) => StructureApiError::StructureNotFound(id),
                DomainError::NodeNotFound(node_id) => StructureApiError::NodeNotFound(node_id),
                DomainError::TemplateNotFound(id) => StructureApiError::TemplateNotFound(id),
                DomainError::ProjectNotFound(code) => StructureApiError::ProjectNotFound(code),
                DomainError::DuplicatePath(path) => StructureApiError::DuplicatePath(path),
                DomainError::DuplicateProjectStructure(code) => {
                    StructureApiError::DuplicateProjectStructure(code)
                }
                DomainError::TemplateInUse(id) => StructureApiError::TemplateInUse(id),
                other => StructureApiError::Repository(other.to_string()),
            },
        }
    }
}
```

Every other `DomainError` variant falls into the `other` arm and becomes a `Repository` error — a 500, never a panic.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p structures --lib`
Expected: PASS — 46 tests.

- [ ] **Step 5: Add the compile-only public API test**

Create `lib/crates/structures/tests/public_api.rs`:

```rust
//! Compile-only guard over the crate's documented public surface.
//!
//! Names every consumer import, pins the constructor chain as function
//! pointers, and asserts the trait bounds the usecase relies on.

use std::sync::Arc;

use structures::{
    CreateStructure, DomainError, MergedNode, MergedTree, Migration, Node, NodeAttributes,
    NodeRepository, NodeView, ProjectLookup, Structure, StructureFilter, StructureLevel,
    StructureNew, StructureRepository, StructureServiceImpl, StructureSummaryView,
    StructureTreeView, StructureUsecase, StructureUsecaseConfig, StructureView, UsecaseError,
};

#[test]
fn the_documented_imports_resolve() {
    // every name above must exist; this test exists to fail at compile
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
    let _: Option<StructureView> = None;
    let _: Option<StructureSummaryView> = None;
    let _: Option<StructureTreeView> = None;
    let _: Option<CreateStructure> = None;
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
fn the_facade_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<StructureServiceImpl<InMemoryStructureRepo, InMemoryNodeRepo, InMemoryProjectLookup>>();
}
```

The last test needs concrete in-memory types, and the crate exposes none publicly. Define the three fakes **in the test file itself** — a `struct FakeStructures`, `struct FakeNodes` and `struct FakeProjects`, each implementing its port with `unimplemented!()` bodies — and assert the bounds against them:

```rust
#[derive(Default)]
struct FakeStructures;

#[async_trait]
impl StructureRepository for FakeStructures {
    async fn create(&self, _input: StructureNew) -> Result<Structure, DomainError> { unimplemented!() }
    async fn find_by_id(&self, _id: i64) -> Result<Structure, DomainError> { unimplemented!() }
    async fn list(&self, _filter: StructureFilter) -> Result<Vec<Structure>, DomainError> { unimplemented!() }
    async fn rename(&self, _id: i64, _name: &str) -> Result<Structure, DomainError> { unimplemented!() }
    async fn delete(&self, _id: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn count_project_structures(&self, _project_code: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn is_template_referenced(&self, _template_id: i64) -> Result<bool, DomainError> { unimplemented!() }
}

#[derive(Default)]
struct FakeNodes;

#[async_trait]
impl NodeRepository for FakeNodes {
    async fn insert(&self, _structure_id: i64, _node: &Node) -> Result<Node, DomainError> { unimplemented!() }
    async fn update(&self, _structure_id: i64, _node: &Node) -> Result<Node, DomainError> { unimplemented!() }
    async fn delete(&self, _structure_id: i64, _node_id: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn find_by_id(&self, _structure_id: i64, _node_id: &str) -> Result<Option<Node>, DomainError> { unimplemented!() }
    async fn list(&self, _structure_id: i64) -> Result<Vec<Node>, DomainError> { unimplemented!() }
    async fn find_by_path(&self, _structure_id: i64, _path: &str) -> Result<Option<Node>, DomainError> { unimplemented!() }
}

#[derive(Default)]
struct FakeProjects;

#[async_trait]
impl ProjectLookup for FakeProjects {
    async fn get_by_code(&self, _code: &str) -> Result<(), DomainError> { unimplemented!() }
}

#[test]
fn the_usecase_accepts_any_port_implementations() {
    fn assert_usecase<S: StructureRepository, N: NodeRepository, P: ProjectLookup>() {}
    assert_usecase::<FakeStructures, FakeNodes, FakeProjects>();

    fn assert_facade<S: StructureRepository, N: NodeRepository, P: ProjectLookup>() {}
    assert_facade::<FakeStructures, FakeNodes, FakeProjects>();
}
```

`async-trait` must be a **dev-dependency** of the crate for this file, or the test must use the `async_trait` attribute from the regular dependency — the crate already depends on `async-trait`, so `use async_trait::async_trait;` works in an integration test only if the dependency is visible to it. Integration tests link against the crate's *public* dependencies only through the crate itself, so add `async-trait = { workspace = true }` to `[dev-dependencies]` if the compiler rejects it.

- [ ] **Step 6: Verify the apis crate**

Run: `cargo clippy -p apis --all-targets --all-features -- -D warnings && cargo test -p apis`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/apis lib/crates/structures
git commit -m "feat(structures): add the apis port and the facade

The new apis::structures::StructureService port with its DTOs and error
type, and the StructureServiceImpl facade that adapts the usecase to
it.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: The server HTTP surface

**Files:**
- Create: `apps/server/aegis-server/src/transport/http/structures.rs`? — no: create the directory form:
  - Create: `apps/server/aegis-server/src/transport/http/structures/router.rs`
  - Create: `apps/server/aegis-server/src/transport/http/structures/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http.rs` — add `pub mod structures;`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs` — add the wire DTOs
- Modify: `apps/server/aegis-server/src/transport/http/error.rs` — add the `Structures` variant and the dispatch arms
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` — nest the new router
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs` — register the schemas
- Modify: `apps/server/aegis-server/src/state.rs` — add the `structures` field and the null double
- Modify: `apps/server/aegis-server/src/run.rs` — build the service
- Modify: `apps/server/aegis-server/Cargo.toml` — add the `structures` dependency

**Interfaces:**
- Consumes: `apis::structures::StructureService` and its DTOs from Task 7.
- Produces: nine routes under `/api/structures`.

- [ ] **Step 1: Add the wire DTOs**

In `dto.rs`, add a `// ---- structures ----` section:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum StructureLevelResponse {
    Template,
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationViewResponse {
    pub directory: String,
    pub filter: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeViewResponse {
    pub node_id: String,
    pub description: String,
    pub path: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
    pub children: Vec<NodeViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureViewResponse {
    pub id: i64,
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
    pub nodes: Vec<NodeViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureSummaryResponse {
    pub id: i64,
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureListResponse {
    pub structures: Vec<StructureSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureTreeResponse {
    pub structure_id: i64,
    pub nodes: Vec<NodeViewResponse>,
}
```

Plus the request DTOs `CreateStructureRequest`, `RenameStructureRequest`, `AddNodeRequest`, `UpdateNodeRequest`, and the query DTO:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListStructuresQuery {
    pub level: Option<StructureLevelResponse>,
    pub project_code: Option<String>,
}
```

Add `From<apis::structures::…>` impls for each response type, mirroring the `sdtm-project` block in the same file.

- [ ] **Step 2: Add the error mapping**

In `error.rs`, add the variant:

```rust
#[error("{0}")]
Structures(#[from] apis::structures::StructureApiError),
```

Add the `status()` arm `Self::Structures(e) => structures_status(e),` and the `code()` arm `Self::Structures(e) => structures_code(e),` and the two helper functions:

```rust
fn structures_status(e: &apis::structures::StructureApiError) -> StatusCode {
    use apis::structures::StructureApiError;
    match e {
        StructureApiError::Validation(_) => StatusCode::BAD_REQUEST,
        StructureApiError::NotFound
        | StructureApiError::StructureNotFound(_)
        | StructureApiError::NodeNotFound(_)
        | StructureApiError::TemplateNotFound(_)
        | StructureApiError::ProjectNotFound(_) => StatusCode::NOT_FOUND,
        StructureApiError::DuplicatePath(_)
        | StructureApiError::DuplicateProjectStructure(_)
        | StructureApiError::TemplateInUse(_) => StatusCode::CONFLICT,
        StructureApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn structures_code(e: &apis::structures::StructureApiError) -> &'static str {
    use apis::structures::StructureApiError;
    match e {
        StructureApiError::Validation(_) => "validation_failed",
        StructureApiError::NotFound => "not_found",
        StructureApiError::StructureNotFound(_) => "structure_not_found",
        StructureApiError::NodeNotFound(_) => "node_not_found",
        StructureApiError::TemplateNotFound(_) => "template_not_found",
        StructureApiError::ProjectNotFound(_) => "project_not_found",
        StructureApiError::DuplicatePath(_) => "duplicate_path",
        StructureApiError::DuplicateProjectStructure(_) => "duplicate_project_structure",
        StructureApiError::TemplateInUse(_) => "template_in_use",
        StructureApiError::Repository(_) => "repository_error",
    }
}
```

- [ ] **Step 3: Write the handlers**

`transport/http/structures/handlers.rs` — nine handlers, each a thin adapter in the shape of the `sdtm_project` handlers. Each takes `State(state): State<AppState>`, `_claims: AuthClaims`, and the path/query/body extractors, calls `state.structures.…`, and returns `Result<Json<T>, ApiError>`.

**The auth gate.** The gate depends on the *target structure's level*, not on the route, so it is resolved per request:

```rust
/// Reject a write against a template structure unless the caller is an
/// admin or root. Template structures are global and affect every
/// project, so they are gated like the `domain-model` module's global
/// data. Project-level structures are open to any authenticated user.
fn require_template_write_permission(state: &AppState, structure_id: i64) -> Result<(), ApiError> {
    let structure = state.structures.get_structure(structure_id).await?;
    if structure.level == apis::structures::StructureLevel::Template {
        // the caller must be admin/root
    }
    Ok(())
}
```

The gate needs the caller's role, so the handler must keep the `AuthClaims` rather than binding it to `_claims`. Use the existing helper:

```rust
use crate::transport::http::auth::middleware::{AuthClaims, require_admin_or_root};

pub async fn add_node(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(path): Path<StructureNodePath>,
    Json(req): Json<dto::AddNodeRequest>,
) -> Result<(StatusCode, Json<dto::NodeViewResponse>), ApiError> {
    let structure = state.structures.get_structure(req.structure_id).await?;
    if structure.level == apis::structures::StructureLevel::Template {
        require_admin_or_root(&claims)?;
    }
    …
}
```

`require_admin_or_root` is `pub(crate)` in `crate::transport::http::auth::middleware` and returns `Result<(), ApiError>`.

The `StructureNodePath` extractor — `{structure_id}` and `{node_id}`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureNodePath {
    pub structure_id: i64,
    pub node_id: String,
}
```

Put it in `dto.rs` next to the other structures wire types.

The nine handlers and their paths:

| Handler | Method + path | `#[utoipa::path]` `path` value | `operation_id` |
| --- | --- | --- | --- |
| `create_structure` | POST `/api/structures` | `/` | `structures_create_structure` |
| `list_structures` | GET `/api/structures` | `/` | `structures_list_structures` |
| `get_structure` | GET `/api/structures/{id}` | `/{id}` | `structures_get_structure` |
| `rename_structure` | PUT `/api/structures/{id}` | `/{id}` | `structures_rename_structure` |
| `delete_structure` | DELETE `/api/structures/{id}` | `/{id}` | `structures_delete_structure` |
| `build_structure` | GET `/api/structures/{id}/tree` | `/{id}/tree` | `structures_build_structure` |
| `add_node` | POST `/api/structures/{id}/nodes` | `/{id}/nodes` | `structures_add_node` |
| `update_node` | PUT `/api/structures/{id}/nodes/{node_id}` | `/{id}/nodes/{node_id}` | `structures_update_node` |
| `delete_node` | DELETE `/api/structures/{id}/nodes/{node_id}` | `/{id}/nodes/{node_id}` | `structures_delete_node` |

`create_structure` has no target structure yet, so it gates on the **requested level**: `req.level == Template` → `require_admin_or_root(&claims)?`.

Every handler's `#[utoipa::path]` block declares `security(("BearerAuth" = []))` and lists its responses: `400`, `401`, `404`, `409`, `500` as applicable. The template-gated handlers also advertise `403`.

- [ ] **Step 4: Write the router**

`transport/http/structures/router.rs` — one `routes!` call per verb, following the `sdtm_project` router:

```rust
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::list_structures))
        .routes(routes!(handlers::create_structure))
        .routes(routes!(handlers::get_structure))
        .routes(routes!(handlers::build_structure))
        .routes(routes!(handlers::rename_structure))
        .routes(routes!(handlers::delete_structure))
        .routes(routes!(handlers::add_node))
        .routes(routes!(handlers::update_node))
        .routes(routes!(handlers::delete_node))
}
```

(The `utoipa-axum` 0.2 panic on two handlers sharing a verb in one `routes!` call is why the calls are split. `list_structures` is a GET and `create_structure` is a POST, so they are in separate calls; `get_structure` and `build_structure` are both GET and therefore also need separate calls — split them further if the compiler or the runtime complains.)

Nest it in `transport/http/router.rs`:

```rust
use crate::transport::http::structures::router as structures_router;
…
.nest("/structures", structures_router::router())
```

- [ ] **Step 5: Wire the state and the bootstrap**

`state.rs` — add the field:

```rust
pub structures: Arc<dyn apis::structures::StructureService>,
```

and a null double in `test_support`, mirroring `NullSdtmProjectService`:

```rust
/// A `StructureService` for tests that need an `AppState` but never
/// touch the structures endpoints. Every method is `unimplemented!()`.
#[derive(Clone)]
pub(crate) struct NullStructureService;
```

with the nine methods, each `unimplemented!()`.

`run.rs` — add the builder:

```rust
fn build_structure_service(
    pool: PgPool,
    project: Arc<dyn apis::project::ProjectService>,
) -> Arc<dyn apis::structures::StructureService> {
    let structures = StructureRepoPg::new(pool.clone());
    let nodes = NodeRepoPg::new(pool);
    let projects = Arc::new(ProjectLookupImpl::new(project));
    let usecase = StructureUsecase::new(StructureUsecaseConfig {
        structures,
        nodes,
        projects,
    });
    Arc::new(StructureServiceImpl::new(usecase))
}
```

and call it in `run()`:

```rust
let structures = build_structure_service(pool.clone(), project.clone());
```

then add `structures,` to the `AppState { … }` literal.

`Cargo.toml` — add:

```toml
# structures provides the Postgres repositories for the structures and
# structure_nodes tables, the StructureUsecase, and the
# apis::structures::StructureService adapter that the HTTP layer
# consumes through AppState.
structures = { path = "../../../lib/crates/structures" }
```

- [ ] **Step 6: Register the OpenAPI schemas**

In `openapi.rs`, add every new response and request type to the `components(schemas(…))` list:

```rust
dto::StructureLevelResponse,
dto::MigrationViewResponse,
dto::NodeViewResponse,
dto::StructureViewResponse,
dto::StructureSummaryResponse,
dto::StructureListResponse,
dto::StructureTreeResponse,
dto::CreateStructureRequest,
dto::RenameStructureRequest,
dto::AddNodeRequest,
dto::UpdateNodeRequest,
```

- [ ] **Step 7: Verify it compiles and the existing tests still pass**

Run: `cargo build -p aegis-server 2>&1 | tail -30 && cargo test -p aegis-server 2>&1 | tail -30`
Expected: compiles; the existing server tests pass.

- [ ] **Step 8: Add the router OpenAPI test**

The server's `transport/http/router.rs` has a test module that asserts every route requires `BearerAuth`. Extend it to cover the nine new routes, and assert that the template-gated ones advertise a `403` while the project-scoped ones do not. Follow the shape of the existing assertions in that file.

- [ ] **Step 9: Commit**

```bash
git add apps/server/aegis-server
git commit -m "feat(aegis-server): add the structures HTTP endpoints

Nine routes under /api/structures covering structure CRUD, node
mutations and the built tree. Writes against a template structure
require admin/root; project-level writes are open to any authenticated
user.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: The README, the live-DB test, and the full verification gate

**Files:**
- Create: `lib/crates/structures/README.md`
- Create: `lib/crates/structures/tests/integration_persistence.rs`

**Interfaces:**
- Consumes: everything from Tasks 1–8.
- Produces: the crate's README and the live-DB round-trip test.

- [ ] **Step 1: Write the live-DB test**

`lib/crates/structures/tests/integration_persistence.rs` — copy the shape of `lib/crates/sdtm-project/tests/integration_persistence.rs`: `dotenvy::dotenv()`, read `AEGIS_STRUCTURES_DATABASE_URL` (panic with a clear message when it is missing), run `sqlx::migrate!("./migrations")`, then **drop the live tables and the `_sqlx_migrations` table** so each run starts clean.

The round trips to cover:

1. Create a template structure with two nodes; read it back and assert the tree.
2. Create a project structure pointing at the template; add a node under a template directory; build it and assert the merged tree has both the template's other child and the new one.
3. Override a template node and assert the template's children under it are gone.
4. Hide a template node and assert it is absent from the built tree.
5. Delete a project structure and assert its node rows went with it (`ON DELETE CASCADE`).
6. Assert that creating a second project structure for the same project fails with `DuplicateProjectStructure`.

Give every `UNIQUE`-constrained column a per-run unique value (an atomic counter plus the wall-clock nanoseconds) so concurrent runs do not collide.

- [ ] **Step 2: Run the live-DB test**

Run: `cargo test -p structures -- --ignored --test-threads=1`
Expected: PASS when `AEGIS_STRUCTURES_DATABASE_URL` points at a **dedicated** database. If the variable is not set, the test panics with the clear message — that is the expected behaviour of the harness, and it is not a failure of the code.

- [ ] **Step 3: Write the README**

`lib/crates/structures/README.md` covers:

- One-sentence purpose: "Records the directory structure of a clinical project and merges a project's layout over a shared template."
- The `src/` tree, matching the real module shape.
- Database setup:

  ```bash
  sqlx migrate run --source lib/crates/structures/migrations
  ```

  and the env var `AEGIS_STRUCTURES_DATABASE_URL`, with a constructor snippet.
- How to run the ignored tests: `cargo test -p structures -- --ignored --test-threads=1`.
- A prominent note that `AEGIS_STRUCTURES_DATABASE_URL` must point at a **dedicated** database, because the live test drops `_sqlx_migrations` — a table SQLx shares across every crate that migrates that database.
- A back-link to `docs/guidelines/lib-crate-development.md`.

- [ ] **Step 4: Run the full verification gate**

Run each of these and confirm a clean result:

```bash
cargo fmt --all -- --check
cargo clippy -p structures --all-targets --all-features -- -D warnings
cargo clippy -p apis --all-targets --all-features -- -D warnings
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p structures
cargo doc -p structures --no-deps
cargo test --workspace
```

If `cargo fmt --all -- --check` reports diffs, run `cargo fmt --all` and re-verify.

- [ ] **Step 5: Confirm the desktop app is untouched**

Run: `git diff --stat main -- apps/desktop`
Expected: empty output. The desktop app must not appear anywhere in this change.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/structures
git commit -m "docs(structures): add the README and the live-DB round trip

The crate README with its database setup and the dedicated-database
warning, plus the ignore-gated live-Postgres round-trip test.

Spec: docs/superpowers/specs/2026-11-02-structures-crate-design.md

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Notes for the executor

- **Do not modify `apps/desktop/aegis-desktop`.** The spec is explicit and Task 9 Step 5 verifies it.
- **The merge is the risky part.** Task 2's tests are the specification of the tree's shape; if a later task's tree does not match them, the merge is wrong.
- **`nanoid::nanoid!()`** mints a 21-character id. It cannot fail.
- **The `apis` crate must stay dependency-free.** `lib/crates/apis/src/structures.rs` declares its own `StructureLevel`; it must not import it from the `structures` crate.
- **Check `apis::project::ProjectService`'s real method name and error variants** before writing the `ProjectLookupImpl` adapter (Task 6, Step 5) — read `lib/crates/apis/src/project.rs` rather than assuming.
- **`migration_filter` is JSONB.** Bind it as `serde_json::Value` and read it back with `serde_json::from_value`. The `CHECK (jsonb_typeof(migration_filter) = 'array')` constraint is what stops a malformed value from landing.
