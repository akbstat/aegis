# Aegis Desktop Templates Feature Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a `templates` feature in `apps/desktop/aegis-desktop` — a Templates landing page with one fixed card, and a Project Structure Template page whose right-hand sidebar lists the global template structures and whose left pane renders each structure's folder tree in a Complete view and a Flat view, with admin-only add/edit/delete on nodes.

**Architecture:** The server's `/api/structures` endpoints already exist and are unchanged. The desktop gains a new transport layer (`src-tauri/src/http/structures.rs` + `src-tauri/src/commands/structures.rs`), a hand-written TS mirror of the wire DTOs, a `features/templates` module, and two file-based routes. Both tree views render from a single `get_structure` fetch — the Flat view is a pure client-side transform of the same tree.

**Tech Stack:** Rust (Tauri 2, reqwest, serde, wiremock for tests), React 19, TypeScript, TanStack Query + TanStack Router (file-based routing), MUI 9, `@mui/x-tree-view@9.16.0`, Vitest + Testing Library.

**Spec:** `docs/superpowers/specs/2026-11-03-aegis-desktop-templates-feature-design.md` — the plan argues from the spec, so the spec travels with it; executors read both.

## Global Constraints

- **Desktop only.** No changes to `apps/server/` or `lib/crates/`. The server's `/api/structures` endpoints are the contract; nothing here may require a server change.
- **The Rust side is transport-only.** `src-tauri/src/commands/` is a 1:1 `#[tauri::command]` shim over `src-tauri/src/http/`. TypeScript never calls the server directly.
- **Wire DTOs are duplicated by hand.** The Rust structs in `src-tauri/src/http/structures.rs` and the TS interfaces in `src/shared/api/types.ts` are mirrored 1:1; TS identifiers camelCase, the wire camelCase for these structures. Both halves change in the same commit.
- **`@mui/x-tree-view` is pinned to `9.16.0`** and is a **regular dependency of `@aegis/ui`** (not a peer, not a desktop dependency) behind a new `@aegis/ui/tree-view` subpath — the same shape as the existing `@mui/x-charts` → `@aegis/ui/charts` subpath. The desktop takes on no direct `@mui/*` dependency.
- **i18n:** `lib/packages/ui/src/i18n/locales/en.ts` is the source of truth (`TranslationKey = keyof typeof en`). Every key added to `en.ts` must also be added to `zhCN.ts` or `pnpm --filter @aegis/ui typecheck` fails.
- **Query keys come from `src/shared/query/keys.ts`** — never an inline array.
- **`hidden` is never sent.** The server rejects `hidden = true` against a template structure, and hidden nodes are already absent from the tree the page renders. The drawer has no `hidden` field and the client never sends the key.
- **`parent` is always sent** on add and on edit. `AddNodeRequest` / `UpdateNodeRequest` are full replacements; `parent: null` means "at the root".
- **A node's `path` is the path relative to the structure root**, and the desktop owns computing it: adding under a parent prefills `parent.path + "/"`.

## Review Focus

Five input classes the spec implies but does not spell out. Each has a test in the owning task.

