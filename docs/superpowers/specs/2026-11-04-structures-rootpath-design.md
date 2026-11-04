# Structures `rootpath` Design

> **Scope.** Add a `rootpath` field to the `Structure` aggregate in
> `lib/crates/structures`, and carry it through the `structures` crate, the
> `apis` crate, and the `aegis-server` HTTP layer. **The desktop crate
> (`apps/desktop/aegis-desktop`) is not touched.**

## Goal

A structure describes a **directory layout** — the set of folders and files a
project should have on disk. The layout is recorded as a tree of nodes, each
carrying its own name; the nesting comes from the `parent` link. What the
model does not record today is **where that tree is rooted**.

`rootpath` is the **absolute path of the directory the structure's tree is
rooted at**. A node's full path is then:

```text
<rootpath> / <ancestor entry_names…> / <entry_name>
```

The crate still performs **no filesystem I/O**. `rootpath` is a recorded
string, exactly like the two absolute paths the model already carries:
`Node.reference` (absolute file path) and `Migration.directory` (absolute
folder path). The desktop app remains responsible for materialising the tree
on disk.

## Terminology

- **rootpath** — the absolute base directory of a structure's tree. Every
  structure has one, at every level.
- **template / project structure** — as today (see the
  [structures crate design](2026-11-02-structures-crate-design.md)). The
  `rootpath` rule does not distinguish between them: a template's rootpath is
  the base directory its layout is authored against, a project structure's is
  the base directory it is materialised into.

## Decisions

| Question | Decision | Why |
| --- | --- | --- |
| What is `rootpath`? | The absolute base directory of the structure's tree. | The tree's nodes are relative; the root is what makes them absolute. It matches the model's existing absolute paths (`reference`, `migration.directory`). |
| Required? | **Yes, at both levels.** Every structure carries one. | A structure without a base directory cannot be materialised. `Option` would push the "is it set?" question onto every reader. |
| Validation | **Non-blank only.** A blank `rootpath` is rejected. | Deliberately lax, matching how `Node.reference` and `Migration.directory` are recorded without being checked for shape. `Structure::new` rejects a blank `name` the same way. |
| Mutability | **Mutable**, via the existing `PUT /api/structures/{id}` endpoint. | No new endpoint. A structure's root can move; the endpoint that already mutates the structure gains one more mutable field. |
| Create request | **`rootpath` is required** on the wire. | A structure cannot exist without its base directory. |
| Rename request | **`rootpath` is optional**; absent means *unchanged*. | Keeps the rename request a partial update and keeps the desktop's existing rename call (which sends only `name`) working. |
| Spelling | **`rootpath`** in Rust and on the wire. | As specified. `rootpath` is a single word, so the wire's camelCase rename leaves it as `rootpath`. |
| What the merged view reports | A project structure's merged tree reports **the project structure's own** rootpath. The template's rootpath is not surfaced. | Each structure records the base directory *it* is rooted at. A node inherited from the template keeps the absolute path it was authored with, as recorded data — the crate performs no filesystem I/O and re-bases nothing. Surfacing the template's rootpath would be a second field with no consumer in this change. |
| Blank root path on the wire | `400` with the generic `validation_failed` code. | `DomainError::EmptyRootpath` → `UsecaseError::Validation` → `StructureApiError::Validation` → `ErrorBody { code: "validation_failed", message }`, exactly as a blank `name` already does. Clients dispatch on `code`, so a dedicated `empty_rootpath` code would be a new contract for one field; this change does not add one. |

## Shape of the change

### `lib/crates/structures` (the aggregate)

- `domain/structure.rs`
  - `Structure` gains `pub rootpath: String` — placed after `name`.
  - `Structure::new(name, rootpath, level, project_code, template_id)` rejects a
    blank `rootpath` with the new `DomainError::EmptyRootpath`.
  - `Structure::for_repository(id, name, rootpath, level, project_code,
    template_id, nodes)` carries it.
  - `StructureNew` gains `rootpath: String`.
- `domain/error.rs` — new variant `EmptyRootpath` ("empty rootpath").
- `domain/repository.rs` — the outbound port's `rename` becomes
  `update(&self, id, name, rootpath: Option<&str>)`. `None` leaves the stored
  root path alone.
- `usecase/commands.rs` — `RenameStructure` becomes
  `UpdateStructure { id, name, rootpath: Option<String> }`; `CreateStructure`
  gains `rootpath: String`.
- `usecase/views.rs` — `StructureView`, `StructureSummaryView` and
  `StructureMetaView` each gain `rootpath: String`.
- `usecase/structure_usecase.rs` — `create_structure` threads the root path
  through validation and persistence; `rename_structure` becomes
  `update_structure`, which validates a supplied root path (blank → rejected)
  and passes `None` straight through when the caller did not supply one.
- `adapter/persistence/postgres/structure_repo.rs` — `StructureRow` gains
  `rootpath: String`; every `SELECT`/`INSERT`/`RETURNING` list gains the
  column. The update statement becomes
  `SET name = $2, rootpath = COALESCE($3, rootpath)`, so an absent root path
  keeps the stored one.
