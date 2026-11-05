# Aegis Desktop — Project Branch Feature Design

## Goal

Give the desktop app a **Branches** section inside the project configuration
page, and show the project's active branch next to the project code in the
workspace sidebar header.

1. **Branches section** — a table of the project's branches with an
   *active* switch per row. Project leaders additionally get create,
   rename and *initialize project folder* actions.
2. **Active-branch chip** — the workspace sidebar header shows a chip with
   the active branch's name, next to the project code.

The scope is **desktop-only**. The server already ships the three branch
routes this feature uses (see
`docs/superpowers/specs/2026-11-05-project-branch-design.md`), and nothing in
`apps/server` or `lib/crates` changes.

## Scope

| In | Out |
| --- | --- |
| Desktop transport for the three branch endpoints the UI needs | `GET /api/project/{code}/branches/{id}` — no screen fetches a single branch |
| A `branches` section on the project configuration page | Deleting a branch (the server has no DELETE route) |
| An active-branch chip in the workspace sidebar header | Branch membership, working-directory paths, dates |
| A stub *initialize project folder* dialog | A real initialize call (no server endpoint exists) |
| New i18n keys in `en` + `zh-CN` | Any change to the app-layout sidebar |

## Terminology

- **Branch** — a named working line inside one project (`"2026Q1"`).
- **Active branch** — the branch the desktop is currently working on. At
  most one per project; zero is legal. **This is desktop-local state** —
  see "The active branch is local state" below.
- **Leader** — a user listed in the project's `members.leaders` or
  `unblindMembers.leaders`. This is the desktop's existing notion of
  project leader (`useProjectConfiguration`), *not* the server's
  admin/root role.

## Decisions

These were settled with the requester and are the load-bearing parts of
the design.

1. **The active branch is desktop-local state.** The server dropped the
   `active` flag from the branch row entirely (commit `1ab41d2`, migration
   `20261105030000_drop_project_branch_active.sql`): the column, the partial
   unique index, the domain field, the apis DTO and the server wire DTO are
   all gone, and `update_project_branch` is now a pure rename. There is no
   endpoint that marks a branch active and no field anywhere that names the
   active branch. The desktop therefore keeps its own active branch, in its
   own settings store, and the server never sees it.
2. **The switch is enabled for every user; the action column is
   leader-only.** Creating, renaming and initializing are leader-only;
   activating is open to anyone, per the request. Because activation is
   local, this costs no server round trip and no permission.
3. **"Leader" means project membership, not role.** The desktop keeps
   using `useProjectConfiguration`'s `isLeader`. The server gates branch
   *writes* behind admin/root, so a General-role leader's rename comes back
   as `403 forbidden`; the desktop surfaces that error as-is rather than
   hiding the control.
4. **Branches is a section of the configuration page, not a route.** The
   configuration page already picks its right-hand section from in-page
   state (`ConfigurationSidebar`); a new tab between *Members* and
   *Filepath* is the whole change. No new route file.
5. **The header chip is a new optional prop on the shared `Sidebar`.**
   `SidebarProps` gains `titleAdornment?: ReactNode`, rendered next to the
   title. `AppLayout` does not pass it, so the app-layout sidebar renders
   exactly as it does today.
6. **"Initialize project folder" is a UI stub.** The dialog lists the
   template structures, requires a selection, and Confirm closes the
   dialog and shows a success message. There is no server endpoint, so
   Confirm makes no request. The dialog is written so that a real call
   slots in where the message is raised.
7. **The dropdown lists template structures** (`listStructures("template")`)
   — a project folder is seeded from a template.

## The active branch is local state

The desktop's active branch is a **branch id** kept per project in
`settings.bin` — the same on-disk store that already carries the theme and
the locale. Nothing is sent to the server.

One key per project code, holding the id or `null`:

```
activeBranch.DEMO-001 → 4
activeBranch.DEMO-002 → null
```

`settings.bin` lives at the app-config level, so every Tauri window of this
app — the main window and every `project:*` workspace window — reads the
same file.

The scope is **per-installation**, like the theme and the locale already
in that store: the key is not namespaced by user, so on a machine where
two different people sign in, the second one inherits the first one's
active branch for that project. That is accepted — it is the same
trade-off the theme and the locale already make — and it keeps the key
format and the hook signatures simple (`readActiveBranch(projectCode)`,
no user lookup).

The id is stored rather than the name so that renaming the active branch
does not lose the selection. Branches are never deleted (the server has no
DELETE route), so an id stays valid for the life of the branch.

### Reading and writing it