1. **The template list comes back empty** (no template structures exist yet). The page must not navigate to a null structure id and must show an empty state. → Task 11.
2. **A template structure has zero nodes.** The tree pane must render an empty state, not an empty `SimpleTreeView` that looks broken. → Task 11.
3. **A node's `description` is empty** — the domain explicitly allows it. The row must render without a blank secondary line. → Task 9.
4. **A path that is not a valid relative path** (leading `/`, trailing `/`, a `.` or `..` segment, or a `\`). The server returns `400 invalid_path`; the drawer must disable Save before the request is ever made. → Task 10.
5. **A directory is deleted.** The server removes the whole subtree with it, so the confirmation copy must say so — otherwise the admin believes only one folder disappeared. → Task 10.

---

### Task 1: `@aegis/ui/tree-view` subpath

**Files:**
- Modify: `lib/packages/ui/package.json` — `dependencies` and `exports`
- Create: `lib/packages/ui/src/tree-view/index.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: the subpath `@aegis/ui/tree-view`, re-exporting `@mui/x-tree-view` in full. Later tasks import `SimpleTreeView` and `TreeItem` from `"@aegis/ui/tree-view"`.

- [ ] **Step 1: Add the dependency and the export entry**

In `lib/packages/ui/package.json`, add to `dependencies` (alongside `"@mui/x-charts": "9.16.0"`):

```json
"@mui/x-tree-view": "9.16.0"
```

Add to the `exports` object (after the `"./charts"` entry):

```json
"./tree-view": "./src/tree-view/index.ts"
```

- [ ] **Step 2: Create the re-export module**

Create `lib/packages/ui/src/tree-view/index.ts`:

```ts
// Thin re-export of `@mui/x-tree-view@9.16.0`. Consumers compose the tree
// primitives directly; the UI package ships no tree wrapper. The dependency
// is a regular (not peer) dependency so the tree resolves the same
// `@mui/material` instance the rest of the app uses — a second instance
// would put the tree on a different theme context.
export * from '@mui/x-tree-view';
```

- [ ] **Step 3: Install the new dependency**

Run: `pnpm install`
Expected: the lockfile gains `@mui/x-tree-view@9.16.0`; `lib/packages/ui/node_modules/@mui/x-tree-view` exists.

- [ ] **Step 4: Verify the UI package still typechecks**

Run: `pnpm --filter @aegis/ui typecheck`
Expected: PASS with no errors.

- [ ] **Step 5: Commit**

```bash
git add lib/packages/ui/package.json lib/packages/ui/src/tree-view/index.ts pnpm-lock.yaml
git commit -m "feat(ui): add the @aegis/ui/tree-view subpath"
```

---

### Task 2: Rust HTTP adapter for structures

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/structures.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http.rs` — add `pub mod structures;`

**Interfaces:**
- Consumes: `super::client::HttpClient` (`request`, `request_bytes`), `super::dto::ApiError`.
- Produces (used by Task 3):
  - `pub enum StructureLevelResponse { Template, Project }` — `#[serde(rename_all = "camelCase")]`, serialises to `"template"` / `"project"`.
  - `pub struct MigrationViewResponse { pub directory: String, pub filter: Vec<String> }`
  - `pub struct NodeViewResponse { pub node_id: String, pub description: String, pub path: String, pub directory: bool, pub parent: Option<String>, pub reference: Option<String>, pub migration: Option<MigrationViewResponse>, pub children: Vec<NodeViewResponse> }`
  - `pub struct StructureViewResponse { pub id: i64, pub name: String, pub level: StructureLevelResponse, pub project_code: Option<String>, pub template_id: Option<i64>, pub nodes: Vec<NodeViewResponse> }`
  - `pub struct StructureSummaryResponse { pub id: i64, pub name: String, pub level: StructureLevelResponse, pub project_code: Option<String>, pub template_id: Option<i64> }`
  - `pub struct StructureListResponse { pub structures: Vec<StructureSummaryResponse> }`
  - `pub struct AddNodeRequest { pub path: String, pub description: String, pub directory: bool, pub parent: Option<String>, pub reference: Option<String>, pub migration: Option<MigrationViewResponse> }`
  - `pub struct UpdateNodeRequest { pub path: String, pub description: String, pub directory: bool, pub parent: Option<String>, pub reference: Option<String>, pub migration: Option<MigrationViewResponse> }`
  - `pub async fn list_structures(c: &HttpClient, level: Option<StructureLevelResponse>) -> Result<Vec<StructureSummaryResponse>, ApiError>`
  - `pub async fn get_structure(c: &HttpClient, id: i64) -> Result<StructureViewResponse, ApiError>`
  - `pub async fn add_node(c: &HttpClient, structure_id: i64, body: &AddNodeRequest) -> Result<NodeViewResponse, ApiError>`
  - `pub async fn update_node(c: &HttpClient, structure_id: i64, node_id: &str, body: &UpdateNodeRequest) -> Result<NodeViewResponse, ApiError>`
  - `pub async fn delete_node(c: &HttpClient, structure_id: i64, node_id: &str) -> Result<(), ApiError>`

- [ ] **Step 1: Write the failing serde tests**

Create `apps/desktop/aegis-desktop/src-tauri/src/http/structures.rs` with the `use` block and the tests below, but with the function bodies stubbed as `todo!()`. The `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::client::{HttpClient, MemoryStore};
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn structure_level_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&StructureLevelResponse::Template).unwrap(),
            "\"template\""
        );
        assert_eq!(
            serde_json::to_string(&StructureLevelResponse::Project).unwrap(),
            "\"project\""
        );
    }

    #[test]
    fn node_view_response_parses_the_full_wire_shape() {
        let j = r#"{
            "nodeId": "n1",
            "description": "raw data",
            "path": "raw/data",
            "directory": true,
            "parent": "n0",
            "reference": null,
            "migration": { "directory": "/opt/sop/raw", "filter": ["*.sas7bdat"] },
            "children": [
                {
                    "nodeId": "n2",
                    "description": "",
                    "path": "raw/data/a.sas7bdat",
                    "directory": false,
                    "parent": "n1",
                    "reference": "/opt/sop/raw/a.sas7bdat",
                    "migration": null,
                    "children": []
                }
            ]
        }"#;
        let node: NodeViewResponse = serde_json::from_str(j).unwrap();
        assert_eq!(node.node_id, "n1");
        assert!(node.directory);
        assert_eq!(node.parent.as_deref(), Some("n0"));
        assert_eq!(node.migration.unwrap().filter, vec!["*.sas7bdat"]);
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].node_id, "n2");
        assert!(!node.children[0].directory);
    }

    #[test]
    fn add_node_request_serializes_camel_case_with_a_null_parent() {
        let body = AddNodeRequest {
            path: "raw".into(),
            description: "d".into(),
            directory: true,
            parent: None,
            reference: None,
            migration: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert!(j.contains("\"parent\":null"), "got {j}");
        assert!(j.contains("\"directory\":true"), "got {j}");
        assert!(j.contains("\"path\":\"raw\""), "got {j}");
    }

    #[tokio::test]
    async fn delete_node_succeeds_on_204_no_body() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        server
            .register(
                Mock::given(method("DELETE"))
                    .and(path("/api/structures/7/nodes/n1"))
                    .respond_with(ResponseTemplate::new(204)),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);

        let result = delete_node(&c, 7, "n1").await;
        assert!(
            result.is_ok(),
            "delete_node must succeed on 204; got {result:?}"
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-desktop --lib http::structures`
Expected: FAIL — `cannot find type StructureLevelResponse` and friends.

- [ ] **Step 3: Implement the DTOs and the five calls**

Above the `#[cfg(test)] mod tests` block in the same file, write the module doc comment and the DTOs:

```rust
//! Directory-structure HTTP adapters. Wire DTOs mirror the server's
//! `apps/server/aegis-server/src/transport/http/dto.rs` §structures.

use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;
```

Then the eight structs/enums listed in this task's **Interfaces** block. Every one of them carries `#[serde(rename_all = "camelCase")]` and derives `Debug, Clone, PartialEq, Serialize, Deserialize` (the enums also derive `Copy, Eq, Hash`).

Then the five functions:

```rust
/// `GET /api/structures` — list structures, optionally filtered by level.
pub async fn list_structures(
    c: &HttpClient,
    level: Option<StructureLevelResponse>,
) -> Result<Vec<StructureSummaryResponse>, ApiError> {
    let mut url = "/api/structures".to_string();
    if let Some(l) = level {
        // Serialize the typed enum and strip the quotes, the same way
        // `http::mission` builds its `?kind=` query parameter.
        let level_str = serde_json::to_string(&l)
            .map_err(|e| ApiError::Parse {
                message: e.to_string(),
            })?
            .trim_matches('"')
            .to_string();
        url.push_str("?level=");
        url.push_str(&level_str);
    }
    let resp: StructureListResponse = c.request(reqwest::Method::GET, &url, None::<&()>).await?;
    Ok(resp.structures)
}

/// `GET /api/structures/{id}` — a structure with its effective tree.
pub async fn get_structure(c: &HttpClient, id: i64) -> Result<StructureViewResponse, ApiError> {
    let resp: StructureViewResponse = c
        .request(reqwest::Method::GET, &format!("/api/structures/{id}"), None::<&()>)
        .await?;
    Ok(resp)
}

/// `POST /api/structures/{id}/nodes` — place a node at a path.
pub async fn add_node(
    c: &HttpClient,
    structure_id: i64,
    body: &AddNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/structures/{structure_id}/nodes"),
        Some(body),
    )
    .await
}

/// `PUT /api/structures/{id}/nodes/{node_id}` — replace a node.
pub async fn update_node(
    c: &HttpClient,
    structure_id: i64,
    node_id: &str,
    body: &UpdateNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    c.request(
        reqwest::Method::PUT,
        &format!("/api/structures/{structure_id}/nodes/{node_id}"),
        Some(body),
    )
    .await
}

/// `DELETE /api/structures/{id}/nodes/{node_id}` — remove a node.
///
/// The server answers `204` with an empty body, so the adapter must not
/// try to JSON-decode it — `request::<Value>` would fail with
/// `decode_failed: EOF while parsing a value`. See `http::mission::remove_assignee`
/// for the same trap.
pub async fn delete_node(
    c: &HttpClient,
    structure_id: i64,
    node_id: &str,
) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/structures/{structure_id}/nodes/{node_id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}
```

- [ ] **Step 4: Register the module**

In `apps/desktop/aegis-desktop/src-tauri/src/http.rs`, add `pub mod structures;` to the module list (alphabetically, after `pub mod project;`).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib http::structures`
Expected: PASS — 4 tests.

- [ ] **Step 6: Lint**

Run: `cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/structures.rs apps/desktop/aegis-desktop/src-tauri/src/http.rs
git commit -m "feat(desktop): add the structures HTTP adapter"
```

---

### Task 3: Tauri commands for structures

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/structures.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` — add `pub mod structures;`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — add the five handlers to `generate_handler!`

**Interfaces:**
- Consumes: the five functions and the DTOs from Task 2.
- Produces: five `#[tauri::command]` functions, invoked from TypeScript as `list_structures`, `get_structure`, `add_structure_node`, `update_structure_node`, `delete_structure_node`. Each takes its arguments as separate parameters so Tauri maps the camelCase JSON keys onto them:
  - `list_structures(level: Option<String>)` → the level string is parsed into `StructureLevelResponse`.
  - `get_structure(id: i64)`
  - `add_structure_node(structure_id: i64, body: AddNodeRequest)`
  - `update_structure_node(structure_id: i64, node_id: String, body: UpdateNodeRequest)`
  - `delete_structure_node(structure_id: i64, node_id: String)`

- [ ] **Step 1: Write the failing camelCase deserialization test**

Create `apps/desktop/aegis-desktop/src-tauri/src/commands/structures.rs` with the imports and this test block, with the command bodies stubbed as `todo!()`:

```rust
#[cfg(test)]
mod tests {
    //! Wire-shape tests. Tauri routes JSON keys to function parameters by
    //! camelCase name, so the frontend wrappers must send exactly:
    //!
    //!   list_structures:       { level? }
    //!   get_structure:         { id }
    //!   add_structure_node:    { structureId, body }
    //!   update_structure_node: { structureId, nodeId, body }
    //!   delete_structure_node: { structureId, nodeId }
    //!
    //! These tests pin the deserialization shape so a future rename cannot
    //! silently desync Rust and TS.

    use super::*;
    use serde_json::json;

    #[test]
    fn add_node_request_deserializes_a_camel_case_payload() {
        let raw = json!({
            "path": "raw",
            "description": "d",
            "directory": true,
            "parent": null,
            "reference": null,
            "migration": null
        });
        let body: AddNodeRequest = serde_json::from_value(raw).unwrap();
        assert_eq!(body.path, "raw");
        assert!(body.directory);
        assert!(body.parent.is_none());
    }

    #[test]
    fn add_node_request_rejects_a_snake_case_payload() {
        let raw = json!({ "structure_id": 3, "path": "raw" });
        let result: Result<AddNodeRequest, _> = serde_json::from_value(raw);
        assert!(
            result.is_err(),
            "a snake_case payload must not parse against AddNodeRequest"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p aegis-desktop --lib commands::structures`
Expected: FAIL — `cannot find type AddNodeRequest` (the import is not there yet).

- [ ] **Step 3: Implement the five commands**

Above the test block, write the module. Each command follows the shape of `commands/mission.rs`: a `#[tauri::command]` function that builds a `RequestContext` and delegates to an `_impl` twin that takes `&SharedAppState` and `&RequestContext`, wrapped in a `tracing::info_span!` and a `TRACE_ID.scope(...)`.

```rust
use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::structures::{
    self, AddNodeRequest, NodeViewResponse, StructureLevelResponse, StructureSummaryResponse,
    StructureViewResponse, UpdateNodeRequest,
};
use crate::state::{Caller, RequestContext, SharedAppState};
```

`list_structures` parses its `level: Option<String>` through serde so an
unrecognised level comes back as `ApiError::Parse` rather than an opaque
Tauri decode error:

```rust
fn parse_level(s: &str) -> Result<StructureLevelResponse, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| ApiError::Parse {
        message: e.to_string(),
    })
}
```

The `_impl` twins have these signatures:

```rust
pub async fn list_structures_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    level: Option<String>,
) -> Result<Vec<StructureSummaryResponse>, ApiError>

pub async fn get_structure_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<StructureViewResponse, ApiError>

pub async fn add_structure_node_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    structure_id: i64,
    body: AddNodeRequest,
) -> Result<NodeViewResponse, ApiError>

pub async fn update_structure_node_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    structure_id: i64,
    node_id: String,
    body: UpdateNodeRequest,
) -> Result<NodeViewResponse, ApiError>

pub async fn delete_structure_node_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    structure_id: i64,
    node_id: String,
) -> Result<(), ApiError>
```

Each `_impl` body follows `commands/mission.rs::list_missions_by_project_impl`: open the span, `TRACE_ID.scope(req_ctx.trace_id.clone(), async { ... }).await`, log `success` or `failed`, and `.instrument(span).await` the whole thing. `Caller::User` is the caller for all five.

- [ ] **Step 4: Register the module and the commands**

In `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`, add `pub mod structures;` to the module list (alphabetically, after `pub mod project;`).

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, add a `// structures` block to `generate_handler!`, after the `// crf` block:

```rust
            // structures
            commands::structures::list_structures,
            commands::structures::get_structure,
            commands::structures::add_structure_node,
            commands::structures::update_structure_node,
            commands::structures::delete_structure_node,
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib commands::structures`
Expected: PASS — 2 tests.

- [ ] **Step 6: Lint**

Run: `cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/structures.rs apps/desktop/aegis-desktop/src-tauri/src/commands.rs apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add the structures Tauri commands"
```

---

### Task 4: TS wire types, API client and query keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/api.test.ts`

**Interfaces:**
- Consumes: the Rust command names from Task 3.
- Produces (used by Tasks 7–11):
  - `type StructureLevel = "template" | "project"`
  - `interface MigrationView { directory: string; filter: string[] }`
  - `interface StructureNodeView { nodeId: string; description: string; path: string; directory: boolean; parent: string | null; reference: string | null; migration: MigrationView | null; children: StructureNodeView[] }`
  - `interface StructureView { id: number; name: string; level: StructureLevel; projectCode: string | null; templateId: number | null; nodes: StructureNodeView[] }`
  - `interface StructureSummaryView { id: number; name: string; level: StructureLevel; projectCode: string | null; templateId: number | null }`
  - `interface StructureNodeInput { path: string; description: string; directory: boolean; parent: string | null; reference: string | null; migration: MigrationView | null }`
  - `api.listStructures(level?: StructureLevel): Promise<StructureSummaryView[]>`
  - `api.getStructure(id: number): Promise<StructureView>`
  - `api.addStructureNode(structureId: number, input: StructureNodeInput): Promise<StructureNodeView>`
  - `api.updateStructureNode(structureId: number, nodeId: string, input: StructureNodeInput): Promise<StructureNodeView>`
  - `api.deleteStructureNode(structureId: number, nodeId: string): Promise<void>`
  - `queryKeys.structures.list(level: StructureLevel): readonly ["structures", "list", StructureLevel]`
  - `queryKeys.structures.detail(id: number): readonly ["structures", "detail", number]`

- [ ] **Step 1: Write the failing API-caller test**

Create `apps/desktop/aegis-desktop/src/test/features/templates/api.test.ts`:

```ts
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../../../shared/api";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";

describe("structures api", () => {
  it("invokes list_structures with the level", async () => {
    mockCommands({ list_structures: () => [] });
    await api.listStructures("template");
    expect(mockInvoke).toHaveBeenCalledWith("list_structures", {
      level: "template",
    });
  });

  it("invokes get_structure with the id", async () => {
    mockCommands({ get_structure: () => ({}) });
    await api.getStructure(3);
    expect(mockInvoke).toHaveBeenCalledWith("get_structure", { id: 3 });
  });

  it("invokes add_structure_node with camelCase arguments", async () => {
    mockCommands({ add_structure_node: () => ({}) });
    await api.addStructureNode(3, {
      path: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith("add_structure_node", {
      structureId: 3,
      path: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
  });

  it("invokes update_structure_node with the node id", async () => {
    mockCommands({ update_structure_node: () => ({}) });
    await api.updateStructureNode(3, "n1", {
      path: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith(
      "update_structure_node",
      expect.objectContaining({ structureId: 3, nodeId: "n1" }),
    );
  });

  it("invokes delete_structure_node with the structure and node ids", async () => {
    mockCommands({ delete_structure_node: () => undefined });
    await api.deleteStructureNode(3, "n1");
    expect(mockInvoke).toHaveBeenCalledWith("delete_structure_node", {
      structureId: 3,
      nodeId: "n1",
    });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/api.test.ts`
Expected: FAIL — `api.listStructures` is not a function.

- [ ] **Step 3: Add the TS wire types**

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, append:

```ts
export type StructureLevel = "template" | "project";

export interface MigrationView {
  directory: string;
  filter: string[];
}

export interface StructureNodeView {
  nodeId: string;
  description: string;
  path: string;
  directory: boolean;
  parent: string | null;
  reference: string | null;
  migration: MigrationView | null;
  children: StructureNodeView[];
}

export interface StructureView {
  id: number;
  name: string;
  level: StructureLevel;
  projectCode: string | null;
  templateId: number | null;
  nodes: StructureNodeView[];
}

export interface StructureSummaryView {
  id: number;
  name: string;
  level: StructureLevel;
  projectCode: string | null;
  templateId: number | null;
}

export interface StructureNodeInput {
  path: string;
  description: string;
  directory: boolean;
  parent: string | null;
  reference: string | null;
  migration: MigrationView | null;
}
```

- [ ] **Step 4: Add the API client wrappers**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`, add the six new type names to the `import type { ... } from "./types"` block, then add a `// structures` section to the `api` object:

```ts
  // structures
  listStructures: (level?: StructureLevel): Promise<StructureSummaryView[]> =>
    call<StructureSummaryView[]>("list_structures", { level }),
  getStructure: (id: number): Promise<StructureView> =>
    call<StructureView>("get_structure", { id }),
  addStructureNode: (
    structureId: number,
    input: StructureNodeInput,
  ): Promise<StructureNodeView> =>
    call<StructureNodeView>("add_structure_node", { structureId, ...input }),
  updateStructureNode: (
    structureId: number,
    nodeId: string,
    input: StructureNodeInput,
  ): Promise<StructureNodeView> =>
    call<StructureNodeView>("update_structure_node", {
      structureId,
      nodeId,
      ...input,
    }),
  deleteStructureNode: (structureId: number, nodeId: string): Promise<void> =>
    call<void>("delete_structure_node", { structureId, nodeId }),
```

- [ ] **Step 5: Add the query keys**

In `apps/desktop/aegis-desktop/src/shared/query/keys.ts`, add at the top of the file (above `queryKeys`):

```ts
import type { StructureLevel } from "../api/types";
```

and add a `structures` group to the `queryKeys` object (after `mission`):

```ts
  structures: {
    list: (level: StructureLevel) => ["structures", "list", level] as const,
    detail: (id: number) => ["structures", "detail", id] as const,
  },
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/api.test.ts`
Expected: PASS — 5 tests.

- [ ] **Step 7: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts apps/desktop/aegis-desktop/src/shared/api/index.ts apps/desktop/aegis-desktop/src/shared/query/keys.ts apps/desktop/aegis-desktop/src/test/features/templates/api.test.ts
git commit -m "feat(desktop): add the structures API client and query keys"
```

---

### Task 5: i18n catalog

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: the `TranslationKey` values listed below. Tasks 8–11 use exactly these keys.

- [ ] **Step 1: Add the English keys**

In `lib/packages/ui/src/i18n/locales/en.ts`, add:

```ts
  'nav.templates': 'Templates',
  'templates.heading': 'Templates',
  'templates.card.standardFolderStructure.title':
    'Standard Project Folder Structure',
  'templates.card.standardFolderStructure.description':
    'Statistical programming standard project folder structure, follow SOP CL-CSP-DOC-004-A01',
  'templates.structure.heading': 'Project Structure Template',
  'templates.structure.view.complete': 'Complete',
  'templates.structure.view.flat': 'Flat',
  'templates.structure.noTemplates': 'No templates yet',
  'templates.structure.noNodes': 'No folder structure yet',
  'templates.node.add': 'Add node',
  'templates.node.edit': 'Edit node',
  'templates.node.delete': 'Delete node',
  'templates.node.addRoot': 'Add root node',
  'templates.node.drawer.addTitle': 'Add node',
  'templates.node.drawer.editTitle': 'Edit node',
  'templates.node.field.path': 'Path',
  'templates.node.field.description': 'Description',
  'templates.node.field.directory': 'Folder',
  'templates.node.field.reference': 'Reference path',
  'templates.node.field.migration': 'Migration',
  'templates.node.field.migrationDirectory': 'Migration source directory',
  'templates.node.field.migrationFilter': 'Migration filter',
  'templates.node.field.invalidPath':
    'Enter a relative path — no leading or trailing "/", and no "." or ".." segment.',
  'templates.node.saveFailed': 'Save failed: {message}',
  'templates.node.delete.title': 'Delete node',
  'templates.node.delete.message':
    'Delete "{path}" and everything under it?',
  'templates.node.delete.failed': 'Delete failed: {message}',
```

- [ ] **Step 2: Add the Chinese keys**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, add the same keys with these values:

```ts
  'nav.templates': '模板',
  'templates.heading': '模板',
  'templates.card.standardFolderStructure.title': '标准项目文件夹结构',
  'templates.card.standardFolderStructure.description':
    '统计编程工作标准文件夹结构, 遵循SOP CL-CSP-DOC-004-A01',
  'templates.structure.heading': '项目结构模板',
  'templates.structure.view.complete': '完整',
  'templates.structure.view.flat': '扁平',
  'templates.structure.noTemplates': '暂无模板',
  'templates.structure.noNodes': '暂无文件夹结构',
  'templates.node.add': '添加节点',
  'templates.node.edit': '编辑节点',
  'templates.node.delete': '删除节点',
  'templates.node.addRoot': '添加根节点',
  'templates.node.drawer.addTitle': '新增节点',
  'templates.node.drawer.editTitle': '编辑节点',
  'templates.node.field.path': '路径',
  'templates.node.field.description': '描述',
  'templates.node.field.directory': '文件夹',
  'templates.node.field.reference': '参考路径',
  'templates.node.field.migration': '迁移',
  'templates.node.field.migrationDirectory': '迁移源目录',
  'templates.node.field.migrationFilter': '迁移过滤条件',
  'templates.node.field.invalidPath':
    '请输入相对路径 —— 不能以 "/" 开头或结尾, 也不能包含 "." 或 ".." 段。',
  'templates.node.saveFailed': '保存失败: {message}',
  'templates.node.delete.title': '删除节点',
  'templates.node.delete.message': '删除 "{path}" 及其所有子节点?',
  'templates.node.delete.failed': '删除失败: {message}',
```

- [ ] **Step 3: Verify the catalogs agree**

Run: `pnpm --filter @aegis/ui typecheck`
Expected: PASS. The `satisfies Record<Locale, TranslationCatalog>` in `registry.ts` fails the build if `zhCN.ts` is missing a key that `en.ts` has, so a PASS means both catalogs carry all 27 keys.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): add the templates catalog entries"
```

---

### Task 6: The flat-tree transform

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/templates/data/tree.ts`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/tree.test.ts`

**Interfaces:**
- Consumes: `StructureNodeView` from `src/shared/api`.
- Produces (used by Tasks 9 and 11):
  - `interface FlatEntry { node: StructureNodeView; name: string; fullPath: string; files: StructureNodeView[] }`
  - `function flattenStructure(nodes: StructureNodeView[]): FlatEntry[]`
  - `function nodeName(path: string): string` — the last `/`-separated segment of a path.

- [ ] **Step 1: Write the failing tests**

Create `apps/desktop/aegis-desktop/src/test/features/templates/tree.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import type { StructureNodeView } from "../../../shared/api";
import { flattenStructure, nodeName } from "../../../features/templates/data/tree";

function dir(
  nodeId: string,
  path: string,
  children: StructureNodeView[] = [],
): StructureNodeView {
  return {
    nodeId,
    description: "",
    path,
    directory: true,
    parent: null,
    reference: null,
    migration: null,
    children,
  };
}

function file(nodeId: string, path: string): StructureNodeView {
  return {
    nodeId,
    description: "",
    path,
    directory: false,
    parent: null,
    reference: "/abs/path",
    migration: null,
    children: [],
  };
}

describe("flattenStructure", () => {
  it("promotes every directory to a top-level entry", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "raw/data")]),
    ]);
    expect(entries.map((e) => e.node.nodeId)).toEqual(["d1", "d2"]);
  });

  it("does not nest a sub-directory under its parent", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "raw/data")]),
    ]);
    const raw = entries.find((e) => e.node.nodeId === "d1")!;
    expect(raw.files).toEqual([]);
    expect(raw.node.children.map((c) => c.nodeId)).toEqual(["d2"]);
  });

  it("collects the files directly inside each directory", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [
        file("f1", "raw/b.txt"),
        dir("d2", "raw/data", [file("f2", "raw/data/a.sas7bdat")]),
      ]),
    ]);
    const raw = entries.find((e) => e.node.nodeId === "d1")!;
    const data = entries.find((e) => e.node.nodeId === "d2")!;
    expect(raw.files.map((f) => f.nodeId)).toEqual(["f1"]);
    expect(data.files.map((f) => f.nodeId)).toEqual(["f2"]);
  });

  it("gives a root-level file its own top-level entry", () => {
    const entries = flattenStructure([file("f1", "README.md")]);
    expect(entries).toHaveLength(1);
    expect(entries[0].node.nodeId).toBe("f1");
    expect(entries[0].files).toEqual([]);
  });

  it("names an entry by the last segment of its path and keeps the full path", () => {
    const entries = flattenStructure([dir("d1", "raw", [dir("d2", "raw/data")])]);
    const data = entries.find((e) => e.node.nodeId === "d2")!;
    expect(data.name).toBe("data");
    expect(data.fullPath).toBe("raw/data");
  });

  it("returns an empty list for an empty tree", () => {
    expect(flattenStructure([])).toEqual([]);
  });
});

describe("nodeName", () => {
  it("returns the last path segment", () => {
    expect(nodeName("raw/data")).toBe("data");
  });

  it("returns the whole path when it has no separator", () => {
    expect(nodeName("raw")).toBe("raw");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/tree.test.ts`
Expected: FAIL — cannot resolve `features/templates/data/tree`.

- [ ] **Step 3: Implement the transform**

Create `apps/desktop/aegis-desktop/src/features/templates/data/tree.ts`:

```ts
import type { StructureNodeView } from "../../../shared/api";

/**
 * One top-level entry of the flat view.
 *
 * A directory becomes an entry of its own; the files sitting directly
 * inside it become that entry's leaves. A sub-directory never nests
 * under its parent — it gets its own entry.
 */
export interface FlatEntry {
  /** The directory node, or a root-level file. */
  node: StructureNodeView;
  /** The node's own name — the last segment of its path. */
  name: string;
  /** The node's full path relative to the structure root. */
  fullPath: string;
  /** The files directly inside the directory. Empty for a file entry. */
  files: StructureNodeView[];
}

/** The last `/`-separated segment of a path. */
export function nodeName(path: string): string {
  return path.split("/").pop() ?? path;
}

/**
 * Flatten a structure tree into one entry per directory.
 *
 * Every directory, at any depth, becomes a top-level entry; a
 * sub-directory is never nested under its parent. The files directly
 * inside a directory become that directory's leaves. A file with no
 * parent directory has nothing to sit under, so it gets its own entry.
 */
export function flattenStructure(nodes: StructureNodeView[]): FlatEntry[] {
  const entries: FlatEntry[] = [];

  const walk = (siblings: StructureNodeView[], atRoot: boolean) => {
    for (const node of siblings) {
      if (node.directory) {
        entries.push({
          node,
          name: nodeName(node.path),
          fullPath: node.path,
          files: node.children.filter((c) => !c.directory),
        });
        walk(
          node.children.filter((c) => c.directory),
          false,
        );
      } else if (atRoot) {
        entries.push({
          node,
          name: nodeName(node.path),
          fullPath: node.path,
          files: [],
        });
      }
    }
  };

  walk(nodes, true);
  return entries;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/tree.test.ts`
Expected: PASS — 8 tests.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/templates/data/tree.ts apps/desktop/aegis-desktop/src/test/features/templates/tree.test.ts
git commit -m "feat(desktop): add the flat structure tree transform"
```

---

### Task 7: The feature data hooks

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/templates/data/structures.ts`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/data.test.tsx`

**Interfaces:**
- Consumes: `api.*` and `queryKeys.structures.*` from Task 4.
- Produces (used by Tasks 9–11):
  - `useListTemplateStructures(): UseQueryResult<StructureSummaryView[], ApiError>` — query key `queryKeys.structures.list("template")`, query fn `api.listStructures("template")`.
  - `useGetStructure(id: number | null): UseQueryResult<StructureView, ApiError>` — query key `queryKeys.structures.detail(id ?? 0)`, `enabled: id != null && id > 0`.
  - `useAddStructureNode(): UseMutation<StructureNodeView, ApiError, { structureId: number; input: StructureNodeInput }>`
  - `useUpdateStructureNode(): UseMutation<StructureNodeView, ApiError, { structureId: number; nodeId: string; input: StructureNodeInput }>`
  - `useDeleteStructureNode(): UseMutation<void, ApiError, { structureId: number; nodeId: string }>`
  - Every mutation invalidates `queryKeys.structures.detail(structureId)` and `queryKeys.structures.list("template")` on success.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/templates/data.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithQueryClient } from "../../helpers/render-with-query-client";
import { mockCommands } from "../../helpers/tauri-mock";
import {
  useAddStructureNode,
  useDeleteStructureNode,
  useGetStructure,
  useListTemplateStructures,
  useUpdateStructureNode,
} from "../../../features/templates/data/structures";

afterEach(() => cleanup());

function Probe() {
  const list = useListTemplateStructures();
  const structure = useGetStructure(3);
  const add = useAddStructureNode();
  const update = useUpdateStructureNode();
  const del = useDeleteStructureNode();
  return (
    <div>
      <span data-testid="list-status">{list.status}</span>
      <span data-testid="list-count">{list.data?.length ?? -1}</span>
      <span data-testid="structure-status">{structure.status}</span>
      <span data-testid="structure-name">{structure.data?.name ?? ""}</span>
      <button
        data-testid="add"
        onClick={() =>
          add.mutate({
            structureId: 3,
            input: {
              path: "raw",
              description: "",
              directory: true,
              parent: null,
              reference: null,
              migration: null,
            },
          })
        }
      >
        add
      </button>
      <button
        data-testid="delete"
        onClick={() => del.mutate({ structureId: 3, nodeId: "n1" })}
      >
        delete
      </button>
    </div>
  );
}

describe("templates data hooks", () => {
  it("lists the template structures", async () => {
    mockCommands({
      list_structures: () => ({
        structures: [
          {
            id: 3,
            name: "Standard",
            level: "template",
            projectCode: null,
            templateId: null,
          },
        ],
      }),
    });
    renderWithQueryClient(<Probe />);
    expect(await screen.findByText("1")).toBeInTheDocument();
  });

  it("loads a structure by id", async () => {
    mockCommands({
      list_structures: () => ({ structures: [] }),
      get_structure: () => ({
        id: 3,
        name: "Standard",
        level: "template",
        projectCode: null,
        templateId: null,
        nodes: [],
      }),
    });
    renderWithQueryClient(<Probe />);
    expect(await screen.findByText("Standard")).toBeInTheDocument();
  });

  it("invalidates the structure and the list after a node is added", async () => {
    mockCommands({
      list_structures: () => ({ structures: [] }),
      get_structure: () => ({
        id: 3,
        name: "Standard",
        level: "template",
        projectCode: null,
        templateId: null,
        nodes: [],
      }),
      add_structure_node: () => ({ nodeId: "n9" }),
    });
    const { client } = renderWithQueryClient(<Probe />);
    await screen.findByText("Standard");
    await userEvent.click(screen.getByTestId("add"));
    await waitFor(() =>
      expect(
        client.getQueryState(["structures", "detail", 3])?.isInvalidated,
      ).toBe(true),
    );
    expect(
      client.getQueryState(["structures", "list", "template"])?.isInvalidated,
    ).toBe(true);
  });

  it("invalidates after a node is deleted", async () => {
    mockCommands({
      list_structures: () => ({ structures: [] }),
      get_structure: () => ({
        id: 3,
        name: "Standard",
        level: "template",
        projectCode: null,
        templateId: null,
        nodes: [],
      }),
      delete_structure_node: () => undefined,
    });
    const { client } = renderWithQueryClient(<Probe />);
    await screen.findByText("Standard");
    await userEvent.click(screen.getByTestId("delete"));
    await waitFor(() =>
      expect(
        client.getQueryState(["structures", "detail", 3])?.isInvalidated,
      ).toBe(true),
    );
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/data.test.tsx`
Expected: FAIL — cannot resolve `features/templates/data/structures`.

- [ ] **Step 3: Implement the hooks**

Create `apps/desktop/aegis-desktop/src/features/templates/data/structures.ts`:

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type StructureNodeInput,
  type StructureNodeView,
  type StructureSummaryView,
  type StructureView,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/** The template-level structures, for the sidebar. */
export function useListTemplateStructures() {
  return useQuery<StructureSummaryView[], ApiError>({
    queryKey: queryKeys.structures.list("template"),
    queryFn: () => api.listStructures("template"),
  });
}

/** One structure with its effective tree. */
export function useGetStructure(id: number | null) {
  return useQuery<StructureView, ApiError>({
    queryKey: queryKeys.structures.detail(id ?? 0),
    queryFn: () => api.getStructure(id!),
    enabled: id != null && id > 0,
  });
}

/** Invalidate the structure and the sidebar after a node write. */
function useInvalidateStructures() {
  const qc = useQueryClient();
  return (structureId: number) => {
    qc.invalidateQueries({ queryKey: queryKeys.structures.detail(structureId) });
    qc.invalidateQueries({ queryKey: queryKeys.structures.list("template") });
  };
}

export function useAddStructureNode() {
  const invalidate = useInvalidateStructures();
  return useMutation<
    StructureNodeView,
    ApiError,
    { structureId: number; input: StructureNodeInput }
  >({
    mutationFn: ({ structureId, input }) =>
      api.addStructureNode(structureId, input),
    onSuccess: (_node, vars) => invalidate(vars.structureId),
  });
}

export function useUpdateStructureNode() {
  const invalidate = useInvalidateStructures();
  return useMutation<
    StructureNodeView,
    ApiError,
    { structureId: number; nodeId: string; input: StructureNodeInput }
  >({
    mutationFn: ({ structureId, nodeId, input }) =>
      api.updateStructureNode(structureId, nodeId, input),
    onSuccess: (_node, vars) => invalidate(vars.structureId),
  });
}

export function useDeleteStructureNode() {
  const invalidate = useInvalidateStructures();
  return useMutation<
    void,
    ApiError,
    { structureId: number; nodeId: string }
  >({
    mutationFn: ({ structureId, nodeId }) =>
      api.deleteStructureNode(structureId, nodeId),
    onSuccess: (_void, vars) => invalidate(vars.structureId),
  });
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/data.test.tsx`
Expected: PASS — 4 tests.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/templates/data/structures.ts apps/desktop/aegis-desktop/src/test/features/templates/data.test.tsx
git commit -m "feat(desktop): add the templates data hooks"
```

---

### Task 8: The Templates page, its route and the nav entry

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/templates/pages/TemplatesPage.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/templates/index.ts`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/templates-page.test.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/app/app-layout-templates.test.tsx`

**Interfaces:**
- Consumes: the i18n keys from Task 5.
- Produces: `TemplatesPage` (exported from `src/features/templates/index.ts`), the route `/templates`, and a "Templates" entry in the Knowledge Base submenu.

- [ ] **Step 1: Write the failing page test**

Create `apps/desktop/aegis-desktop/src/test/features/templates/templates-page.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { renderInRouter } from "../../helpers/file-route-utils";
import { TemplatesPage } from "../../../features/templates";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

beforeEach(() => {
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderTemplates() {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <TemplatesPage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("TemplatesPage", () => {
  it("renders the heading and the one card", async () => {
    await renderTemplates();
    expect(
      screen.getByRole("heading", { level: 4, name: /templates/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Standard Project Folder Structure"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Statistical programming standard project folder structure/i,
      ),
    ).toBeInTheDocument();
  });

  it("navigates to /templates/structure when the card is clicked", async () => {
    const { router } = await renderTemplates();
    await userEvent.click(
      screen.getByText("Standard Project Folder Structure"),
    );
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/templates/structure"),
    );
  });
});
```

- [ ] **Step 2: Write the failing nav test**

Create `apps/desktop/aegis-desktop/src/test/features/app/app-layout-templates.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

beforeEach(() => {
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderRoot() {
  return renderWithFullRouter({
    initialEntries: ["/"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("AppLayout — Templates sub-menu entry", () => {
  it("shows Templates under Knowledge Base", async () => {
    mockCommands({ is_logged_in: () => true });
    await renderRoot();
    await userEvent.click(await screen.findByText("Knowledge Base"));
    expect(screen.getByText("Templates")).toBeInTheDocument();
  });

  it("navigates to /templates when the Templates sub-menu is clicked", async () => {
    mockCommands({ is_logged_in: () => true });
    const { router } = await renderRoot();
    await userEvent.click(await screen.findByText("Knowledge Base"));
    await userEvent.click(screen.getByText("Templates"));
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/templates"),
    );
  });
});
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/templates-page.test.tsx src/test/features/app/app-layout-templates.test.tsx`
Expected: FAIL — `TemplatesPage` is not exported, and "Templates" is not in the sidebar.

- [ ] **Step 4: Implement the Templates page**

Create `apps/desktop/aegis-desktop/src/features/templates/pages/TemplatesPage.tsx`:

```tsx
import { useNavigate } from "@tanstack/react-router";
import { Box, Card, CardContent, CardHeader, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

/**
 * The template catalogue. The entries are fixed text in the frontend —
 * they are not fetched — so each card is a launcher into one of the
 * template pages rather than a row of API data.
 */
const TEMPLATES = [
  {
    titleKey: "templates.card.standardFolderStructure.title" as const,
    descriptionKey:
      "templates.card.standardFolderStructure.description" as const,
    target: "/templates/structure",
  },
];

export function TemplatesPage() {
  const { t } = useI18n();
  const navigate = useNavigate();

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 3 }}>
      <Typography variant="h4">{t("templates.heading")}</Typography>

      <Box sx={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
        {TEMPLATES.map((template) => (
          <Card
            key={template.target}
            sx={{ flex: 1, minWidth: 320, cursor: "pointer" }}
            onClick={() => navigate({ to: template.target })}
          >
            <CardHeader title={t(template.titleKey)} />
            <CardContent>
              <Typography variant="body2">
                {t(template.descriptionKey)}
              </Typography>
            </CardContent>
          </Card>
        ))}
      </Box>
    </Box>
  );
}
```

- [ ] **Step 5: Export the feature module**

Create `apps/desktop/aegis-desktop/src/features/templates/index.ts`:

```ts
export { TemplatesPage } from "./pages/TemplatesPage";
```

- [ ] **Step 6: Add the route**

Create `apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { TemplatesPage } from "../../../features/templates";

export const Route = createFileRoute("/_authed/_layout/templates")({
  component: TemplatesPage,
});
```

- [ ] **Step 7: Regenerate the route tree**

The TanStack Router Vite plugin writes `src/routes/routeTree.gen.ts` during a
Vite build, and `pnpm --filter aegis-desktop build` runs `tsc` *before*
`vite build` — so the tree must be regenerated before the typecheck passes.

Run: `pnpm --filter aegis-desktop exec vite build`
Expected: `src/routes/routeTree.gen.ts` now contains a `_authed/_layout/templates` entry.

- [ ] **Step 8: Add the nav entry**

In `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`:

Add an icon import and a component, next to the existing menu icons:

```tsx
import {
  AccountTree as AccountTreeIcon,
  ...
} from "@aegis/ui/icons";

const TemplatesMenuIcon = () => <AccountTreeIcon />;
```

Add the entry to the Knowledge Base submenu in `metadataEntry`:

```tsx
    subMenu: [
      {
        link: "/metadata",
        title: t("nav.metadata"),
        icon: MetadataMenuIcon,
      },
      {
        link: "/templates",
        title: t("nav.templates"),
        icon: TemplatesMenuIcon,
      },
    ],
```

- [ ] **Step 9: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/templates-page.test.tsx src/test/features/app/app-layout-templates.test.tsx`
Expected: PASS — 4 tests.

- [ ] **Step 10: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/templates apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates.tsx apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx apps/desktop/aegis-desktop/src/test/features/templates/templates-page.test.tsx apps/desktop/aegis-desktop/src/test/features/app/app-layout-templates.test.tsx
git commit -m "feat(desktop): add the templates page and its nav entry"
```

---

### Task 9: The node row and its action menu

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/templates/components/StructureNodeRow.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/templates/components/NodeActionMenu.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/node-row.test.tsx`

**Interfaces:**
- Consumes: `StructureNodeView` from `src/shared/api`; the i18n keys from Task 5.
- Produces:
  - `NodeActionMenuProps { node: StructureNodeView; onAdd: (node: StructureNodeView) => void; onEdit: (node: StructureNodeView) => void; onDelete: (node: StructureNodeView) => void }` — a `MoreVert` `IconButton` that opens a `Menu` of three entries (add / edit / delete), each an icon plus its label.
  - `StructureNodeRowProps { node: StructureNodeView; canMutate: boolean; onAdd: (node: StructureNodeView) => void; onEdit: (node: StructureNodeView) => void; onDelete: (node: StructureNodeView) => void; label?: string; secondaryPath?: string }` — the row's primary label is `label` when given, otherwise `node.path`; `secondaryPath` (used by the flat view) is shown as an extra line beneath the label.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/templates/node-row.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureNodeView } from "../../../shared/api";
import { StructureNodeRow } from "../../../features/templates/components/StructureNodeRow";

afterEach(() => cleanup());

const DIRECTORY: StructureNodeView = {
  nodeId: "n1",
  description: "Statistical programming raw data",
  path: "raw/data",
  directory: true,
  parent: "n0",
  reference: null,
  migration: { directory: "/opt/sop/raw", filter: ["*.sas7bdat"] },
  children: [],
};

const FILE: StructureNodeView = {
  nodeId: "n2",
  description: "",
  path: "raw/data/a.sas7bdat",
  directory: false,
  parent: "n1",
  reference: "/opt/sop/raw/a.sas7bdat",
  migration: null,
  children: [],
};

function renderRow(
  node: StructureNodeView,
  props: Partial<ComponentProps<typeof StructureNodeRow>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <StructureNodeRow
          node={node}
          canMutate={false}
          onAdd={() => {}}
          onEdit={() => {}}
          onDelete={() => {}}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("StructureNodeRow", () => {
  it("shows the node path", () => {
    renderRow(DIRECTORY);
    expect(screen.getByText("raw/data")).toBeInTheDocument();
  });

  it("shows the description, the migration source and the filters", () => {
    renderRow(DIRECTORY);
    expect(
      screen.getByText("Statistical programming raw data"),
    ).toBeInTheDocument();
    expect(screen.getByText("/opt/sop/raw")).toBeInTheDocument();
    expect(screen.getByText("*.sas7bdat")).toBeInTheDocument();
  });

  it("omits the empty description instead of rendering a blank line", () => {
    renderRow(FILE);
    expect(screen.getByText("raw/data/a.sas7bdat")).toBeInTheDocument();
    expect(screen.queryByText("Migration")).not.toBeInTheDocument();
  });

  it("shows the reference path of a file", () => {
    renderRow(FILE);
    expect(screen.getByText("/opt/sop/raw/a.sas7bdat")).toBeInTheDocument();
  });

  it("shows the given label and the extra full path", () => {
    renderRow(DIRECTORY, { label: "data", secondaryPath: "raw/data" });
    expect(screen.getByText("data")).toBeInTheDocument();
    expect(screen.getByText("raw/data")).toBeInTheDocument();
  });

  it("hides the action menu from a read-only user", () => {
    renderRow(DIRECTORY, { canMutate: false });
    expect(
      screen.queryByRole("button", { name: /node actions/i }),
    ).not.toBeInTheDocument();
  });

  it("offers add, edit and delete to an admin", async () => {
    const onAdd = vi.fn();
    renderRow(DIRECTORY, { canMutate: true, onAdd });
    await userEvent.click(
      screen.getByRole("button", { name: /node actions/i }),
    );
    expect(screen.getByText("Add node")).toBeInTheDocument();
    expect(screen.getByText("Edit node")).toBeInTheDocument();
    expect(screen.getByText("Delete node")).toBeInTheDocument();
    await userEvent.click(screen.getByText("Add node"));
    expect(onAdd).toHaveBeenCalledWith(DIRECTORY);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/node-row.test.tsx`
Expected: FAIL — cannot resolve `features/templates/components/StructureNodeRow`.

- [ ] **Step 3: Implement the action menu**

Create `apps/desktop/aegis-desktop/src/features/templates/components/NodeActionMenu.tsx`:

```tsx
import { useState } from "react";
import { IconButton, Menu, MenuItem } from "@aegis/ui/mui";
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  MoreHoriz as MoreHorizIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { StructureNodeView } from "../../../shared/api";

export interface NodeActionMenuProps {
  node: StructureNodeView;
  onAdd: (node: StructureNodeView) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
}

/**
 * The per-node admin menu. The spec asks for "a list of icon buttons";
 * each entry carries an action icon *and* its label, so the three
 * actions are readable rather than merely recognisable.
 */
export function NodeActionMenu({
  node,
  onAdd,
  onEdit,
  onDelete,
}: NodeActionMenuProps) {
  const { t } = useI18n();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);

  const close = () => setAnchor(null);

  return (
    <>
      <IconButton
        size="small"
        aria-label={t("templates.node.add")}
        onClick={(e) => setAnchor(e.currentTarget)}
      >
        <MoreHorizIcon fontSize="small" />
      </IconButton>
      <Menu anchorEl={anchor} open={anchor != null} onClose={close}>
        <MenuItem
          onClick={() => {
            close();
            onAdd(node);
          }}
        >
          <AddIcon fontSize="small" sx={{ mr: 1 }} />
          {t("templates.node.add")}
        </MenuItem>
        <MenuItem
          onClick={() => {
            close();
            onEdit(node);
          }}
        >
          <EditIcon fontSize="small" sx={{ mr: 1 }} />
          {t("templates.node.edit")}
        </MenuItem>
        <MenuItem
          onClick={() => {
            close();
            onDelete(node);
          }}
        >
          <DeleteIcon fontSize="small" sx={{ mr: 1 }} />
          {t("templates.node.delete")}
        </MenuItem>
      </Menu>
    </>
  );
}
```

- [ ] **Step 4: Implement the node row**

Create `apps/desktop/aegis-desktop/src/features/templates/components/StructureNodeRow.tsx`:

```tsx
import { Box, Stack, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { StructureNodeView } from "../../../shared/api";
import { NodeActionMenu } from "./NodeActionMenu";

export interface StructureNodeRowProps {
  node: StructureNodeView;
  canMutate: boolean;
  onAdd: (node: StructureNodeView) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
  /** The flat view's extra full-path line. */
  secondaryPath?: string;
}

/**
 * One node's row: the path as the label, then the description, the
 * reference and the migration as secondary text. An empty description
 * is skipped rather than rendered as a blank line.
 */
export function StructureNodeRow({
  node,
  canMutate,
  onAdd,
  onEdit,
  onDelete,
  label,
  secondaryPath,
}: StructureNodeRowProps) {
  const { t } = useI18n();

  return (
    <Stack direction="row" spacing={1} alignItems="flex-start">
      <Box sx={{ flexGrow: 1, minWidth: 0 }}>
        <Typography variant="body2" fontWeight={600} noWrap>
          {label ?? node.path}
        </Typography>
        {secondaryPath && (
          <Typography variant="caption" color="text.secondary" noWrap>
            {secondaryPath}
          </Typography>
        )}
        {node.description && (
          <Typography variant="caption" color="text.secondary" noWrap>
            {node.description}
          </Typography>
        )}
        {node.reference && (
          <Typography variant="caption" color="text.secondary" noWrap>
            {node.reference}
          </Typography>
        )}
        {node.migration && (
          <>
            <Typography variant="caption" color="text.secondary" noWrap>
              {t("templates.node.field.migration")}
            </Typography>
            <Typography variant="caption" color="text.secondary" noWrap>
              {node.migration.directory}
            </Typography>
            {node.migration.filter.map((f) => (
              <Typography
                key={f}
                variant="caption"
                color="text.secondary"
                noWrap
              >
                {f}
              </Typography>
            ))}
          </>
        )}
      </Box>
      {canMutate && (
        <NodeActionMenu
          node={node}
          onAdd={onAdd}
          onEdit={onEdit}
          onDelete={onDelete}
        />
      )}
    </Stack>
  );
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/node-row.test.tsx`
Expected: PASS — 7 tests.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/templates/components/StructureNodeRow.tsx apps/desktop/aegis-desktop/src/features/templates/components/NodeActionMenu.tsx apps/desktop/aegis-desktop/src/test/features/templates/node-row.test.tsx
git commit -m "feat(desktop): add the structure node row and its action menu"
```

---

### Task 10: The node drawer and the delete dialog

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/templates/components/NodeDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/templates/components/DeleteNodeDialog.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/node-drawer.test.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/delete-node-dialog.test.tsx`

**Interfaces:**
- Consumes: `StructureNodeInput` and `StructureNodeView` from `src/shared/api`; the i18n keys from Task 5; `errorMessage` from `src/shared/api/error`.
- Produces:
  - `NodeDrawerProps { open: boolean; mode: "create" | "edit"; structureId: number; node: StructureNodeView | null; parent: StructureNodeView | null; onClose: () => void; onSubmit: (input: StructureNodeInput) => void; pending: boolean; error: ApiError | null }`
  - `DeleteNodeDialogProps { open: boolean; node: StructureNodeView | null; onClose: () => void; onConfirm: (node: StructureNodeView) => void; pending: boolean; error: unknown }`
  - `isValidNodePath(path: string): boolean` — exported from `NodeDrawer.tsx` so the page can reuse it.

- [ ] **Step 1: Write the failing drawer test**

Create `apps/desktop/aegis-desktop/src/test/features/templates/node-drawer.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureNodeView } from "../../../shared/api";
import {
  NodeDrawer,
  isValidNodePath,
} from "../../../features/templates/components/NodeDrawer";

afterEach(() => cleanup());

const PARENT: StructureNodeView = {
  nodeId: "n0",
  description: "",
  path: "raw",
  directory: true,
  parent: null,
  reference: null,
  migration: null,
  children: [],
};

function renderDrawer(props: Partial<ComponentProps<typeof NodeDrawer>> = {}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <NodeDrawer
          open
          mode="create"
          structureId={3}
          node={null}
          parent={PARENT}
          onClose={() => {}}
          onSubmit={() => {}}
          pending={false}
          error={null}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("isValidNodePath", () => {
  it("accepts a plain relative path", () => {
    expect(isValidNodePath("raw/data")).toBe(true);
  });

  it("rejects a blank path", () => {
    expect(isValidNodePath("   ")).toBe(false);
  });

  it("rejects a leading or trailing slash", () => {
    expect(isValidNodePath("/raw")).toBe(false);
    expect(isValidNodePath("raw/")).toBe(false);
  });

  it("rejects a dot or dot-dot segment", () => {
    expect(isValidNodePath("raw/./data")).toBe(false);
    expect(isValidNodePath("raw/../data")).toBe(false);
  });

  it("rejects a backslash and an empty segment", () => {
    expect(isValidNodePath("raw\\data")).toBe(false);
    expect(isValidNodePath("raw//data")).toBe(false);
  });
});

describe("NodeDrawer", () => {
  it("prefills the path with the parent's path when adding", () => {
    renderDrawer();
    expect(screen.getByLabelText("Path")).toHaveValue("raw/");
  });

  it("shows the reference field only for a file", async () => {
    renderDrawer();
    expect(screen.queryByLabelText("Reference path")).not.toBeInTheDocument();
    await userEvent.click(screen.getByLabelText("Folder"));
    expect(screen.getByLabelText("Reference path")).toBeInTheDocument();
    expect(
      screen.queryByLabelText("Migration source directory"),
    ).not.toBeInTheDocument();
  });

  it("shows the migration fields only for a directory", async () => {
    renderDrawer();
    expect(
      screen.getByLabelText("Migration source directory"),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByLabelText("Folder"));
    expect(
      screen.queryByLabelText("Migration source directory"),
    ).not.toBeInTheDocument();
  });

  it("disables Save while the path is invalid", async () => {
    renderDrawer();
    const save = screen.getByRole("button", { name: "Create" });
    expect(save).toBeDisabled();
    await userEvent.clear(screen.getByLabelText("Path"));
    await userEvent.type(screen.getByLabelText("Path"), "raw//data");
    expect(save).toBeDisabled();
    await userEvent.clear(screen.getByLabelText("Path"));
    await userEvent.type(screen.getByLabelText("Path"), "raw/data");
    expect(save).toBeEnabled();
  });

  it("sends the parent id when adding under a parent", async () => {
    const onSubmit = vi.fn();
    renderDrawer({ onSubmit });
    await userEvent.type(screen.getByLabelText("Path"), "data");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({ path: "raw/data", parent: "n0" }),
    );
  });

  it("sends a null parent when adding at the root", async () => {
    const onSubmit = vi.fn();
    renderDrawer({ parent: null, onSubmit });
    await userEvent.type(screen.getByLabelText("Path"), "raw");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({ path: "raw", parent: null }),
    );
  });

  it("shows the node's current values when editing", () => {
    renderDrawer({
      mode: "edit",
      node: {
        nodeId: "n1",
        description: "raw data",
        path: "raw/data",
        directory: true,
        parent: "n0",
        reference: null,
        migration: null,
        children: [],
      },
      parent: null,
    });
    expect(screen.getByLabelText("Path")).toHaveValue("raw/data");
    expect(screen.getByLabelText("Description")).toHaveValue("raw data");
  });
});
```

- [ ] **Step 2: Write the failing delete-dialog test**

Create `apps/desktop/aegis-desktop/src/test/features/templates/delete-node-dialog.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureNodeView } from "../../../shared/api";
import { DeleteNodeDialog } from "../../../features/templates/components/DeleteNodeDialog";

afterEach(() => cleanup());

const DIRECTORY: StructureNodeView = {
  nodeId: "n1",
  description: "",
  path: "raw/data",
  directory: true,
  parent: "n0",
  reference: null,
  migration: null,
  children: [],
};

function renderDialog(
  props: Partial<ComponentProps<typeof DeleteNodeDialog>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DeleteNodeDialog
          open
          node={DIRECTORY}
          onClose={() => {}}
          onConfirm={() => {}}
          pending={false}
          error={null}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("DeleteNodeDialog", () => {
  it("names the node and warns that its subtree goes too", () => {
    renderDialog();
    expect(
      screen.getByText(/Delete "raw\/data" and everything under it\?/),
    ).toBeInTheDocument();
  });

  it("confirms with the node", async () => {
    const onConfirm = vi.fn();
    renderDialog({ onConfirm });
    await userEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(onConfirm).toHaveBeenCalledWith(DIRECTORY);
  });

  it("shows the error when the delete fails", () => {
    renderDialog({
      error: { kind: "http", status: 403, code: "forbidden", message: "nope" },
    });
    expect(screen.getByText(/nope/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/node-drawer.test.tsx src/test/features/templates/delete-node-dialog.test.tsx`
Expected: FAIL — the components do not exist.

- [ ] **Step 4: Implement the drawer**

Create `apps/desktop/aegis-desktop/src/features/templates/components/NodeDrawer.tsx`:

```tsx
import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Drawer,
  FormControlLabel,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  MigrationView,
  StructureNodeInput,
  StructureNodeView,
} from "../../../shared/api";

export interface NodeDrawerProps {
  open: boolean;
  mode: "create" | "edit";
  structureId: number;
  /** The node being edited. `null` in create mode. */
  node: StructureNodeView | null;
  /** The parent the new node goes under. `null` means the root. */
  parent: StructureNodeView | null;
  onClose: () => void;
  onSubmit: (input: StructureNodeInput) => void;
  pending: boolean;
  error: ApiError | null;
}

/**
 * A relative path is well formed when it is non-blank, uses `/` as its
 * only separator, and has no empty, `.` or `..` segment — the same rule
 * the server's `validate_path` enforces.
 */
export function isValidNodePath(path: string): boolean {
  const trimmed = path.trim();
  if (trimmed === "") return false;
  if (trimmed.startsWith("/") || trimmed.endsWith("/")) return false;
  if (trimmed.includes("\\")) return false;
  return trimmed
    .split("/")
    .every((seg) => seg !== "" && seg !== "." && seg !== "..");
}

export function NodeDrawer({
  open,
  mode,
  structureId,
  node,
  parent,
  onClose,
  onSubmit,
  pending,
  error,
}: NodeDrawerProps) {
  const { t } = useI18n();

  const [path, setPath] = useState("");
  const [description, setDescription] = useState("");
  const [directory, setDirectory] = useState(true);
  const [reference, setReference] = useState("");
  const [migrationDirectory, setMigrationDirectory] = useState("");
  const [migrationFilters, setMigrationFilters] = useState<string[]>([]);

  useEffect(() => {
    if (!open) return;
    if (mode === "create") {
      setPath(parent ? `${parent.path}/` : "");
      setDescription("");
      setDirectory(true);
      setReference("");
      setMigrationDirectory("");
      setMigrationFilters([]);
      return;
    }
    setPath(node?.path ?? "");
    setDescription(node?.description ?? "");
    setDirectory(node?.directory ?? true);
    setReference(node?.reference ?? "");
    setMigrationDirectory(node?.migration?.directory ?? "");
    setMigrationFilters(node?.migration?.filter ?? []);
  }, [open, mode, node, parent]);

  const pathValid = isValidNodePath(path);

  function handleSubmit() {
    if (!pathValid) return;
    const migration: MigrationView | null = directory
      ? migrationDirectory.trim() === ""
        ? null
        : { directory: migrationDirectory.trim(), filter: migrationFilters }
      : null;
    onSubmit({
      path: path.trim(),
      description,
      directory,
      parent: mode === "create" ? (parent?.nodeId ?? null) : (node?.parent ?? null),
      reference: directory ? null : reference.trim() || null,
      migration,
    });
  }

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {mode === "create"
            ? t("templates.node.drawer.addTitle")
            : t("templates.node.drawer.editTitle")}
        </Typography>

        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("templates.node.field.path")}
            value={path}
            onChange={(e) => setPath(e.target.value)}
            required
            error={path !== "" && !pathValid}
            helperText={
              path !== "" && !pathValid
                ? t("templates.node.field.invalidPath")
                : undefined
            }
          />
          <TextField
            size="small"
            label={t("templates.node.field.description")}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
          <FormControlLabel
            control={
              <Switch
                checked={directory}
                onChange={(e) => setDirectory(e.target.checked)}
              />
            }
            label={t("templates.node.field.directory")}
          />
          {!directory && (
            <TextField
              size="small"
              label={t("templates.node.field.reference")}
              value={reference}
              onChange={(e) => setReference(e.target.value)}
            />
          )}
          {directory && (
            <>
              <TextField
                size="small"
                label={t("templates.node.field.migrationDirectory")}
                value={migrationDirectory}
                onChange={(e) => setMigrationDirectory(e.target.value)}
              />
              {migrationFilters.map((filter, index) => (
                <TextField
                  key={index}
                  size="small"
                  label={t("templates.node.field.migrationFilter")}
                  value={filter}
                  onChange={(e) =>
                    setMigrationFilters((filters) =>
                      filters.map((f, i) =>
                        i === index ? e.target.value : f,
                      ),
                    )
                  }
                />
              ))}
              <Button
                size="small"
                sx={{ alignSelf: "flex-start" }}
                onClick={() =>
                  setMigrationFilters((filters) => [...filters, ""])
                }
              >
                {t("templates.node.field.migrationFilter")}
              </Button>
            </>
          )}
        </Stack>

        {error && (
          <Alert severity="error">
            {t("templates.node.saveFailed", { message: errorMessage(error) })}
          </Alert>
        )}

        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClose} disabled={pending}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="contained"
            onClick={handleSubmit}
            disabled={!pathValid || pending}
          >
            {mode === "create" ? t("common.create") : t("common.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 5: Implement the delete dialog**

Create `apps/desktop/aegis-desktop/src/features/templates/components/DeleteNodeDialog.tsx`:

```tsx
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { StructureNodeView } from "../../../shared/api";

export interface DeleteNodeDialogProps {
  open: boolean;
  node: StructureNodeView | null;
  onClose: () => void;
  onConfirm: (node: StructureNodeView) => void;
  pending: boolean;
  error: unknown;
}

export function DeleteNodeDialog({
  open,
  node,
  onClose,
  onConfirm,
  pending,
  error,
}: DeleteNodeDialogProps) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>{t("templates.node.delete.title")}</DialogTitle>
      <DialogContent>
        <DialogContentText>
          {node
            ? t("templates.node.delete.message", { path: node.path })
            : t("templates.node.delete.title")}
        </DialogContentText>
        {Boolean(error) && (
          <DialogContentText sx={{ mt: 2, color: "error.main" }}>
            {t("templates.node.delete.failed", {
              message: errorMessage(error),
            })}
          </DialogContentText>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={pending}>
          {t("common.cancel")}
        </Button>
        <Button
          color="error"
          onClick={() => node && onConfirm(node)}
          disabled={pending || !node}
        >
          {t("common.confirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/node-drawer.test.tsx src/test/features/templates/delete-node-dialog.test.tsx`
Expected: PASS — 12 tests.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/templates/components/NodeDrawer.tsx apps/desktop/aegis-desktop/src/features/templates/components/DeleteNodeDialog.tsx apps/desktop/aegis-desktop/src/test/features/templates/node-drawer.test.tsx apps/desktop/aegis-desktop/src/test/features/templates/delete-node-dialog.test.tsx
git commit -m "feat(desktop): add the structure node drawer and delete dialog"
```

---

### Task 11: The Project Structure Template page

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/templates/components/StructureSidebar.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/templates/components/StructureTree.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/templates/pages/StructureTemplatePage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/templates/index.ts`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates/structure/index.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates/structure/$structureId.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/templates/structure-template-page.test.tsx`

**Interfaces:**
- Consumes: the hooks from Task 7, the row from Task 9, the drawer and dialog from Task 10, `flattenStructure` from Task 6, `useCurrentUser` from `src/features/auth`, and the i18n keys from Task 5.
- Produces: `StructureTemplatePage({ structureId }: { structureId: number | null })`, exported from `src/features/templates/index.ts`. The two route files render it — the index route with `structureId={null}`, the param route with the parsed id.

- [ ] **Step 1: Write the failing page test**

Create `apps/desktop/aegis-desktop/src/test/features/templates/structure-template-page.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";
import { StructureTemplatePage } from "../../../features/templates";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

const STRUCTURE = {
  id: 3,
  name: "Standard",
  level: "template",
  projectCode: null,
  templateId: null,
  nodes: [
    {
      nodeId: "d1",
      description: "raw data",
      path: "raw",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
      children: [
        {
          nodeId: "d2",
          description: "",
          path: "raw/data",
          directory: true,
          parent: "d1",
          reference: null,
          migration: null,
          children: [
            {
              nodeId: "f1",
              description: "",
              path: "raw/data/a.sas7bdat",
              directory: false,
              parent: "d2",
              reference: "/abs/a.sas7bdat",
              migration: null,
              children: [],
            },
          ],
        },
      ],
    },
  ],
};

const TEMPLATES = {
  structures: [
    {
      id: 3,
      name: "Standard",
      level: "template",
      projectCode: null,
      templateId: null,
    },
  ],
};

function baseCommands(extra: Record<string, unknown> = {}) {
  return {
    is_logged_in: () => true,
    current_user: () => ({
      id: 1,
      code: "alice",
      name: "Alice",
      role: "general",
      active: true,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    }),
    list_structures: () => TEMPLATES,
    get_structure: () => STRUCTURE,
    ...extra,
  };
}

beforeEach(() => {
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderPage(path = "/templates/structure/3") {
  return renderWithFullRouter({
    initialEntries: [path],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    }),
  });
}

describe("StructureTemplatePage", () => {
  it("lists the template structures in the sidebar", async () => {
    mockCommands(baseCommands());
    await renderPage();
    expect(await screen.findByText("Standard")).toBeInTheDocument();
  });

  it("renders the tree of the selected structure", async () => {
    mockCommands(baseCommands());
    await renderPage();
    expect(await screen.findByText("raw")).toBeInTheDocument();
    expect(await screen.findByText("raw/data")).toBeInTheDocument();
  });

  it("switches to the flat view, where a sub-directory is not nested", async () => {
    mockCommands(baseCommands());
    await renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Flat" }));
    // The flat view shows the directory by its own name, plus its full path.
    await waitFor(() => expect(screen.getByText("data")).toBeInTheDocument());
    expect(screen.getByText("raw/data")).toBeInTheDocument();
  });

  it("shows an empty state when the structure has no nodes", async () => {
    mockCommands(
      baseCommands({
        get_structure: () => ({ ...STRUCTURE, nodes: [] }),
      }),
    );
    await renderPage();
    expect(
      await screen.findByText("No folder structure yet"),
    ).toBeInTheDocument();
  });

  it("shows an empty state and does not select anything when there are no templates", async () => {
    mockCommands(
      baseCommands({
        list_structures: () => ({ structures: [] }),
      }),
    );
    await renderPage("/templates/structure");
    expect(await screen.findByText("No templates yet")).toBeInTheDocument();
  });

  it("hides the node actions from a general user", async () => {
    mockCommands(baseCommands());
    await renderPage();
    expect(
      await screen.findByText("raw"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /add node/i }),
    ).not.toBeInTheDocument();
  });

  it("shows the node actions to an admin", async () => {
    mockCommands(
      baseCommands({
        current_user: () => ({
          id: 1,
          code: "alice",
          name: "Alice",
          role: "admin",
          active: true,
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-01T00:00:00Z",
        }),
      }),
    );
    await renderPage();
    expect(
      await screen.findByRole("button", { name: /add node/i }),
    ).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/structure-template-page.test.tsx`
Expected: FAIL — `StructureTemplatePage` is not exported.

- [ ] **Step 3: Implement the sidebar**

Create `apps/desktop/aegis-desktop/src/features/templates/components/StructureSidebar.tsx`:

```tsx
import { Box, List, ListItemButton, ListItemText, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { StructureSummaryView } from "../../../shared/api";

export interface StructureSidebarProps {
  structures: StructureSummaryView[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  loading: boolean;
}

/** The template-level structures, listed down the right-hand side. */
export function StructureSidebar({
  structures,
  selectedId,
  onSelect,
  loading,
}: StructureSidebarProps) {
  const { t } = useI18n();

  return (
    <Box
      sx={{
        width: 260,
        flexShrink: 0,
        borderLeft: 1,
        borderColor: "divider",
        overflowY: "auto",
        p: 2,
      }}
    >
      <Typography variant="subtitle2" sx={{ mb: 1 }}>
        {t("templates.structure.heading")}
      </Typography>

      {loading && <Typography variant="body2">…</Typography>}

      {!loading && structures.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          {t("templates.structure.noTemplates")}
        </Typography>
      )}

      <List disablePadding>
        {structures.map((structure) => (
          <ListItemButton
            key={structure.id}
            selected={structure.id === selectedId}
            onClick={() => onSelect(structure.id)}
          >
            <ListItemText primary={structure.name} />
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
}
```

- [ ] **Step 4: Implement the tree pane**

Create `apps/desktop/aegis-desktop/src/features/templates/components/StructureTree.tsx`:

```tsx
import { useState } from "react";
import { Box, Button, Stack, ToggleButton, ToggleButtonGroup, Typography } from "@aegis/ui/mui";
import { Add as AddIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { SimpleTreeView, TreeItem } from "@aegis/ui/tree-view";

import type { StructureNodeView } from "../../../shared/api";
import { flattenStructure } from "../data/tree";
import { StructureNodeRow } from "./StructureNodeRow";

export type StructureTreeViewMode = "complete" | "flat";

export interface StructureTreeProps {
  nodes: StructureNodeView[];
  canMutate: boolean;
  onAdd: (node: StructureNodeView | null) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
}

/** The Complete tree — the API's tree, nested by `parent`. */
function CompleteTree(props: {
  nodes: StructureNodeView[];
  canMutate: boolean;
  onAdd: (node: StructureNodeView) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
}) {
  const renderNodes = (nodes: StructureNodeView[]) =>
    nodes.map((node) => (
      <TreeItem
        key={node.nodeId}
        itemId={node.nodeId}
        label={
          <StructureNodeRow
            node={node}
            canMutate={props.canMutate}
            onAdd={props.onAdd}
            onEdit={props.onEdit}
            onDelete={props.onDelete}
          />
        }
      >
        {node.children.length > 0 ? renderNodes(node.children) : null}
      </TreeItem>
    ));

  return <SimpleTreeView>{renderNodes(props.nodes)}</SimpleTreeView>;
}

/** The Flat tree — one row per directory, with its files beneath it. */
function FlatTree(props: {
  nodes: StructureNodeView[];
  canMutate: boolean;
  onAdd: (node: StructureNodeView) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
}) {
  const entries = flattenStructure(props.nodes);

  return (
    <SimpleTreeView>
      {entries.map((entry) => (
        <TreeItem
          key={entry.node.nodeId}
          itemId={entry.node.nodeId}
          label={
            <StructureNodeRow
              node={entry.node}
              canMutate={props.canMutate}
              onAdd={props.onAdd}
              onEdit={props.onEdit}
              onDelete={props.onDelete}
              label={entry.name}
              secondaryPath={entry.fullPath}
            />
          }
        >
          {entry.files.map((file) => (
            <TreeItem
              key={file.nodeId}
              itemId={file.nodeId}
              label={
                <StructureNodeRow
                  node={file}
                  canMutate={props.canMutate}
                  onAdd={props.onAdd}
                  onEdit={props.onEdit}
                  onDelete={props.onDelete}
                />
              }
            />
          ))}
        </TreeItem>
      ))}
    </SimpleTreeView>
  );
}

export function StructureTree({
  nodes,
  canMutate,
  onAdd,
  onEdit,
  onDelete,
}: StructureTreeProps) {
  const { t } = useI18n();
  const [mode, setMode] = useState<StructureTreeViewMode>("complete");

  const shared = { nodes, canMutate, onAdd, onEdit, onDelete };

  return (
    <Box sx={{ flexGrow: 1, minWidth: 0, p: 3, overflow: "auto" }}>
      <Stack direction="row" spacing={2} alignItems="center" sx={{ mb: 2 }}>
        <ToggleButtonGroup
          size="small"
          exclusive
          value={mode}
          onChange={(_e, next) => {
            if (next) setMode(next as StructureTreeViewMode);
          }}
        >
          <ToggleButton value="complete">
            {t("templates.structure.view.complete")}
          </ToggleButton>
          <ToggleButton value="flat">
            {t("templates.structure.view.flat")}
          </ToggleButton>
        </ToggleButtonGroup>

        {canMutate && (
          <Button
            size="small"
            startIcon={<AddIcon />}
            onClick={() => onAdd(null)}
          >
            {t("templates.node.addRoot")}
          </Button>
        )}
      </Stack>

      {nodes.length === 0 ? (
        <Typography variant="body2" color="text.secondary">
          {t("templates.structure.noNodes")}
        </Typography>
      ) : mode === "complete" ? (
        <CompleteTree {...shared} />
      ) : (
        <FlatTree {...shared} />
      )}
    </Box>
  );
}
```

- [ ] **Step 5: Implement the page**

Create `apps/desktop/aegis-desktop/src/features/templates/pages/StructureTemplatePage.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser } from "../../auth";
import type { StructureNodeInput, StructureNodeView } from "../../../shared/api";
import {
  useAddStructureNode,
  useDeleteStructureNode,
  useGetStructure,
  useListTemplateStructures,
  useUpdateStructureNode,
} from "../data/structures";
import { DeleteNodeDialog } from "../components/DeleteNodeDialog";
import { NodeDrawer } from "../components/NodeDrawer";
import { StructureSidebar } from "../components/StructureSidebar";
import { StructureTree } from "../components/StructureTree";

export interface StructureTemplatePageProps {
  /** `null` on the index route — the first template is selected then. */
  structureId: number | null;
}

export function StructureTemplatePage({
  structureId,
}: StructureTemplatePageProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();

  const structures = useListTemplateStructures();
  const structure = useGetStructure(structureId);

  const addNode = useAddStructureNode();
  const updateNode = useUpdateStructureNode();
  const deleteNode = useDeleteStructureNode();

  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<"create" | "edit">("create");
  const [drawerNode, setDrawerNode] = useState<StructureNodeView | null>(null);
  const [drawerParent, setDrawerParent] = useState<StructureNodeView | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<StructureNodeView | null>(null);

  const canMutate =
    currentUser.data?.role === "root" || currentUser.data?.role === "admin";

  // With no structure selected, settle on the first template so the URL
  // becomes a shareable link.
  useEffect(() => {
    if (structureId != null) return;
    const first = structures.data?.[0];
    if (!first) return;
    navigate({
      to: "/templates/structure/$structureId",
      params: { structureId: String(first.id) },
      replace: true,
    });
  }, [structureId, structures.data, navigate]);

  function openAdd(parent: StructureNodeView | null) {
    setDrawerMode("create");
    setDrawerNode(null);
    setDrawerParent(parent);
    setDrawerOpen(true);
  }

  function openEdit(node: StructureNodeView) {
    setDrawerMode("edit");
    setDrawerNode(node);
    setDrawerParent(null);
    setDrawerOpen(true);
  }

  function handleSubmit(input: StructureNodeInput) {
    if (structureId == null) return;
    // The drawer stays open until the write succeeds, so a failure shows
    // the server's message in the drawer instead of vanishing.
    const onSuccess = () => setDrawerOpen(false);
    if (drawerMode === "create") {
      addNode.mutate({ structureId, input }, { onSuccess });
    } else if (drawerNode) {
      updateNode.mutate(
        { structureId, nodeId: drawerNode.nodeId, input },
        { onSuccess },
      );
    }
  }

  function handleDelete(node: StructureNodeView) {
    if (structureId == null) return;
    deleteNode.mutate(
      { structureId, nodeId: node.nodeId },
      { onSuccess: () => setDeleteTarget(null) },
    );
  }

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Box sx={{ flexGrow: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
        <Typography variant="h5" sx={{ p: 3, pb: 0 }}>
          {structure.data?.name ?? t("templates.structure.heading")}
        </Typography>
        <StructureTree
          nodes={structure.data?.nodes ?? []}
          canMutate={canMutate}
          onAdd={openAdd}
          onEdit={openEdit}
          onDelete={setDeleteTarget}
        />
      </Box>

      <StructureSidebar
        structures={structures.data ?? []}
        selectedId={structureId}
        onSelect={(id) =>
          navigate({ to: "/templates/structure/$structureId", params: { structureId: String(id) } })
        }
        loading={structures.isLoading}
      />

      <NodeDrawer
        open={drawerOpen}
        mode={drawerMode}
        structureId={structureId ?? 0}
        node={drawerNode}
        parent={drawerParent}
        onClose={() => setDrawerOpen(false)}
        onSubmit={handleSubmit}
        pending={addNode.isPending || updateNode.isPending}
        error={addNode.error ?? updateNode.error}
      />

      <DeleteNodeDialog
        open={deleteTarget != null}
        node={deleteTarget}
        onClose={() => setDeleteTarget(null)}
        onConfirm={handleDelete}
        pending={deleteNode.isPending}
        error={deleteNode.error}
      />
    </Box>
  );
}
```

- [ ] **Step 6: Export the page**

In `apps/desktop/aegis-desktop/src/features/templates/index.ts`, add:

```ts
export { StructureTemplatePage } from "./pages/StructureTemplatePage";
export { TemplatesPage } from "./pages/TemplatesPage";
```

- [ ] **Step 7: Add the two routes**

Create `apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates/structure/index.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { StructureTemplatePage } from "../../../../../features/templates";

export const Route = createFileRoute("/_authed/_layout/templates/structure/")({
  component: () => <StructureTemplatePage structureId={null} />,
});
```

Create `apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates/structure/$structureId.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { StructureTemplatePage } from "../../../../../features/templates";

export const Route = createFileRoute(
  "/_authed/_layout/templates/structure/$structureId",
)({
  component: () => (
    <StructureTemplatePage
      structureId={Number(Route.useParams().structureId)}
    />
  ),
});
```

- [ ] **Step 8: Regenerate the route tree**

Run: `pnpm --filter aegis-desktop exec vite build`
Expected: `src/routes/routeTree.gen.ts` gains the two structure routes.

- [ ] **Step 9: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/structure-template-page.test.tsx`
Expected: PASS — 7 tests.

If the sidebar test cannot find "Standard", check that the page's `useGetStructure` is enabled: the test renders with `structureId = 3` via the route, so `get_structure` must be mocked with the `get_structure` key — it is.

- [ ] **Step 10: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 11: Run the whole frontend suite**

Run: `pnpm --filter aegis-desktop test`
Expected: PASS — no regression in the existing tests.

- [ ] **Step 12: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/templates apps/desktop/aegis-desktop/src/routes/_authed/_layout/templates apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts apps/desktop/aegis-desktop/src/test/features/templates/structure-template-page.test.tsx
git commit -m "feat(desktop): add the project structure template page"
```

---

## Final Verification

Run all four gates:

```bash
cargo test -p aegis-desktop --lib
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
pnpm --filter @aegis/ui typecheck
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

All five must pass before the branch is ready for review.
