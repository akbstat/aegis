# Structures `rootpath` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `rootpath` field to the `Structure` aggregate — the absolute base directory of a structure's tree — and carry it through the `structures` crate, the `apis` crate, and the `aegis-server` wire.

**Architecture:** The field is added bottom-up: the domain aggregate and its database column first, then the create path that supplies it, then the update path that can change it. `rootpath` is required on create and optional on update, where an absent value means *unchanged*.

**Tech Stack:** Rust 2024, SQLx (runtime API), thiserror, serde, axum, utoipa.

**Spec:** [`docs/superpowers/specs/2026-11-04-structures-rootpath-design.md`](../specs/2026-11-04-structures-rootpath-design.md)

## Global Constraints

- **The desktop crate is not touched.** `apps/desktop/aegis-desktop/` must not appear in any diff.
- `rootpath` is spelled exactly `rootpath` — in Rust identifiers, in serde field names, and on the wire. It is a single word, so the wire's `camelCase` rename leaves it as `rootpath`.
- `rootpath` is **required** in `CreateStructureRequest` (Rust type `String`) and **optional** in `RenameStructureRequest` (Rust type `Option<String>`; absent *or* explicitly `null` means *unchanged*).
- Validation is **non-blank only**: a root path that is empty or whitespace-only is rejected. It is *not* checked for being an absolute path.
- The wire key on the server is `rootpath`; the `PUT /api/structures/{id}` route, its operation id `structures_rename_structure`, and the OpenAPI schema name `RenameStructureRequest` are **unchanged**.
- Migration files are named by the sqlx CLI, never by hand. The file for this change is `lib/crates/structures/migrations/20261104065624_add_structures_rootpath.sql` and it already exists — do not create another.
- Every error the crates raise is a typed enum: `DomainError` → `UsecaseError` → `StructureApiError` → the server's `ErrorBody { code, message }`. A blank root path therefore answers `400` with the generic code `validation_failed` — the same code a blank `name` produces. No new error code is introduced.
- A merged (effective) tree reports the **requested structure's own** rootpath. The template's rootpath is not surfaced, and an inherited node's recorded absolute path is left as authored — the crate re-bases nothing.
- **Deployment order:** the server does not auto-migrate. Apply `sqlx migrate run --source lib/crates/structures/migrations` to the server's database **before** deploying a binary that reads the new column, or every structures query fails with "column rootpath does not exist".
- Verification gate before each commit:
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p structures -p apis -p aegis-server --all-targets --all-features -- -D warnings
  cargo test -p structures -p apis -p aegis-server
  ```

## Review Focus

Inputs and failure modes the spec implies that a task's own happy-path test does not exercise. Each line has its test in the task that owns the code.

1. **A create request whose `rootpath` is blank or whitespace-only** → `400 validation_failed`, and *nothing* is stored. Not a serde 422, not a stored empty string. → Task 1
2. **A create request that omits `rootpath` entirely** → the request is rejected before it reaches the handler (422), never silently creating a rootless structure. → Task 1
3. **An update request that omits `rootpath`** (or sends `"rootpath": null`) → the stored root path is **unchanged** and the name still changes. → Task 2
4. **An update request whose `rootpath` is blank** → `400 validation_failed`, and the **name is not changed either** — the whole update is rejected, not half-applied. → Task 2
5. **A structure row written before this change** (empty root path in the database) → reads back as `""` without erroring, because the repository constructor bypasses validation. → Task 3

### Hand-off to the desktop (not in this plan)

The desktop crate is out of scope, and its `POST /api/structures` call breaks
until it is updated — the server now requires a `rootpath` the desktop does
not send. When the desktop picks this up, three places need the field:

- `apps/desktop/aegis-desktop/src-tauri/src/http/structures.rs` — the Rust
  `CreateStructureRequest` gains `pub rootpath: String`.
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — the TypeScript
  `CreateStructureInput` mirror gains `rootpath: string`.
- `apps/desktop/aegis-desktop/src/features/templates/components/StructureDrawer.tsx`
  — the create form gains a field for it.

The desktop's *reads* need no change: its wire DTOs do not use
`deny_unknown_fields`, so the new `rootpath` key in a response is ignored.

---

### Task 1: A structure is created with a root path

The field is added to the aggregate, to the database, and to the create path. The update path is Task 2.

**Files:**
- Modify: `lib/crates/structures/src/domain/structure.rs`
- Modify: `lib/crates/structures/src/domain/error.rs`
- Modify: `lib/crates/structures/src/domain/tests.rs`
- Modify: `lib/crates/structures/migrations/20261104065624_add_structures_rootpath.sql`
- Modify: `lib/crates/structures/src/adapter/persistence/postgres/structure_repo.rs`
- Modify: `lib/crates/structures/src/usecase/commands.rs`
- Modify: `lib/crates/structures/src/usecase/views.rs`
- Modify: `lib/crates/structures/src/usecase/structure_usecase.rs`
- Modify: `lib/crates/structures/src/usecase/tests.rs`
- Modify: `lib/crates/structures/src/adapter/facade/internal/service.rs`
- Modify: `lib/crates/structures/src/adapter/facade/internal/tests.rs`
- Modify: `lib/crates/apis/src/structures.rs`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/structures/handlers.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks — this is the first task.
- Produces:
  - `structures::domain::Structure { id: i64, name: String, rootpath: String, level: StructureLevel, project_code: Option<String>, template_id: Option<i64>, nodes: Vec<Node> }`
  - `structures::domain::Structure::new(name: String, rootpath: String, level: StructureLevel, project_code: Option<String>, template_id: Option<i64>) -> Result<Self, DomainError>`
  - `structures::domain::Structure::for_repository(id: i64, name: String, rootpath: String, level: StructureLevel, project_code: Option<String>, template_id: Option<i64>, nodes: Vec<Node>) -> Self`
  - `structures::domain::StructureNew { name: String, rootpath: String, level: StructureLevel, project_code: Option<String>, template_id: Option<i64> }`
  - `structures::domain::DomainError::EmptyRootpath` — message `"empty rootpath"`.
  - `structures::usecase::CreateStructure { name: String, rootpath: String, level: StructureLevel, project_code: Option<String>, template_id: Option<i64> }`
  - `structures::usecase::StructureView` / `StructureSummaryView` / `StructureMetaView` each gain `pub rootpath: String`, placed **after** `name`.
  - `apis::structures::CreateStructureRequest { name: String, rootpath: String, level: StructureLevel, project_code: Option<String>, template_id: Option<i64> }`
  - `apis::structures::StructureView` / `StructureSummaryView` / `StructureMetaView` each gain `pub rootpath: String`, placed **after** `name`.
  - `aegis_server::transport::http::dto::CreateStructureRequest` gains `pub rootpath: String`; `StructureViewResponse` / `StructureSummaryResponse` / `StructureMetaResponse` each gain `pub rootpath: String`.

- [ ] **Step 1: Write the failing domain tests**

In `lib/crates/structures/src/domain/tests.rs`, add:

```rust
#[test]
fn structure_new_rejects_a_blank_rootpath() {
    for blank in ["", "   "] {
        let err = Structure::new(
            "Standard".into(),
            blank.into(),
            StructureLevel::Template,
            None,
            None,
        )
        .unwrap_err();
        assert!(
            matches!(err, DomainError::EmptyRootpath),
            "rootpath {blank:?} should be rejected, got {err:?}"
        );
    }
}