`features/settings/data/persist.ts` is the app's persistence module, so the
store access goes there:

```ts
const activeBranchKey = (projectCode: string) => `activeBranch.${projectCode}`;

export async function readActiveBranch(projectCode: string): Promise<number | null>;
export async function writeActiveBranch(projectCode: string, branchId: number | null): Promise<void>;
```

Both are exported through the settings feature's barrel.

`features/project-workspace/data/project-branches.ts` wraps them in the
usual TanStack Query shape:

| Hook | Shape |
| --- | --- |
| `useActiveBranchId(projectCode)` | `useQuery` on `queryKeys.project.activeBranch(projectCode)`; `queryFn` reads the store. `staleTime: Infinity` — the value only changes when this app writes it. |
| `useSetActiveBranchId(projectCode)` | `useMutation`; writes the store, then `setQueryData`s the query with the new id. |

No cross-window event is needed: the chip and the switch live in the same
workspace window, and the app opens one window per project
(`project:${code}`).

### What this means for the UI

- The switch's checked state is `branch.id === activeBranchId`.
- Flipping the switch calls `useSetActiveBranchId` with the branch id (on)
  or `null` (off). **No server call.**
- Because a project holds exactly one active-branch id, "only one branch
  active at a time" and "all branches deactivated is allowed" both fall out
  of the storage model rather than being enforced by a rule.
- The chip resolves the name from the branches list:
  `branches?.find((b) => b.id === activeBranchId)?.branch`.

## Architecture

```
  React (features/project-workspace)
        │  invoke("list_project_branches", …)
        ▼
  Tauri commands (src-tauri/src/commands/project.rs)   ← 1:1 shim
        │
        ▼
  HTTP client  (src-tauri/src/http/project.rs)          ← wire DTOs
        │  GET/POST/PATCH /api/project/{code}/branches…
        ▼
  aegis-server (unchanged)

  React ──▶ settings.bin   ← the active branch, desktop-local
```

### Where each piece lives

| Layer | File | Notes |
| --- | --- | --- |
| HTTP adapter | `src-tauri/src/http/project.rs` | Branch DTOs + the three calls. |
| Tauri commands | `src-tauri/src/commands/project.rs` | `#[tauri::command]` shims, each with an `_impl` twin, following the file's existing shape. |
| Command registry | `src-tauri/src/lib.rs` | A `// project` block in `generate_handler!`. |
| Wire types (TS) | `src/shared/api/types.ts` | Hand-written mirror of the desktop's Rust DTOs, camelCase. |
| API client (TS) | `src/shared/api/index.ts` | `api.listProjectBranches(…)` etc. |
| Query keys | `src/shared/query/keys.ts` | `project.branches(code)`, `project.activeBranch(code)`. |
| Feature data | `src/features/project-workspace/data/project-branches.ts` | List + create + rename + active-branch hooks. |
| Feature components | `src/features/project-workspace/components/` | `ProjectBranchesSection`, `BranchNameDialog`, `InitializeProjectFolderDialog`. |
| Configuration page | `src/features/project-workspace/pages/ProjectConfigurationPage.tsx` | Renders the new section. |
| Configuration nav | `src/features/project-workspace/components/ConfigurationSidebar.tsx` | A new tab between *Members* and *Filepath*. |
| Workspace layout | `src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx` | Passes the chip as `titleAdornment`. |
| Shared sidebar | `lib/packages/ui/src/components/Sidebar/` | `titleAdornment?: ReactNode` on `SidebarProps`. |
| Settings | `src/features/settings/data/persist.ts` | `readActiveBranch` / `writeActiveBranch`. |
| i18n | `lib/packages/ui/src/i18n/locales/en.ts`, `zhCN.ts` | `en.ts` is the source of truth; `zhCN.ts` must carry every key (the registry test pins the key sets). |

## The transport slice

### `src-tauri/src/http/project.rs`

The server's branch DTOs no longer carry an active flag, so neither does
the desktop's:

```rust
/// Wire-level projection of a project branch. Mirrors the server's
/// `ProjectBranchViewResponse` field-for-field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBranchViewResponse {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Wire-level request body for `POST /api/project/{code}/branches`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectBranchRequest {
    pub branch: String,
}

/// Wire-level request body for `PATCH /api/project/{code}/branches/{id}`.
///
/// An absent `branch` leaves the name alone. The server dropped the
/// active flag from the update body, so the desktop never sends one.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectBranchRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}
```

Three functions, following the file's existing shape:

| Function | Call | Returns |
| --- | --- | --- |
| `list_branches(c, code)` | `GET /api/project/{code}/branches` | `Vec<ProjectBranchViewResponse>` |
| `create_branch(c, code, body)` | `POST /api/project/{code}/branches` | `ProjectBranchViewResponse` |
| `update_branch(c, code, id, body)` | `PATCH /api/project/{code}/branches/{id}` | `ProjectBranchViewResponse` |

The server wraps the list in a `{ "branches": [...] }` envelope. The
desktop **unwraps** it, exactly as `http::project::list` already does for
projects — the TypeScript side sees a bare `ProjectBranchView[]` for
branches and a bare `ProjectView[]` for projects, the same shape in both
cases. The envelope is a private wire struct inside the `http` module.

There is no other reshaping: the desktop's branch DTO is the server's
branch DTO.

### `src-tauri/src/commands/project.rs`

Three commands, each with the `_impl` twin + tracing span + `TRACE_ID`
scope the file already uses:

| Command | Arguments | Returns |
| --- | --- | --- |
| `list_project_branches` | `code` | `Vec<ProjectBranchViewResponse>` |
| `create_project_branch` | `code`, `body` | `ProjectBranchViewResponse` |
| `update_project_branch` | `code`, `id`, `body` | `ProjectBranchViewResponse` |

Tauri resolves each argument by the lower-camel-case of the Rust
parameter name, so the TypeScript wrapper passes `code`, `id` and `body`.

The new commands take `&SharedAppState` and call
`app_state.http_client()`, like their siblings in the same file.

### TypeScript bridge

```ts
listProjectBranches: (code: string): Promise<ProjectBranchView[]> =>
  call<ProjectBranchView[]>("list_project_branches", { code }),
createProjectBranch: (
  code: string,
  body: CreateProjectBranchInput,
): Promise<ProjectBranchView> =>
  call<ProjectBranchView>("create_project_branch", { code, body: { ...body } }),
updateProjectBranch: (
  code: string,
  id: number,
  body: UpdateProjectBranchInput,
): Promise<ProjectBranchView> =>
  call<ProjectBranchView>("update_project_branch", { code, id, body: { ...body } }),
```

```ts
export interface ProjectBranchView {
  id: number;
  projectId: number;
  branch: string;
  createdAt: string;
  updatedAt: string;
}
export interface CreateProjectBranchInput {
  branch: string;
}
export interface UpdateProjectBranchInput {
  branch?: string;
}
```

The field names are camelCase because the server's branch DTOs carry
`#[serde(rename_all = "camelCase")]` — the same convention the existing
`ProjectView` mirror already follows (`createdAt`, `unblindMembers`).

## The feature module

### Data hooks — `data/project-branches.ts`

| Hook | Shape |
| --- | --- |
| `useProjectBranches(projectCode)` | `useQuery` on `queryKeys.project.branches(projectCode)` |
| `useCreateProjectBranch(projectCode)` | `useMutation`; invalidates `queryKeys.project.branches(projectCode)` |
| `useUpdateProjectBranch(projectCode)` | `useMutation`; invalidates `queryKeys.project.branches(projectCode)` |
| `useActiveBranchId(projectCode)` | `useQuery` on `queryKeys.project.activeBranch(projectCode)`; reads the settings store |
| `useSetActiveBranchId(projectCode)` | `useMutation`; writes the settings store and updates the query cache |

The two server mutations invalidate the branch list, so a rename is
reflected in the table and in the chip's name.

### The table — `components/ProjectBranchesSection.tsx`

A MUI `Table` with three columns:

| Column | Header | Cell |
| --- | --- | --- |
| 0 | *(empty)* | A `Switch`. Checked when `branch.id === activeBranchId`. Flipping it writes the local active-branch id — **no server call**. Enabled for every user. |
| 1 | *Branch* (zh-CN: 项目分支) | The branch name. |
| 2 | *Leader-only.* An `IconButton` that opens the create dialog. | *Leader-only.* Two `IconButton`s: rename (opens the name dialog) and initialize project folder (opens the structure dialog). |

The action column is rendered only when `isLeader` is true — the whole
column, header included, so a non-leader sees a two-column table.

An empty project renders an empty-state row with the *No branches yet*
message.

### Dialogs

**`BranchNameDialog`** — one dialog for create and rename, with a `mode:
"create" | "edit"` prop and an optional `row`. It owns the name field
state; the parent owns the open/close boolean and the mutation. The name
is trimmed; an empty name disables Confirm (the server rejects a blank
name with `validation_failed`).

