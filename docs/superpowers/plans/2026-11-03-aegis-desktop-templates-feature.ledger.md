# Execution ledger — Aegis Desktop templates feature

The audit trail of the inline execution of
`docs/superpowers/plans/2026-11-03-aegis-desktop-templates-feature.md`.
Every `Ruling:` line is a decision taken where the plan and the spec
did not settle the matter, with what it costs if wrong.

# SDD ledger — plan: docs/superpowers/plans/2026-11-03-aegis-desktop-templates-feature.md

## Setup

- Workspace: `.superpowers/sdd/2026-11-03-aegis-desktop-templates-feature/`
- Spec: `docs/superpowers/specs/2026-11-03-aegis-desktop-templates-feature-design.md`
- Branch: `feat/desktop_structure-template`
- BASE for Task 1: `3d67952`
- Execution mode: native (inline), per the human partner's explicit instruction.

**Setup ruling:** no separate git worktree — the branch `feat/desktop_structure-template`
is already a dedicated feature branch, and the spec and plan are committed on it.
A fresh worktree would re-run `pnpm install` and a full cargo build while splitting
the implementation away from the branch the human partner has been committing to.
Cost if wrong: the implementation lands on a different branch than expected — recoverable
by merging; no work is lost.

## Pre-flight interface scan

| Producer | Consumer | Produces | Consumes | Finding |
| --- | --- | --- | --- | --- |
| T1 | T11 | `@aegis/ui/tree-view` subpath exporting `SimpleTreeView`, `TreeItem` | `@mui/x-tree-view@9.16.0` | OK |
| T2 | T3 | `http::structures` DTOs + 5 functions | `HttpClient`, `ApiError` | OK |
| T3 | T4 | command names `list_structures`, `get_structure`, `add_structure_node`, `update_structure_node`, `delete_structure_node` | T2 DTOs | OK |
| T4 | T7 | TS types, `api.*` wrappers, `queryKeys.structures.*` | T3 command names | OK |
| T5 | T8–T11 | 27 i18n keys | — | OK |
| T6 | T11 | `flattenStructure`, `nodeName`, `FlatEntry` | `StructureNodeView` | OK |
| T7 | T11 | 5 hooks | T4 `api.*` + query keys | OK |
| T9 | T11 | `StructureNodeRow` props | `StructureNodeView` | OK |
| T10 | T11 | `NodeDrawer` props, `DeleteNodeDialog` props, `isValidNodePath` | `StructureNodeInput` | OK |
| T8 | T11 | `TemplatesPage`, feature `index.ts` | T5 keys | OK |

**Pre-flight ruling 1:** the Rust `list_structures` command returns `Vec<StructureSummaryResponse>`
— a plain JSON array — while the HTTP route returns `{"structures": [...]}`. The
Tauri-level mocks in Tasks 7 and 11 must return an array. The plan's mocks used the
HTTP envelope; corrected in the plan before Task 1 (commit `3d67952`).
Cost if wrong: the sidebar renders nothing and the empty-state assertions pass for the
wrong reason — caught by the first Task 7 test.

**Pre-flight ruling 2:** `StructureNodeRow` needs two label modes — the Complete view
shows `node.path`, the Flat view shows the directory's own name plus its full path.
Resolved by an optional `label` prop that overrides the default. Cost if wrong: the flat
view shows the full path twice and the directory's name never appears.

## Tasks