- `migrations/20261104065624_add_structures_rootpath.sql` — new migration,
  created by the sqlx CLI (per the crate guideline, never hand-named):

  ```sql
  ALTER TABLE structures
      ADD COLUMN rootpath TEXT NOT NULL DEFAULT '';
  ALTER TABLE structures
      ALTER COLUMN rootpath DROP DEFAULT;
  ```

  The `DEFAULT ''` exists only so the `ADD COLUMN` succeeds against a table
  that already holds rows; dropping it immediately afterwards means every
  **new** row must supply a root path. Rows created before this change carry
  an empty root path and should be given one through the update endpoint.
  This mirrors `name`: `NOT NULL` in the database, non-blank enforced by the
  domain. No `CHECK` constraint is added, because it would reject the
  back-filled empty values.

### `lib/crates/apis` (the port contract)

- `StructureView`, `StructureSummaryView`, `StructureMetaView` gain
  `rootpath: String`.
- `CreateStructureRequest` gains `rootpath: String` — **required**.
- `RenameStructureRequest` gains `rootpath: Option<String>`.
- `StructureService::rename_structure` keeps its name and its request type.

### `apps/server/aegis-server` (the wire)

- `transport/http/dto.rs` — `StructureViewResponse`, `StructureSummaryResponse`
  and `StructureMetaResponse` gain `rootpath: String`; `CreateStructureRequest`
  gains `rootpath: String`; `RenameStructureRequest` gains
  `rootpath: Option<String>`. The `camelCase` serde rename leaves the
  single-word field as `rootpath` on the wire.
- `transport/http/structures/handlers.rs` — the create handler passes the root
  path through; the rename handler forwards the optional root path.
- The route (`PUT /api/structures/{id}`), the operation id
  (`structures_rename_structure`) and the OpenAPI schema name
  (`RenameStructureRequest`) are **unchanged** — the desktop's Rust wire DTO
  mirrors these names, and the desktop is out of scope.

## Naming: what changes and what does not

Names change only where the old name would become a lie, and only where no
external contract pins them.

| Symbol | Before | After | Pinned by an external contract? |
| --- | --- | --- | --- |
| `StructureRepository::rename` | `rename(id, name)` | `update(id, name, rootpath)` | No — crate-internal port. |
| `RenameStructure` (usecase command) | — | `UpdateStructure` | No — crate-internal. |
| `StructureUsecase::rename_structure` | — | `update_structure` | No — crate-internal. |
| `StructureService::rename_structure` (apis) | — | unchanged | Mapped 1:1 onto the HTTP route. |
| `dto::RenameStructureRequest` (server) | — | unchanged | Mirrored by the desktop's Rust DTO. |
| `PUT /api/structures/{id}` | — | unchanged | The desktop's route. |

The facade (`structures::adapter::facade::internal`) is the translation
boundary: it maps the apis-layer `RenameStructureRequest` onto the usecase's
`UpdateStructure`.

## Known consequences

1. **The desktop's create-structure call breaks.** The desktop's
   `CreateStructureRequest` carries no `rootpath`, and the server now
   requires one — the request is rejected by the deserializer (HTTP 422)
   before it reaches the handler. This is accepted for now: the desktop crate
   is explicitly out of scope, and the field is required on purpose. The
   desktop picks this up in a later change, where three places need the
   field: the Rust wire DTO in `src-tauri/src/http/structures.rs`, the
   TypeScript mirror in `src/shared/api/types.ts`, and the templates page's
   create form.
2. **The desktop's reads keep working.** The desktop's Rust wire DTOs do not
   use `deny_unknown_fields`, so the new `rootpath` key in a response body is
   simply ignored. The desktop's rename call keeps working too — it sends
   only `name`, and an absent `rootpath` means *unchanged*.
3. **The 422 is axum's default**, not the server's `ErrorBody` envelope. That
   is pre-existing behaviour for any request missing a required field, and
   fixing it is out of scope.
4. **Structures created before this change** have an empty root path until an
   operator sets one through the update endpoint.
5. **The migration must be applied before the new binary serves.** The server
   does not auto-migrate, so the `rootpath` column has to exist in the
   server's database before a binary that selects it is deployed — otherwise
   every structures query fails with "column rootpath does not exist". The
   order is: apply
   `sqlx migrate run --source lib/crates/structures/migrations`, then deploy.
   The column is `NOT NULL`, so the migration itself is what backfills the
   rows that predate it; there is no separate data step to order against the
   deploy.

## Tests

- `structures/src/domain/tests.rs` — `Structure::new` calls carry a root path;
  a new test rejects a blank one.
- `structures/src/usecase/tests.rs` — the fake repository gains the widened
  `update`; the update path is covered (a supplied root path is stored, an
  absent one leaves the stored value alone).
- `structures/src/adapter/facade/internal/tests.rs` — the facade's
  create/update requests carry a root path.
- `structures/tests/public_api.rs` — the fake repository implements the
  widened port method.
- `structures/tests/integration_persistence.rs` — the create command carries a
  root path; the live round-trip asserts it survives the database.
- `structures/src/adapter/persistence/postgres/structure_repo.rs` — the
  migration-file assertion covers the new column (per the crate guideline's
  "read the migration as a string" rule).
- `aegis-server/src/transport/http/structures/handlers.rs` — the mock service's
  views carry a root path; the route test's create bodies include one.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy -p structures -p apis -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p structures -p apis -p aegis-server
cargo check --workspace
cargo test -p structures -- --ignored --test-threads=1   # needs AEGIS_STRUCTURES_DATABASE_URL
```