**`InitializeProjectFolderDialog`** — a `Select` of the template
structures, a required selection (Confirm is disabled until one is picked),
and Cancel. Confirm closes the dialog and raises `onInitialized(structure)`;
the page turns that into a success snackbar. The dialog makes **no**
request — there is no server endpoint for it.

The structures come from the existing `useListTemplateStructures` hook in
`features/templates`, exported through that feature's barrel (the barrel
currently exports pages only; it gains the hook). The query key
`queryKeys.structures.list("template")` is shared, so the dropdown costs
no extra request when the Templates feature has already loaded it.

### Page and nav wiring

`ProjectConfigurationPage` gains a `branches` case that renders
`<ProjectBranchesSection projectCode={projectCode} isLeader={isLeader === true} />`.

`ConfigurationSidebar` gains a tab between *Members* and *Filepath*:

```ts
export type ConfigurationSection = "general" | "members" | "branches" | "filepath";

const ORDER: SectionEntry[] = [
  { key: "general", icon: Settings },
  { key: "members", icon: People },
  { key: "branches", icon: AccountTree },
  { key: "filepath", icon: Folder },
];
```

### The header chip

`ProjectWorkspaceLayout` reads the branches list and the local active
branch, and passes a chip as `titleAdornment`:

```tsx
const { data: branches } = useProjectBranches(projectCode);
const { data: activeBranchId } = useActiveBranchId(projectCode);
const activeBranch = branches?.find((b) => b.id === activeBranchId);

const sidebarProps: SidebarProps = {
  title: projectCode,
  titleAdornment: activeBranch ? (
    <Chip size="small" label={activeBranch.branch} />
  ) : undefined,
  ...
};
```

`Chip` comes from `@aegis/ui/mui`, like every other MUI primitive in the
app.

When the project has no active branch, no chip is rendered.

`SidebarProps` in `lib/packages/ui/src/components/Sidebar/types.ts` gains
`titleAdornment?: ReactNode`, and `Sidebar.tsx` renders it next to the
title `Typography` in the header row. `AppLayout` passes nothing, so the
app-layout sidebar is unchanged.

The layout now issues a branches query on every workspace page (dashboard,
CRF, configuration). It is one cached request per window.

## Error mapping

The desktop surfaces the server's typed error codes verbatim. Only the
create / rename paths talk to the server; the switch and the chip are
local and cannot fail against it.

Every create / rename error renders **inside the dialog that triggered
it**, and the dialog stays open so the message is visible where the
user clicked. That matters because the `403 forbidden` case is common,
not exceptional: the server gates branch writes behind admin/root, so a
project leader who is not an admin gets `403` on every create and
rename. The section's own alert is only for the branch *list* failing
to load.

| Situation | HTTP | wire code | On screen |
| --- | --- | --- | --- |
| Branch name is blank | 400 | `validation_failed` | Alert in the name dialog |
| Name collides in the project | 409 | `duplicate_branch` | Alert in the name dialog |
| Caller is not admin/root | 403 | `forbidden` | Alert in the name dialog |
| Unknown branch / project | 404 | `branch_not_found`, `not_found` | Alert in the name dialog |
| No bearer | 401 | `token_verification_failed` | Alert in the name dialog |
| Driver failure | 500 | `repository_error` | Alert in the name dialog |
| The branch list fails to load | 4xx / 5xx | any | Alert in the section |

The frontend narrows with `errorMessage(error)` from
`src/shared/api/error.ts`; the `kind === "http"` dance is never
re-written inline.

## Testing

**Rust** (`src-tauri/src/http/project.rs`) — serde shape tests and
`wiremock` tests, mirroring the file's existing ones:

- A branch view parses the full server wire shape.
- The update request skips `None` fields, so a rename sends
  `{"branch":"2026Q2"}` and nothing else.
- `list_branches` issues a `GET` to `/api/project/{code}/branches` and
  unwraps the `{ branches: [...] }` envelope into a `Vec`.
- `create_branch` issues a `POST` to `/api/project/{code}/branches` with
  the branch name as the body.
- `update_branch` issues a `PATCH` to `/api/project/{code}/branches/{id}`
  with the rename body.

The **`http` layer is the Rust test seam**, and it is the whole seam.
There is no `HttpClient` trait and no fake to inject — `HttpClient` is a
concrete struct over `reqwest` — so the `http::` functions are exercised
against a real `MockServer` with a `MemoryStore` token store, exactly as
`http/project.rs` already does for projects.