#[test]
fn structure_new_accepts_a_rootpath() {
    let structure = Structure::new(
        "Standard".into(),
        "/abs/root".into(),
        StructureLevel::Template,
        None,
        None,
    )
    .unwrap();
    assert_eq!(structure.rootpath, "/abs/root");
}
```

Also update the **six** existing `Structure::new` call sites in that file — every one gains a root path as the second argument:

- `structure_new_accepts_a_template_with_no_owner` — pass `"/abs/root"`, and assert `structure.rootpath == "/abs/root"`.
- `structure_new_accepts_a_project_with_an_owner_and_a_template` — pass `"/abs/root"`, and assert `structure.rootpath == "/abs/root"`.
- `structure_new_rejects_a_blank_name` — pass `"/abs/root"`; the blank **name** is still what must be reported.
- `structure_new_rejects_a_template_with_an_owner` — pass `"/abs/root"`.
- `structure_new_rejects_a_project_without_a_project_code` — pass `"/abs/root"`.
- `structure_new_rejects_a_project_without_a_template` — pass `"/abs/root"`.

- [ ] **Step 2: Run the domain tests to verify they fail**

Run: `cargo test -p structures --lib domain::tests`
Expected: FAIL to compile — `Structure::new` takes 4 arguments but 5 were supplied, and `DomainError::EmptyRootpath` does not exist.

- [ ] **Step 3: Implement the domain aggregate**

In `lib/crates/structures/src/domain/error.rs`, add the variant to the validation block, next to `EmptyName`:

```rust
    #[error("empty rootpath")]
    EmptyRootpath,