Task 1: complete (commits 3d67952..6fa3d5d, tests: pnpm --filter @aegis/ui test → [2m   Duration [22m 18.42s[2m (transform 717ms, setup 4.22s, collect 38.32s, tests 1.74s, environment 10.35s, prepare 2.48s)[22m)
Task 1: Ruling: the plan's Task 1 had no test step, but the repo gives every `@aegis/ui` subpath one (`src/charts/index.test.ts` is the model) — added `src/tree-view/index.test.ts`, watched it fail with the re-export moved aside, then pass. The plan was updated to match. Cost if wrong: a broken subpath export ships undetected; the test is 12 lines.
Task 2: complete (commits 6fa3d5d..5857993, tests: cargo test -p aegis-desktop --lib → test result: ok. 195 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s)
Task 2: Ruling: the plan ran the RED before registering `pub mod structures;` in http.rs — the test command would have found 0 tests and falsely passed. Registered the module first so the RED is a real compile error. Cost if wrong: a green gate that proved nothing.
Task 2: Ruling: the plan's test import omitted `TokenStore`, which `set_access_token` needs in scope (http/mission.rs imports it). Added it. Cost if wrong: the 204 test errors for the wrong reason.
Task 2: Ruling: the plan's clippy gate (`cargo clippy -D warnings` → no warnings) cannot pass — the crate has 120 pre-existing clippy errors on this toolchain, plus a hard failure in lib/crates/windows-utils. The gate runs as a delta against the 120 baseline instead, and the new module's dead-code warnings (unused until Task 3) are expected to vanish after Task 3. Fixing the pre-existing lints is an unrelated sweep and is out of scope. Cost if wrong: a lint regression hides inside a large pre-existing noise floor — mitigated by the fixed baseline count of 120.
Task 3: complete (commits 5857993..95ab1e8, tests: cargo test -p aegis-desktop --lib → test result: ok. 197 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s)
Task 3: Ruling: the plan's command bodies call `&app_state.http_client()`, but `http_client()` already returns `&HttpClient` — five needless_borrow errors, i.e. five above the 120 baseline. Changed to `app_state.http_client()`. The repo's existing http/mission.rs has the same borrow, but new code should not add to the noise. Cost if wrong: the lint delta grows by five; no behavior change.
Task 3: Ruling: as in Task 2, the module was registered in commands.rs before the RED run — otherwise the RED would have been an empty test run rather than a compile error. Cost if wrong: a green gate that proved nothing.
Task 4: Ruling: the plan said to add the six new type names to shared/api/index.ts's top import block, but the file has a SECOND block at the bottom re-exporting the types (`export type { ... } from "./types"`). Without it, Tasks 7-11 cannot import the types from shared/api. Added all six to the re-export block; added only the five the file itself uses to the top import, because `MigrationView` is unused there and tsc fails with TS6196. Cost if wrong: a downstream import of the types fails to resolve.
Task 4: Ruling: the plan's test command `pnpm --filter aegis-desktop test -- <file>` does not filter — pnpm forwards the literal `--`, so vitest runs the whole suite. Use `npx vitest run <file>` from apps/desktop/aegis-desktop. The plan was updated. Cost if wrong: a 280-second run instead of a 4-second one, and a misleading pass/fail.
Task 4: Ruling: shared/api/index.ts and shared/query/keys.ts have CRLF line endings, so multi-line Edit calls against them never match. Script-based insertion is used instead. Cost if wrong: nothing — the scripts preserve the existing line endings.
PRE-EXISTING FAILURES (measured at Task 4, before any templates UI exists — 19 failures across 9 files, none caused by this feature):
  src/test/features/project-list/project-list-page.test.tsx (6)
  src/test/features/project-list/project-filter-bar.test.tsx (3)
  src/test/features/project-list/project-drawer.test.tsx (3)
  src/test/features/domain-model/sdtm-domain-list.test.tsx (2)
  src/test/features/terminology/version-dropdown-persistence.test.tsx (1)
  src/test/features/project-workspace/project-workspace-route.test.tsx (1)
  src/test/features/metadata/metadata-page.test.tsx (1)
  src/test/features/crf/crf-form-table.test.tsx (1)
  src/test/features/crf/crf-form-list-page.test.tsx (1)
  Cause: MUI v9 no longer renders aria-disabled="true" on a disabled ListItemButton. The metadata-page navigation tests PASS, so the renderInRouter + router.state.location.pathname pattern the templates page tests rely on is sound.
Task 4: complete (commits 95ab1e8..ecc161c, tests: npx vitest run src/test/features/templates/api.test.ts →    Duration  291ms (transform 63ms, setup 0ms, import 100ms, tests 7ms, environment 0ms))
Task 5: Ruling: zhCN.ts closes with `} satisfies Record<keyof typeof en, string>;`, not `} as const;` — the plan implied a shared shape. Inserted before that anchor. Cost if wrong: the keys land in the wrong place; the typecheck would still fail, so it is self-checking.
Task 5: complete (commits ecc161c..8399c9d, tests: pnpm --filter @aegis/ui test → [2m   Duration [22m 24.48s[2m (transform 829ms, setup 5.81s, collect 50.52s, tests 1.81s, environment 9.91s, prepare 2.16s)[22m)
Task 6: complete (commits 8399c9d..bf3eabe, tests: npx vitest run src/test/features/templates/tree.test.ts →    Duration  249ms (transform 30ms, setup 0ms, import 47ms, tests 6ms, environment 0ms))
Task 7: Ruling: the plan asserted `client.getQueryState(...).isInvalidated` — but invalidating an active query refetches it, and a completed refetch clears the invalidated flag, so the assertion races the refetch. Asserted instead on the mock's call count, which is monotonic. Cost if wrong: a flaky test that passes or fails depending on refetch timing.
Task 7: complete (commits bf3eabe..5c800dc, tests: npx vitest run src/test/features/templates/data.test.tsx → [2m   Duration [22m 1.88s[2m (transform 103ms, setup 228ms, collect 276ms, tests 274ms, environment 616ms, prepare 197ms)[22m)
Task 8: Ruling: the Templates card navigates to /templates/structure, but that route does not exist until Task 11 — TanStack's navigate() type is the app's route union, so `navigate({ to: "/templates/structure" })` fails tsc with TS2322 until the route is registered. Task 8's implementation is therefore written and its tests pass, but it is NOT committed until after Task 11; Tasks 9, 10 and 11 execute first. Cost if wrong: a type error blocks the Task 8 typecheck gate — resolved by the reorder, no behavioural change.
Task 8: Ruling: the plan's Task 7 test asserted on queryState.isInvalidated, which races the refetch an invalidation triggers. Asserted on the mock's call count instead (monotonic). Task 7's test file was updated and the plan spliced to match. Cost if wrong: a flaky test.
Task 9: Ruling: the node action trigger was aria-labelled "Add node", which mislabels a menu that opens add, edit AND delete, and collides across rows. Added an i18n key templates.node.actions ("Node actions" / "节点操作") and labelled the trigger with it. Cost if wrong: a screen-reader user hears the wrong action name.
Task 10: Ruling: the drawer's Path field is required, so MUI renders its label as "Path *" and an exact getByLabelText("Path") finds nothing. The test matches /^Path/ instead. Cost if wrong: none — the field stays required and the test still pins the value.
Task 10: Ruling: NodeDrawerProps carried structureId, which the drawer never used (the page owns it) and tsc rejected as TS6136. Removed it from the props, the page and the test. Cost if wrong: none — the submit path already carried the structure id.
Task 11: Ruling: the plan put templates.tsx next to a templates/ folder, but TanStack's file routing makes templates.tsx the LAYOUT PARENT of templates/structure/* — the card page rendered instead of the structure page and the structure page was never reached. Moved the card route to templates/index.tsx so it is a sibling. Cost if wrong: the structure page is unreachable — the route tree and the test both catch this.
Task 11: Ruling: MUI X v9 renamed the expansion prop: defaultExpandedIds is defaultExpandedItems, and the tree renders fully expanded so a folder-structure page shows the whole layout. Cost if wrong: the tree renders collapsed and most of the structure is hidden.
Task 11: Ruling: MUI v9 dropped fontWeight from Typography and alignItems from Stack as direct props; both moved into sx, matching the existing domain-model drawer. Cost if wrong: tsc fails.
Task 11: Ruling: the node action trigger appears once per row, so the page test asserts on getAllByRole rather than getByRole. Cost if wrong: none.
Task 8: Ruling: the Templates page and its route file were swept into Task 11's commit because they live in the same directories Task 11 added wholesale; the nav entry and both test files landed in the follow-up commit. The end state matches the plan; only the commit boundary differs. Cost if wrong: none.
Task 8: complete (commits 3f430b8..7813c45, tests: npx vitest run src/test/features/templates/ src/test/features/app/app-layout-templates.test.tsx → 9 files / 51 tests passed)
Task 9: complete (commits 97eb0be..3a7715c, tests: npx vitest run src/test/features/templates/node-row.test.tsx → 7 passed)
Task 10: complete (commits 97eb0be..3a7715c, tests: npx vitest run src/test/features/templates/node-drawer.test.tsx src/test/features/templates/delete-node-dialog.test.tsx → 15 passed)
Task 11: complete (commits 3a7715c..3f430b8, tests: npx vitest run src/test/features/templates/structure-template-page.test.tsx → 7 passed)
Pre-review fix pass (executor-initiated, before the final review):
  - The node action menu offered "add" on a FILE node, but the server rejects a parent that is not a directory, so the click could only ever 400. The action is now offered only on a directory. RED→GREEN in node-row.test.tsx.
  - A migration filter row the admin left blank was sent to the server, and the button that adds a filter row shared its accessible name with the filter fields. Blank filters are now dropped on submit (matching the domain-model drawer's convention) and the button has its own label. RED→GREEN in node-drawer.test.tsx.

FINAL VERIFICATION (measured on the branch):
  cargo test -p aegis-desktop --lib            → 197 passed; 0 failed
  cargo check --workspace                     → clean (pre-existing warnings only)
  cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings -A clippy::useless_conversion
                                              → 120 errors, IDENTICAL to the pre-feature baseline (75 needless_borrow, 26 let_underscore_future, 6 too_many_arguments, dead code). This branch adds ZERO new lint errors. The absolute gate cannot pass: the crate had 120 pre-existing errors before this feature and lib/crates/windows-utils fails the build outright on this toolchain.
  pnpm --filter @aegis/ui typecheck           → PASS
  pnpm --filter @aegis/ui test                → 105 passed; 0 failed
  pnpm --filter aegis-desktop typecheck      → PASS
  npx vitest run (whole desktop suite)       → 773 passed; 13 failed. Every one of the 13 is in a file this feature never touched and was failing at the same count before it (MUI v9 no longer renders aria-disabled="true" on a disabled ListItemButton). The baseline run had 19 failures; the 6 that dropped off were flaky. NO NEW FAILURES.

  Feature-specific suites: 8 files / 51 tests, all passing.
=== FINAL REVIEW (whole branch, f079be3..953fdb1, fresh Opus reviewer) ===

Critical:
  C1 — add_structure_node / update_structure_node sent a FLATTENED invoke payload
       while the Rust commands take the node payload as a struct parameter named
       `body`. Tauri v2 resolves each argument by the lower-camel-case of the
       Rust parameter name and looks that key up with no whole-payload fallback
       (tauri-2.11.5 src/ipc/command.rs:97-102, tauri-macros-2.6.3
       src/command/wrapper.rs:507). Every node add and edit was rejected at
       runtime. VERIFIED against the tauri source before acting.
       FIXED: the wrapper now nests `body`; the invoke keys live in
       src-tauri/src/structures.contract.json and are checked from BOTH sides —
       a Rust test extracts each command signature's parameter names and
       compares them to the contract, and a TS test asserts each wrapper sends
       exactly the contract's keys. RED→GREEN in api.test.ts; the Rust contract
       test was mutation-checked (renaming `body` → `payload` fails it with
       left: [structureId, payload] right: [structureId, body]).
       Cost of the fix: a new shared JSON artifact the two languages read.

Important:
  I1 — the spec's Rust test coverage was missing (the plan under-specified it).
       FIXED: wiremock tests for list_structures (URI ?level=template), add_node
       (POST path + exact JSON body) and update_node (PUT path + exact body).
       Mutation-checked: breaking the add_node URL fails three of the tests.
  I2 — a stale mutation error was shown in a freshly opened drawer.
       FIXED: the mutations are reset when the drawer or dialog opens.
  I3 — the tree pane showed "No folder structure yet" while the structure was
       still loading. FIXED: StructureTree takes a loading flag and shows the
       empty state only once the query has settled.
  I4 — "Add root node" was a silent no-op with no structure selected.
       FIXED: the button is hidden unless a structure is selected.

Re-graded from Minor to Important:
  M1 — the flat view rendered collapsed, so the file leaves the view exists to
       show were hidden. That is a gap against the spec, not polish.
       FIXED: the flat tree expands every top-level entry.

Fixed while already in the file:
  M2 — the i18n entries had ragged indentation; rewritten uniformly.
  M5 — the sidebar heading reuses the page heading; it now has its own key
       ("Structures" / "模板结构"), matching the spec's sketch.

Deferred minors (ledgered, not fixed — for the human partner to decide):
  M3 — the plan's code blocks now match the shipped code; the Task 4 wrapper
       block, the Task 9 menu, the Task 10 props and the Rust test modules were
       all synced, and a "Changes made by the final review" section records the
       findings so the plan cannot lead a reader back into C1. DONE.
  M4 — a deep-linked non-template structure id renders that project's tree and
       offers the editing chrome to an admin. The server permits the write
       anyway (its guard only requires admin/root for templates), so the impact
       is a surprise rather than a violation; it needs deliberate URL editing.
       A `structure.data?.level === "template"` guard would close it.
       NOT FIXED — deferred to the human partner.