The **command layer is not unit-tested**, and that is deliberate: the new
commands take `&SharedAppState` like their siblings in
`commands/project.rs`, and `SharedAppState::new` requires a `LogGuard`
that comes out of `init_tracing` — which installs a global subscriber and
can therefore only be initialised once per process. `commands/project.rs`
has no `#[cfg(test)]` module today for the same reason. The command
shims' only real content is the argument mapping, and that is pinned from
the TypeScript side instead (below).

**TypeScript** (`src/test/features/project-workspace/`):

- `api.test.ts` — pins the invoke keys of the three new commands
  (`list_project_branches` → `{ code }`, `create_project_branch` →
  `{ code, body }`, `update_project_branch` → `{ code, id, body }`). This
  is the check the Rust source-inspection test in
  `commands/structures.rs` performs, mirrored on the TypeScript side the
  way `src/test/features/templates/api.test.ts` does for the structures
  commands.
- `project-branches-section.test.tsx` — the table lists the branches; the
  switch is checked only for the active branch; flipping it writes the
  local active-branch id and issues **no** `invoke`; a leader sees the
  action column and a non-leader does not; the create and rename dialogs
  open; the rename dialog is pre-filled with the current name.
- `initialize-project-folder-dialog.test.tsx` — the dropdown lists the
  template structures; Confirm is disabled until one is selected; Confirm
  closes the dialog and raises the success message; Cancel closes without
  raising it.
- `project-workspace-layout.test.tsx` — the header chip shows the active
  branch's name, and no chip when there is no active branch. The layout
  now needs a `QueryClient`, so the existing test moves to
  `renderWithQueryClient`.

**TypeScript** (`src/test/features/settings/settings-persist.test.tsx`) —
the existing file already mocks `@tauri-apps/plugin-store`; it gains
cases for `readActiveBranch` / `writeActiveBranch`.

## Calls I made

These were not settled in the conversation; they are my calls, recorded
here so they can be overridden.

- **The active branch id is persisted in `settings.bin`**, one key per
  project code. It survives an app restart and is shared by this app's
  windows. The scope is **per-installation**, not per-user — a second
  user signing in on the same machine inherits the selection, the same
  trade-off the theme and the locale already make.
- **The id is stored, not the name**, so a rename does not lose the
  selection.
- **No cross-window event is emitted** for the active branch: the chip
  and the switch live in the same workspace window, and the app opens one
  window per project.
- **`GET /api/project/{code}/branches/{id}` is not wired.** No screen
  fetches a single branch; the table works off the list. Leaving it out
  keeps the desktop from shipping an unused command.
- **The chip is hidden when there is no active branch** rather than
  showing a "no active branch" placeholder.
- **A stale active-branch id is left alone.** If the stored id matches no
  branch in the project's list, the chip renders nothing and no switch is
  checked; nothing is cleared. The key is scoped to one project code and
  branch ids are never reused, so a stale id can only ever match nothing.
- **The list envelope is unwrapped by the desktop**, so the TypeScript
  side sees `ProjectBranchView[]` for branches and `ProjectView[]` for
  projects — the same shape in both cases.
- **The branch field names are camelCase**, because the server's branch
  DTOs carry `#[serde(rename_all = "camelCase")]`. The general guideline
  that "the actual JSON keys are snake_case" describes the server DTOs
  that do *not* rename; the branch DTOs do, and the existing `ProjectView`
  mirror already follows them.
- **The command layer is not unit-tested.** The `http` layer is the Rust
  seam; the invoke keys are pinned from the TypeScript side.
- **The switch only ever appears in the workspace window** — the branches
  section lives on `/project/<code>/configuration` — so the chip and the
  switch are always in the same window and cannot disagree.
- **The initialize dialog calls nothing.** There is no server endpoint;
  the success message is the whole behaviour until one exists. When it
  becomes real, the call is `POST /api/structures` with `level: "project"`,
  `templateId` and `projectCode`, plus a `rootpath` the desktop picks
  with the existing `PathPicker` — and the desktop then materialises the
  folder tree on disk, which the structures crate deliberately does not do.
- **`titleAdornment` is a `ReactNode` prop** rather than a widening of
  `title` to `ReactNode`, so the app layout's title stays a string and
  the shared component's contract does not change for existing callers.
- **The action column is hidden entirely for non-leaders** — header
  included — so the table keeps two columns for them.

## Verification gate

```bash
# Frontend
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test

# Backend
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```

## Not in this change

- Deleting a branch. The server has no DELETE route.
- A real *initialize project folder* action.
- The `GET` single-branch command.
- Branch membership, working directories, dates.
- Any change to `apps/server` or `lib/crates`.
