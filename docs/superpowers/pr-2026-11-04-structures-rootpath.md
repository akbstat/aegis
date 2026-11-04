feat(structures): add the rootpath to a structure

A structure describes a *directory layout* — the folders and files a
project should have on disk. The layout is recorded as a tree of nodes,
each carrying its own name; the nesting comes from the `parent` link.
What the model did not record was **where that tree is rooted**.

`rootpath` is the absolute base directory the structure's tree is rooted
at. A node's full path is then

    <rootpath> / <ancestor entry_names…> / <entry_name>

The crate still performs no filesystem I/O. `rootpath` is a recorded
string, exactly like the two absolute paths the model already carries:
`Node.reference` (absolute file path) and `Migration.directory`
(absolute folder path).

Every structure has one, at every level — a structure without a base
directory cannot be materialised. It is required on create and optional
on update, where an absent (or `null`) value means *unchanged*.

The column arrives `NOT NULL` with a default so the ALTER succeeds
against a table that already holds rows; the default is dropped
immediately afterwards, so every *new* row must supply a root path:

    ALTER TABLE structures
        ADD COLUMN rootpath TEXT NOT NULL DEFAULT '';

    ALTER TABLE structures
        ALTER COLUMN rootpath DROP DEFAULT;

The blank check lives in the domain (`Structure::new`), not in the
database — the same split the `name` column uses.

The update statement keeps the stored root path when the caller does
not supply one:

    UPDATE structures SET name = $2, rootpath = COALESCE($3, rootpath) WHERE id = $1

The change carries through the domain (`Structure`, `StructureNew`,
`DomainError::EmptyRootpath`), the usecase commands and views, the
`apis` port, the server's wire DTOs (`rootpath`) and the postgres
adapter. The views that carry a structure's identity — `StructureView`,
`StructureSummaryView`, `StructureMetaView` — gain the field; the tree
view does not.

The crate-internal names changed to match what they now do: the
repository port's `rename` becomes `update(id, name, rootpath)`, the
usecase command `RenameStructure` becomes `UpdateStructure`, and
`StructureUsecase::rename_structure` becomes `update_structure`. The
*apis* port method (`rename_structure`) and the wire DTO
(`RenameStructureRequest`) keep their names — they mirror the
`PUT /api/structures/{id}` route, whose operation id and OpenAPI schema
name are unchanged.

**Two things to know before shipping.**

The desktop's create-structure call now gets a **422**. The server
requires a `rootpath` that the desktop does not send, so the request is
rejected by the deserializer before it reaches the handler. The desktop
crate is deliberately untouched in this PR; it needs a follow-up in
three places — the Rust `CreateStructureRequest` in
`src-tauri/src/http/structures.rs`, the TypeScript `CreateStructureInput`
mirror in `src/shared/api/types.ts`, and the create form in
`src/features/templates/components/StructureDrawer.tsx`. The desktop's
*reads* are unaffected (its wire DTOs ignore unknown fields) and its
update call is unaffected (the root path is optional there). **The
server must not ship ahead of that follow-up**, or the desktop's
create-structure feature is dead in the released app.

The server does not auto-migrate, so **the migration has to be applied
before a binary that reads the column is deployed** — otherwise every
structures query fails with "column rootpath does not exist". The
`NOT NULL` constraint is what backfills the rows that predate the
column; those rows carry an empty root path until an operator sets one
through the update endpoint.

Spec: docs/superpowers/specs/2026-11-04-structures-rootpath-design.md
Plan: docs/superpowers/plans/2026-11-04-structures-rootpath.md

Verification:

    cargo fmt -p structures -p apis -p aegis-server -- --check
    cargo clippy -p structures -p apis -p aegis-server \
      --all-targets --all-features -- -D warnings
    cargo test -p structures -p apis
    cargo test -p structures -- --ignored --test-threads=1
    cargo test -p aegis-server --lib structures
    cargo test -p aegis-server --lib error::tests::structures

(`structures` 82 passed · `apis` 11 passed · `public_api` 4 passed ·
live PostgreSQL 10 passed · `aegis-server` structures-scoped 12 passed
· `aegis-server` error-mapping 2 passed.)

Two pre-existing conditions in the repo, unchanged by this PR and
verified as such at the base commit:

- `cargo test -p aegis-server` fails 28 tests on `main` — 10 in
  `dto::tests`, 9 in `auth::handlers::tests`, 7 in
  `auth::user_credential::handlers::tests`, 2 in `router::tests`. The
  server's DTOs carry `#[serde(rename_all = "camelCase")]` while their
  round-trip tests still feed snake_case JSON. The failing-test name
  sets are identical before and after this change.
- `cargo fmt --all -- --check` reports 4 violations, all in
  `apps/desktop/aegis-desktop/src-tauri/src/{commands,http}/structures.rs`
  — files this PR does not touch. The three crates this PR does touch
  are fmt-clean.

Co-Authored-By: Claude Code <noreply@anthropic.com>