```

In `lib/crates/structures/src/domain/structure.rs`:

- Add `pub rootpath: String` to `Structure`, immediately after `pub name: String`. Document it as the absolute base directory of the structure's tree.
- Change `Structure::new` to take `rootpath: String` as its **second** parameter, and reject a blank one. The check goes **after** the existing blank-name check and **before** the level-shape checks, so the error a caller sees for a doubly-invalid input stays the same one it saw before:

  ```rust
  if name.trim().is_empty() {
      return Err(DomainError::EmptyName);
  }
  if rootpath.trim().is_empty() {
      return Err(DomainError::EmptyRootpath);
  }
  ```

  The new field is assigned into the returned `Self` alongside the others.
- Change `Structure::for_repository` to take `rootpath: String` as its **third** parameter (after `id` and `name`) and assign it.
- Add `pub rootpath: String` to `StructureNew`, immediately after `pub name: String`.

- [ ] **Step 4: Run the domain tests to verify they pass**

Run: `cargo test -p structures --lib domain::tests`
Expected: PASS.

- [ ] **Step 5: Write the failing migration assertion**

In `lib/crates/structures/src/adapter/persistence/postgres/structure_repo.rs`, in the existing `mod tests`, add:

```rust
    #[test]
    fn the_structures_table_gained_a_rootpath_column() {
        let sql = include_str!(
            "../../../../migrations/20261104065624_add_structures_rootpath.sql"
        );
        assert!(sql.contains("ADD COLUMN rootpath TEXT NOT NULL DEFAULT ''"));
        assert!(sql.contains("ALTER COLUMN rootpath DROP DEFAULT"));
    }
```

- [ ] **Step 6: Run the migration assertion to verify it fails**

Run: `cargo test -p structures --lib adapter::persistence::postgres::structure_repo`
Expected: FAIL — the migration file still holds only the sqlx CLI's placeholder comment, so neither `contains` matches.

- [ ] **Step 7: Write the migration DDL**

In `lib/crates/structures/migrations/20261104065624_add_structures_rootpath.sql`, replace the placeholder comment with:

```sql
-- structures.rootpath: the absolute base directory the structure's
-- tree is rooted at.
--
-- The column arrives NOT NULL with a default so the ALTER succeeds
-- against a table that already holds rows; the default is dropped
-- immediately afterwards, so every new row must supply a root path.
-- Rows created before this change carry an empty root path until an
-- operator sets one through the update endpoint.
--
-- The blank check lives in the domain (`Structure::new`), not here --
-- the same split the `name` column uses.

ALTER TABLE structures
    ADD COLUMN rootpath TEXT NOT NULL DEFAULT '';

ALTER TABLE structures
    ALTER COLUMN rootpath DROP DEFAULT;
```

**Deployment order.** The server does not auto-migrate. This column must exist in
the server's database *before* a binary that selects it is deployed, or every
structures query fails with "column rootpath does not exist". The order is:

```bash
sqlx migrate run --source lib/crates/structures/migrations   # against AEGIS_DATABASE_URL
# then deploy the new binary
```

The `NOT NULL` constraint is what backfills the pre-existing rows, so there is
no separate data step to order against the deploy.

- [ ] **Step 8: Run the migration assertion to verify it passes**

Run: `cargo test -p structures --lib adapter::persistence::postgres::structure_repo`
Expected: PASS.

- [ ] **Step 9: Carry the column through the PostgreSQL repository**

In `lib/crates/structures/src/adapter/persistence/postgres/structure_repo.rs`:

- Add `rootpath: String` to `StructureRow`, immediately after `name`.
- In `TryFrom<StructureRow> for Structure`, pass `row.rootpath` to `Structure::for_repository` as the third argument.
- In `create`, use:
  ```sql
  INSERT INTO structures (name, rootpath, level, project_code, template_id)
  VALUES ($1, $2, $3, $4, $5)
  RETURNING id, name, rootpath, level, project_code, template_id
  ```
  and bind `input.rootpath` as `$2`, shifting the existing level/project_code/template_id binds to `$3`/`$4`/`$5`.
- In `find_by_id` and `list`, add `rootpath` to the `SELECT` list, immediately after `name`.
- In `rename` (the port method is still called `rename` in this task), add `rootpath` to the `RETURNING` list:
  ```sql
  UPDATE structures SET name = $2 WHERE id = $1
  RETURNING id, name, rootpath, level, project_code, template_id
  ```

- [ ] **Step 10: Run the adapter tests to verify they pass**

Run: `cargo test -p structures --lib adapter::persistence::postgres`
Expected: PASS.

- [ ] **Step 11: Carry the field through the usecase**

In `lib/crates/structures/src/usecase/commands.rs`, add `pub rootpath: String` to `CreateStructure`, immediately after `pub name: String`.

In `lib/crates/structures/src/usecase/views.rs`, add `pub rootpath: String` to `StructureView`, `StructureSummaryView` and `StructureMetaView`, each immediately after `pub name: String`.

In `lib/crates/structures/src/usecase/structure_usecase.rs`:

- `create_structure`: pass `cmd.rootpath.clone()` as the second argument to the validating `Structure::new` call, and set `rootpath: cmd.rootpath` in the `StructureNew` literal. Copy `structure.rootpath` into the returned `StructureView`.
- `get_structure`: copy `effective.structure.rootpath` into the returned `StructureView`.
- `get_structure_meta`: copy `structure.rootpath` into the returned `StructureMetaView`.
- `list_structures`: copy `s.rootpath` into each `StructureSummaryView`.

- [ ] **Step 12: Update the usecase's in-memory fakes and tests**

In `lib/crates/structures/src/usecase/tests.rs`:

- `FakeStructureRepo::create` calls `Structure::new(input.name, input.rootpath, input.level, input.project_code, input.template_id)`.
- `template_with_child()` calls `Structure::new("Standard".into(), "/abs/standard".into(), StructureLevel::Template, None, None)`.
- Every `CreateStructure { .. }` literal in the file gains `rootpath: "/abs/root".into(),` immediately after `name`.
- `creating_a_template_structure_stores_it_with_no_owner` additionally asserts `view.rootpath == "/abs/root"`.

- [ ] **Step 13: Carry the field through the apis contract and the facade**

> The `structures` crate does not compile between Step 12 and this one: the facade builds a usecase `CreateStructure`, and the usecase command now carries a root path the apis request does not have yet. Finish this step before running any test.

In `lib/crates/apis/src/structures.rs`:

- Add `pub rootpath: String` to `StructureView`, `StructureSummaryView` and `StructureMetaView`, each immediately after `pub name: String`.
- Add `pub rootpath: String` to `CreateStructureRequest`, immediately after `pub name: String`.

In `lib/crates/structures/src/adapter/facade/internal/service.rs`:

- `create_structure`: map `req.rootpath` into the usecase `CreateStructure`.
- The `From<InternalStructureView> for StructureView`, `From<InternalSummaryView> for StructureSummaryView` and `From<InternalMetaView> for StructureMetaView` impls copy `rootpath: v.rootpath`.

In `lib/crates/structures/src/adapter/facade/internal/tests.rs`: every `CreateStructureRequest { .. }` literal gains `rootpath: "/abs/root".into(),` immediately after `name`, and `the_facade_round_trips_a_template_structure` asserts `created.rootpath == "/abs/root"`.

Add a test that a blank root path is refused at the facade boundary (Review Focus #1):

```rust
#[tokio::test]
async fn the_facade_refuses_a_blank_rootpath() {
    let h = harness(&[]);
    let svc = service(&h);
    for blank in ["", "   "] {
        let err = svc
            .create_structure(CreateStructureRequest {
                name: "Standard".into(),
                rootpath: blank.into(),
                level: StructureLevel::Template,
                project_code: None,
                template_id: None,
            })
            .await
            .unwrap_err();
        assert!(
            matches!(err, StructureApiError::Validation(_)),
            "rootpath {blank:?} should be refused, got {err:?}"
        );
    }
    // nothing was stored
    assert_eq!(h.structures.count(), 0);
}
```

- [ ] **Step 14: Run the usecase, facade and apis tests to verify they pass**

Run: `cargo test -p structures -p apis`
Expected: PASS.

- [ ] **Step 15: Carry the field through the server wire**

In `apps/server/aegis-server/src/transport/http/dto.rs`:

- Add `pub rootpath: String` to `StructureViewResponse`, `StructureSummaryResponse` and `StructureMetaResponse`, each immediately after `pub name: String`.
- Add `pub rootpath: String` to `CreateStructureRequest`, immediately after `pub name: String`.
- The three `From<apis::structures::…>` impls copy `rootpath: v.rootpath`.

In `apps/server/aegis-server/src/transport/http/structures/handlers.rs`:

- `create_structure`: add `rootpath: req.rootpath,` to the `apis::structures::CreateStructureRequest` literal.

- [ ] **Step 16: Update the server's handler tests**

In the same file's `mod tests`:

- `structure_view()` gains `rootpath: "/abs/root".into(),` after `name`.
- The `get_structure_meta` and `list_structures` mock responses gain `rootpath: "/abs/root".into(),` after `name`.
- Every request body that creates a structure gains the root path. The bodies to change are:
  - `{"name":"Standard","level":"template"}` → `{"name":"Standard","rootpath":"/abs/root","level":"template"}` — **two** occurrences: the `POST /api/structures` case in `all_routes_dispatch_to_their_handler`, and the body in `creating_a_template_as_a_general_user_returns_403`.
  - `{"name":"Study","level":"project","projectCode":"ABC","templateId":1}` → `{"name":"Study","rootpath":"/abs/root","level":"project","projectCode":"ABC","templateId":1}` — **one** occurrence, in `creating_a_project_structure_as_a_general_user_succeeds`.

  The `PUT /api/structures/1` case in `all_routes_dispatch_to_their_handler` sends `{"name":"Renamed"}` and is **not** changed: the update request's root path is optional (Task 2).

Add a test that a create request omitting the root path is rejected before it reaches the handler (Review Focus #2):

```rust
    #[tokio::test]
    async fn a_create_without_a_rootpath_is_rejected() {
        let state = test_state(MockStructureService::default(), apis::user::Role::Admin);
        let app = app(state);
        let response = app
            .oneshot(post("/api/structures", r#"{"name":"Standard","level":"template"}"#))
            .await
            .unwrap();
        // the body does not deserialize -- axum rejects it before the
        // handler runs
        assert_eq!(response.status(), AxStatus::UNPROCESSABLE_ENTITY);
    }
```

- [ ] **Step 17: Run the server tests to verify they pass**

Run: `cargo test -p aegis-server`
Expected: PASS.

- [ ] **Step 18: Run the full verification gate**

```bash
cargo fmt --all -- --check
cargo clippy -p structures -p apis -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p structures -p apis -p aegis-server
```

Expected: all clean.

- [ ] **Step 19: Confirm the desktop crate is untouched**

Run: `git status --short`
Expected: no path under `apps/desktop/` appears.

- [ ] **Step 20: Commit**

```bash
git add lib/crates/structures lib/crates/apis apps/server/aegis-server
git commit -m "feat(structures): add the rootpath to a structure

The rootpath is the absolute base directory a structure's tree is
rooted at. It is required when a structure is created.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 2: A structure's root path can be changed

The root path becomes mutable through the existing update endpoint. The wire's `PUT /api/structures/{id}` route, its operation id and its schema name are unchanged; the request body gains an optional `rootpath`.

**Files:**
- Modify: `lib/crates/structures/src/domain/repository.rs`
- Modify: `lib/crates/structures/src/adapter/persistence/postgres/structure_repo.rs`
- Modify: `lib/crates/structures/src/usecase/commands.rs`
- Modify: `lib/crates/structures/src/usecase.rs`
- Modify: `lib/crates/structures/src/usecase/structure_usecase.rs`
- Modify: `lib/crates/structures/src/usecase/tests.rs`
- Modify: `lib/crates/structures/src/adapter/facade/internal/service.rs`
- Modify: `lib/crates/structures/src/adapter/facade/internal/tests.rs`
- Modify: `lib/crates/structures/src/lib.rs`
- Modify: `lib/crates/structures/tests/public_api.rs`
- Modify: `lib/crates/apis/src/structures.rs`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/structures/handlers.rs`

**Interfaces:**
- Consumes: everything Task 1 produced.
- Produces:
  - `structures::domain::StructureRepository::update(&self, id: i64, name: &str, rootpath: Option<&str>) -> Result<Structure, DomainError>` — **replaces** `rename(&self, id: i64, name: &str)`. `None` leaves the stored root path alone.
  - `structures::usecase::UpdateStructure { id: i64, name: String, rootpath: Option<String> }` — **replaces** `RenameStructure`.
  - `structures::usecase::StructureUsecase::update_structure(&self, cmd: UpdateStructure) -> Result<StructureView, UsecaseError>` — **replaces** `rename_structure`.
  - `structures::UpdateStructure` is re-exported from the crate root in place of `structures::RenameStructure`.
  - `apis::structures::RenameStructureRequest { id: i64, name: String, rootpath: Option<String> }` — the apis **port method keeps its name** `rename_structure`.
  - `aegis_server::transport::http::dto::RenameStructureRequest { name: String, rootpath: Option<String> }` — the wire DTO keeps its name.

- [ ] **Step 1: Write the failing usecase tests**

In `lib/crates/structures/src/usecase/tests.rs`, replace the two existing rename tests with:

```rust
#[tokio::test]
async fn updating_a_structure_changes_its_name_and_rootpath() {
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
    let updated = h
        .usecase
        .update_structure(UpdateStructure {
            id: project.id,
            name: "Study renamed".into(),
            rootpath: Some("/abs/other".into()),
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "Study renamed");
    assert_eq!(updated.rootpath, "/abs/other");
    assert_eq!(updated.level, StructureLevel::Project);
    assert_eq!(updated.project_code.as_deref(), Some("ABC"));
    assert_eq!(updated.template_id, Some(template.id));
}

#[tokio::test]
async fn updating_a_structure_without_a_rootpath_leaves_it_alone() {
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
    let updated = h
        .usecase
        .update_structure(UpdateStructure {
            id: project.id,
            name: "Study renamed".into(),
            rootpath: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "Study renamed");
    assert_eq!(updated.rootpath, "/abs/root");
}

#[tokio::test]
async fn updating_a_structure_with_a_blank_rootpath_is_rejected() {
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
    let err = h
        .usecase
        .update_structure(UpdateStructure {
            id: project.id,
            name: "Study renamed".into(),
            rootpath: Some("   ".into()),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, UsecaseError::Validation(DomainError::EmptyRootpath)),
        "got {err:?}"
    );
    // the whole update is rejected -- the name is untouched too
    let unchanged = h.usecase.get_structure(project.id).await.unwrap();
    assert_eq!(unchanged.name, "Study");
    assert_eq!(unchanged.rootpath, "/abs/root");
}
```

- [ ] **Step 2: Run the usecase tests to verify they fail**

Run: `cargo test -p structures --lib usecase::tests`
Expected: FAIL to compile — `UpdateStructure` does not exist and `StructureUsecase` has no `update_structure` method.

- [ ] **Step 3: Widen the domain port**

In `lib/crates/structures/src/domain/repository.rs`, replace the `rename` method with:

```rust
    /// Update a structure's name, and optionally its root path.
    ///
    /// A `rootpath` of `None` leaves the stored root path unchanged.
    async fn update(
        &self,
        id: i64,
        name: &str,
        rootpath: Option<&str>,
    ) -> Result<Structure, DomainError>;
```

- [ ] **Step 4: Implement the update in the usecase**

In `lib/crates/structures/src/usecase/commands.rs`, replace `RenameStructure` with:

```rust
/// Update a structure.
///
/// The name is replaced. The root path is optional: `None` leaves the
/// stored root path unchanged.
#[derive(Debug, Clone)]
pub struct UpdateStructure {
    pub id: i64,
    pub name: String,
    /// The new root path. `None` leaves the stored one alone.
    pub rootpath: Option<String>,
}
```

In `lib/crates/structures/src/usecase/structure_usecase.rs`, replace `rename_structure` with `update_structure`:

```rust
    /// Update a structure's name, and optionally its root path.
    ///
    /// Only the name and the root path are mutable. The level, the
    /// owning project and the template pointer are immutable after
    /// creation.
    pub async fn update_structure(
        &self,
        cmd: UpdateStructure,
    ) -> Result<StructureView, UsecaseError> {
        if cmd.name.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyName));
        }
        if let Some(rootpath) = cmd.rootpath.as_deref()
            && rootpath.trim().is_empty()
        {
            return Err(UsecaseError::Validation(DomainError::EmptyRootpath));
        }
        self.structures
            .update(cmd.id, &cmd.name, cmd.rootpath.as_deref())
            .await?;
        self.get_structure(cmd.id).await
    }
```

Update the `use` block at the top of that file: `RenameStructure` → `UpdateStructure`.

In `lib/crates/structures/src/usecase.rs`, change the re-export to
`pub use commands::{AddNode, CreateStructure, DeleteNode, UpdateNode, UpdateStructure};`.

In `lib/crates/structures/src/lib.rs`, change the re-export to
`pub use usecase::{AddNode, CreateStructure, DeleteNode, MigrationView, NodeView, OwnNodeView, StructureMetaView, StructureSummaryView, StructureTreeView, StructureUsecase, StructureUsecaseConfig, StructureView, UpdateNode, UpdateStructure, UsecaseError};`
— `RenameStructure` is removed and `UpdateStructure` takes its place, in alphabetical position.

- [ ] **Step 5: Implement the widened port in the PostgreSQL repository**

In `lib/crates/structures/src/adapter/persistence/postgres/structure_repo.rs`, replace the `rename` implementation with:

```rust
    async fn update(
        &self,
        id: i64,
        name: &str,
        rootpath: Option<&str>,
    ) -> Result<Structure, DomainError> {
        let row = sqlx::query_as::<_, StructureRow>(
            "UPDATE structures SET name = $2, rootpath = COALESCE($3, rootpath) WHERE id = $1
             RETURNING id, name, rootpath, level, project_code, template_id",
        )
        .bind(id)
        .bind(name)
        .bind(rootpath)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?
        .ok_or(DomainError::StructureNotFound(id))?;
        Structure::try_from(row)
    }
```

- [ ] **Step 6: Update the usecase's in-memory fakes**

In `lib/crates/structures/src/usecase/tests.rs`, replace `FakeStructureRepo::rename` with:

```rust
    async fn update(
        &self,
        id: i64,
        name: &str,
        rootpath: Option<&str>,
    ) -> Result<Structure, DomainError> {
        let mut state = self.inner.lock().unwrap();
        let structure = state
            .structures
            .get_mut(&id)
            .ok_or(DomainError::StructureNotFound(id))?;
        structure.name = name.to_string();
        if let Some(rootpath) = rootpath {
            structure.rootpath = rootpath.to_string();
        }
        Ok(structure.clone())
    }
```

- [ ] **Step 7: Update the compile-only public API guard**

In `lib/crates/structures/tests/public_api.rs`:

- Change the import `RenameStructure` → `UpdateStructure`.
- Change `let _: Option<RenameStructure> = None;` → `let _: Option<UpdateStructure> = None;`.
- Change the `FakeStructures` implementation of `rename` to the widened `update`:

```rust
    async fn update(
        &self,
        _id: i64,
        _name: &str,
        _rootpath: Option<&str>,
    ) -> Result<Structure, DomainError> {
        unimplemented!()
    }
```

> This step comes before any test run: `cargo test -p structures` compiles
> the `tests/` directory too, so the guard must already implement the new
> port method or the whole test target fails to build.

- [ ] **Step 8: Widen the apis request and the facade**

In `lib/crates/apis/src/structures.rs`, add to `RenameStructureRequest`:

```rust
    /// The new root path. Absent (or `null`) leaves the stored one
    /// unchanged.
    pub rootpath: Option<String>,
```

The apis port method keeps its name `rename_structure` and its signature — only the request type changed.

In `lib/crates/structures/src/adapter/facade/internal/service.rs`, the `rename_structure` implementation maps the request onto the usecase command:

```rust
            .update_structure(UpdateStructure {
                id: req.id,
                name: req.name,
                rootpath: req.rootpath,
            })
```

Update the file's `use` block: `RenameStructure` → `UpdateStructure` in the `crate::usecase` import.

- [ ] **Step 9: Update the facade tests**

In `lib/crates/structures/src/adapter/facade/internal/tests.rs`, the one `RenameStructureRequest { .. }` literal gains `rootpath: Some("/abs/other".into()),` after `name`. Add an assertion that the renamed structure's root path is the new one.

- [ ] **Step 10: Run the usecase, facade, apis and public API tests to verify they pass**

Run: `cargo test -p structures -p apis`
Expected: PASS — this covers the usecase tests, the facade tests, and the `public_api` guard.

- [ ] **Step 11: Widen the server wire**

In `apps/server/aegis-server/src/transport/http/dto.rs`, add to `RenameStructureRequest`:

```rust
    /// The new root path. Absent (or `null`) leaves the stored one
    /// unchanged.
    pub rootpath: Option<String>,
```

Serde maps a missing field *and* an explicit `null` to `None` for an
`Option<String>`, so both mean *unchanged* — no `#[serde(default)]` is
needed, and the field is never serialized (it is a request body), so
`skip_serializing_if` would be noise.

In `apps/server/aegis-server/src/transport/http/structures/handlers.rs`, the `rename_structure` handler adds `rootpath: req.rootpath,` to the `apis::structures::RenameStructureRequest` literal.

- [ ] **Step 12: Add a server test for the optional root path**

In the same file's `mod tests`, the mock service must record what the handler was asked for. Extend `MockStructureService` with a field that captures the request's root path:

```rust
    #[derive(Clone, Default)]
    pub struct MockStructureService {
        /// The level `get_structure_meta` reports.
        pub level: Option<StructureLevel>,
        /// The root path the last `rename_structure` request carried.
        /// `None` means the request did not carry one.
        pub updated_rootpath: Arc<std::sync::Mutex<Option<Option<String>>>>,
    }
```

Its `rename_structure` implementation records the request:

```rust
        async fn rename_structure(
            &self,
            req: RenameStructureRequest,
        ) -> Result<StructureView, StructureApiError> {
            *self.updated_rootpath.lock().unwrap() = Some(req.rootpath);
            Ok(structure_view())
        }
```

The two existing struct literals — `MockStructureService { level: Some(StructureLevel::Template) }` and `MockStructureService { level: Some(StructureLevel::Project) }` — now need `..Default::default()`:

```rust
            MockStructureService {
                level: Some(StructureLevel::Template),
                ..Default::default()
            },
```

Then add the test:

```rust
    #[tokio::test]
    async fn an_update_without_a_root_path_leaves_the_stored_one_alone() {
        let service = MockStructureService::default();
        let state = test_state(service.clone(), apis::user::Role::Admin);
        let app = app(state);
        let response = app
            .oneshot(put("/api/structures/1", r#"{"name":"Renamed"}"#))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        // the handler forwarded "no root path", not an empty one
        assert_eq!(*service.updated_rootpath.lock().unwrap(), Some(None));
    }
```

Add a second test that an explicit `null` root path also means *unchanged*:

```rust
    #[tokio::test]
    async fn an_update_with_a_null_root_path_leaves_the_stored_one_alone() {
        let service = MockStructureService::default();
        let state = test_state(service.clone(), apis::user::Role::Admin);
        let app = app(state);
        let response = app
            .oneshot(put(
                "/api/structures/1",
                r#"{"name":"Renamed","rootpath":null}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        assert_eq!(*service.updated_rootpath.lock().unwrap(), Some(None));
    }
```

`MockStructureService` derives `Clone`, so cloning it before moving it into `AppState` keeps the assertion possible.

- [ ] **Step 13: Run the server tests to verify they pass**

Run: `cargo test -p aegis-server`
Expected: PASS.

- [ ] **Step 14: Run the full verification gate**

```bash
cargo fmt --all -- --check
cargo clippy -p structures -p apis -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p structures -p apis -p aegis-server
```

Expected: all clean.

- [ ] **Step 15: Confirm no stale references remain**

Run: `grep -rn "RenameStructure\|rename_structure" lib/crates/structures/`
Expected: no output. (`lib/crates/apis`, `apps/server` and the desktop keep the name deliberately — the apis port method and the wire schema mirror the frozen HTTP route.)

- [ ] **Step 16: Commit**

```bash
git add lib/crates/structures lib/crates/apis apps/server/aegis-server
git commit -m "feat(structures): let a structure's root path change

The update endpoint takes an optional root path; absent means the
stored one is kept.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 3: The live database round trip, and the crate's documentation

The root path must survive the database, and a row written before this change must still read back.

**Files:**
- Modify: `lib/crates/structures/tests/integration_persistence.rs`
- Modify: `lib/crates/structures/README.md`

**Interfaces:**
- Consumes: everything Tasks 1 and 2 produced.
- Produces: nothing other tasks depend on.

- [ ] **Step 1: Update the live integration tests**

In `lib/crates/structures/tests/integration_persistence.rs`:

- Every `CreateStructure { .. }` literal gains `rootpath: "/abs/root".into(),` immediately after `name`.
- In `a_template_structure_round_trips_through_postgres`, after the fetch, assert `assert_eq!(fetched.rootpath, "/abs/root");`.

- [ ] **Step 2: Add the live test for a pre-existing row**

Add to the same file:

```rust
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
```

- [ ] **Step 3: Run the live tests**

Run: `cargo test -p structures -- --ignored --test-threads=1`
Expected: PASS, when `AEGIS_STRUCTURES_DATABASE_URL` points at a **dedicated** database. If the variable is unset, the tests panic with a message saying so — that is the expected behaviour, and it means this step cannot be completed on this machine. Say so rather than skipping it silently.

- [ ] **Step 4: Update the crate README**

In `lib/crates/structures/README.md`, add a bullet to the "The model" list, right after the bullet describing a node's `entry_name`:

```markdown
- A structure's **`rootpath`** is the absolute base directory its tree is rooted at. Every structure
  has one, at every level; it is set when the structure is created and can be changed afterwards. A
  node's full path is the root path followed by the chain of ancestor names.
```

The existing bullet (README lines 19–23) says a node's "full path of a node is the chain of names
above it", which now reads as if that chain were the whole path. Amend it so the two agree — the
chain is the path *relative to the root path*:

```markdown
- A node's **`entry_name`** is its own name — one folder or one file name, *not*
  a path. The nesting comes from the `parent` link, and a node's path relative
  to the structure's root path is the chain of names above it. A name is unique
  among the node's **siblings**: the same name may appear under two different
  parents, but never twice under the same one.
```

Also add a sentence to the crate's `src/lib.rs` module doc, which describes the model in prose, so
it says the tree is rooted at the structure's `rootpath`.

- [ ] **Step 5: Run the full verification gate**

```bash
cargo fmt --all -- --check
cargo clippy -p structures -p apis -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p structures -p apis -p aegis-server
```

Expected: all clean.

- [ ] **Step 6: Confirm the desktop crate is still untouched**

Run: `git status --short`
Expected: no path under `apps/desktop/` appears.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/structures
git commit -m "test(structures): round trip the root path through postgres

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```
