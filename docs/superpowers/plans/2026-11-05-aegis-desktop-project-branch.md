# Aegis Desktop — Project Branch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Branches section to the desktop's project configuration page — a table of the project's branches with an active switch — plus an active-branch chip in the workspace sidebar header.

**Architecture:** The server already exposes three branch routes and no longer models an active branch at all, so the desktop owns that concept: a branch **id** per project persisted in `settings.bin`. The switch writes local state (no server call); create and rename go through a new Tauri transport slice (`http/project.rs` → `commands/project.rs`) that mirrors the server's DTOs field-for-field.

**Tech Stack:** Rust (axum-free HTTP client + `wiremock` tests, serde), TypeScript (React 19, MUI, TanStack Query, TanStack Router, Vitest), `@aegis/ui` (MUI re-export, i18n catalog, shared `Sidebar`).

**Spec:** `docs/superpowers/specs/2026-11-05-aegis-desktop-project-branch-design.md`

## Global Constraints

- **Scope is desktop-only.** Nothing in `apps/server` or `lib/crates` changes. The server's branch API is exactly: `POST /api/project/{code}/branches` (body `{ branch }`), `GET /api/project/{code}/branches` (body `{ branches: [...] }`), `PATCH /api/project/{code}/branches/{id}` (body `{ branch? }`).
- **The branch object has no `active` field.** The server dropped it in commit `1ab41d2`. The desktop's branch DTO must not carry one either.
- **The active branch is desktop-local state.** It lives in `settings.bin` under `activeBranch.<projectCode>` → branch id (`null` = none). It is never sent to the server. Its scope is **per-installation**, not per-user.
- **The switch makes no server call.** It writes the local active-branch id only.
- **"Leader" means project membership** — `project.members.leaders` or `project.unblindMembers.leaders` — via the existing `useProjectConfiguration` hook. It is *not* the admin/root role. The server gates branch *writes* behind admin/root, so a General-role leader's rename returns `403 forbidden`; surface the server's error, do not hide the control.
- **The initialize-project-folder dialog is a stub.** It lists template structures, requires a selection, and Confirm closes it and shows a success message. It makes **no** request.
- **All MUI primitives and icons come from `@aegis/ui/mui` and `@aegis/ui/icons`.** Never import from `@mui/material` directly.
- **All user-visible strings go through `useI18n()` → `t("path.to.key")`.** Keys are added to `lib/packages/ui/src/i18n/locales/en.ts` (the source of truth — `TranslationKey = keyof typeof en`) and `zhCN.ts` (must carry every key; `registry.test.ts` pins the key sets).
- **Cross-feature imports go through the feature barrel** (`features/<name>/index.ts`), never through a nested path.
- **Query keys are built only in `src/shared/query/keys.ts`**, typed `as const`.
- **The app-layout sidebar must not change.** The shared `Sidebar` gains an *optional* prop; `AppLayout` does not pass it.

## Review Focus

Five input classes the spec implies that a careless implementation would get wrong. Each has a test pinned to the task that owns the code.

1. **A whitespace-only branch name.** The server trims and rejects it (`validation_failed`). The create/rename dialog must disable Confirm on a trimmed-empty name rather than sending it. → Task 9.
2. **A duplicate branch name.** The server answers `409 duplicate_branch`. The dialog must show the server's message, not a generic "save failed". → Task 9.
3. **A project with zero branches.** The table renders an empty state, and the chip renders nothing — no crash on an empty list, and no chip for a `null` active id. → Tasks 9 and 11.
4. **A stale active-branch id** (the stored id is not in the project's branch list). No chip, no checked switch, and the stored value is *not* cleared — a project may simply not have loaded yet. → Task 9.
5. **Renaming the branch that is currently active.** The chip's label comes from the branches list, so the rename mutation must invalidate that list or the chip keeps showing the old name. → Task 11.

---

### Task 1: Branch DTOs and HTTP calls in the desktop's `http/project.rs`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs`
- Test: `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` (the existing `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `HttpClient::request<TReq, TResp>(&self, method: reqwest::Method, path: &str, body: Option<&TReq>) -> Result<TResp, ApiError>`.
- Produces (used by Task 2, then by the TypeScript mirror in Task 3):
  - `pub struct ProjectBranchViewResponse { pub id: i32, pub project_id: i32, pub branch: String, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }`
  - `pub struct CreateProjectBranchRequest { pub branch: String }`
  - `pub struct UpdateProjectBranchRequest { pub branch: Option<String> }`
  - `pub async fn list_branches(c: &HttpClient, code: &str) -> Result<Vec<ProjectBranchViewResponse>, ApiError>`
  - `pub async fn create_branch(c: &HttpClient, code: &str, body: CreateProjectBranchRequest) -> Result<ProjectBranchViewResponse, ApiError>`
  - `pub async fn update_branch(c: &HttpClient, code: &str, id: i32, body: UpdateProjectBranchRequest) -> Result<ProjectBranchViewResponse, ApiError>`

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests` in `http/project.rs`:

```rust
#[test]
fn project_branch_view_parses_the_server_wire_shape() {
    let raw = r#"{
        "id": 4,
        "projectId": 9,
        "branch": "2026Q1",
        "createdAt": "2026-01-02T03:04:05Z",
        "updatedAt": "2026-01-02T03:04:05Z"
    }"#;
    let view: ProjectBranchViewResponse = serde_json::from_str(raw).unwrap();
    assert_eq!(view.id, 4);
    assert_eq!(view.project_id, 9);
    assert_eq!(view.branch, "2026Q1");
}

#[test]
fn update_project_branch_request_skips_absent_fields() {
    let body = UpdateProjectBranchRequest { branch: Some("2026Q2".into()) };
    let j = serde_json::to_string(&body).unwrap();
    assert_eq!(j, r#"{"branch":"2026Q2"}"#);
}

#[tokio::test]
async fn list_branches_unwraps_the_envelope() {
    // MockServer on GET /api/project/pr1/branches returning
    // { "branches": [ { id: 4, projectId: 9, branch: "2026Q1", ... } ] }
    // assert list_branches(&client, "pr1").await.unwrap() has len 1 and
    // branches[0].branch == "2026Q1"
}
```

Also add a `wiremock` test for `create_branch` (asserts a `POST` to `/api/project/pr1/branches` with body `{"branch":"2026Q1"}`) and one for `update_branch` (asserts a `PATCH` to `/api/project/pr1/branches/4` with body `{"branch":"2026Q2"}`).

Follow the existing test helpers in that file: `HttpClient::new(server.uri(), store)` with an `Arc<MemoryStore>` holding `AT` / `RT`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-desktop --lib http::project::tests::project_branch`
Expected: FAIL — `cannot find type ProjectBranchViewResponse` (or `cannot find function list_branches`).

- [ ] **Step 3: Implement the DTOs and the three calls**

In `http/project.rs`, next to the existing project DTOs, add the three public structs exactly as listed under *Interfaces*, each with `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` and `#[serde(rename_all = "camelCase")]`. `UpdateProjectBranchRequest` also derives `Default` and carries `#[serde(default, skip_serializing_if = "Option::is_none")]` on `branch`.

Add a private wire envelope for the list response:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectBranchListWire {
    branches: Vec<ProjectBranchViewResponse>,
}
```

Implement the three functions. `list_branches` issues `GET /api/project/{code}/branches`, deserializes into `ProjectBranchListWire`, and returns `wire.branches`. `create_branch` issues `POST /api/project/{code}/branches` with `Some(&body)`. `update_branch` issues `PATCH /api/project/{code}/branches/{id}` with `Some(&body)`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib http::project::tests`
Expected: PASS — all the existing project tests plus the new branch tests.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/project.rs
git commit -m "feat(desktop): add the branch calls to the project http client"
```

---

### Task 2: Branch Tauri commands

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: the three `http::project` functions from Task 1.
- Produces: three `#[tauri::command]`s, registered in `generate_handler!`:
  - `list_project_branches(app_state, code: String) -> Result<Vec<ProjectBranchViewResponse>, ApiError>`
  - `create_project_branch(app_state, code: String, body: CreateProjectBranchRequest) -> Result<ProjectBranchViewResponse, ApiError>`
  - `update_project_branch(app_state, code: String, id: i32, body: UpdateProjectBranchRequest) -> Result<ProjectBranchViewResponse, ApiError>`

- [ ] **Step 1: Write the commands**

Each command follows the shape of its siblings in the same file: a `#[tauri::command]` that builds a `RequestContext { trace_id: app_state.trace_id_generator().client_side(), caller: Caller::User }` and delegates to an `_impl` twin; the twin opens a `tracing::info_span!("command", trace_id = %req_ctx.trace_id, caller = %req_ctx.caller, command = "<name>")`, scopes `TRACE_ID`, calls the `http::` function, logs `success` / `failed`, and returns the result.

The `_impl` twins take `&SharedAppState` and call `app_state.http_client()` — the same as the existing project commands.

- [ ] **Step 2: Register the commands**

In `lib.rs`, add the three names to the `// project` block of `generate_handler!`, next to `commands::project::update_project`.

- [ ] **Step 3: Verify it compiles and the existing suite still passes**

Run: `cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings && cargo test -p aegis-desktop`
Expected: PASS, no new warnings.

The command layer is deliberately **not** unit-tested: `SharedAppState::new` needs a `LogGuard` from `init_tracing`, which installs a global subscriber and can only be initialised once per process. The invoke keys are pinned from the TypeScript side in Task 3.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add the project branch commands"
```

---

### Task 3: TypeScript wire types, API wrappers and query keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/project-workspace/api.test.ts`

**Interfaces:**
- Consumes: the Tauri commands from Task 2 (`list_project_branches`, `create_project_branch`, `update_project_branch`).
- Produces:
  - `src/shared/api/types.ts`: `ProjectBranchView { id: number; projectId: number; branch: string; createdAt: string; updatedAt: string }`, `CreateProjectBranchInput { branch: string }`, `UpdateProjectBranchInput { branch?: string }`.
  - `src/shared/api/index.ts`: `api.listProjectBranches(code: string): Promise<ProjectBranchView[]>`, `api.createProjectBranch(code: string, body: CreateProjectBranchInput): Promise<ProjectBranchView>`, `api.updateProjectBranch(code: string, id: number, body: UpdateProjectBranchInput): Promise<ProjectBranchView>`.
  - `src/shared/query/keys.ts`: `queryKeys.project.branches(code: string)` → `["project", "branches", code]`, `queryKeys.project.activeBranch(code: string)` → `["project", "activeBranch", code]`.

- [ ] **Step 1: Write the failing test**

Create `src/test/features/project-workspace/api.test.ts`. It mocks `@tauri-apps/api/core` with `vi.mock` (hoisted — it cannot come from a helper module) and asserts the invoke keys, which Tauri resolves by the lower-camel-case of each Rust parameter name:

```ts
expect(invoke).toHaveBeenCalledWith("list_project_branches", { code: "pr1" });
expect(invoke).toHaveBeenCalledWith("create_project_branch", { code: "pr1", body: { branch: "2026Q1" } });
expect(invoke).toHaveBeenCalledWith("update_project_branch", { code: "pr1", id: 4, body: { branch: "2026Q2" } });
```

Also assert that `api.listProjectBranches` resolves to the array the mock returns (the envelope is unwrapped in Rust, so TypeScript sees a bare array).

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/api.test.ts`
Expected: FAIL — `api.listProjectBranches is not a function`.

- [ ] **Step 3: Implement the types, wrappers and keys**

In `types.ts`, add the three interfaces next to the other project types, and re-export them from `src/shared/api/index.ts`'s type export block.

In `api/index.ts`, add the three methods to the `// project` group. The `body` payload must be nested under the `body` key — Tauri looks each argument up by name and has no whole-payload fallback, so a flattened body is rejected at runtime with "missing required key body".

In `keys.ts`, add `branches` and `activeBranch` to the `project` group.

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/api.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts apps/desktop/aegis-desktop/src/shared/api/index.ts apps/desktop/aegis-desktop/src/shared/query/keys.ts apps/desktop/aegis-desktop/src/test/features/project-workspace/api.test.ts
git commit -m "feat(desktop): add the project branch api client"
```

---

### Task 4: i18n keys for the branch feature

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: the translation keys listed below, used by Tasks 8–11. `en.ts` is the source of truth for the `TranslationKey` type, so a key used in a `t()` call must exist here or TypeScript fails to compile.

Keys to add to **both** catalogs:

| key | en | zh-CN |
| --- | --- | --- |
| `project.configuration.section.branches` | Branches | 分支 |
| `project.branches.name.label` | Branch | 项目分支 |
| `project.branches.empty` | No branches yet | 暂无分支 |
| `project.branches.create` | Create branch | 新建分支 |
| `project.branches.rename` | Rename branch | 重命名分支 |
| `project.branches.initializeFolder` | Initialize project folder | 初始化项目文件夹 |
| `project.branches.dialog.create.title` | Create branch | 新建分支 |
| `project.branches.dialog.rename.title` | Rename branch | 重命名分支 |
| `project.branches.dialog.name.label` | Branch name | 分支名称 |
| `project.branches.dialog.name.required` | Branch name must not be empty | 分支名称不能为空 |
| `project.branches.initialize.title` | Initialize project folder | 初始化项目文件夹 |
| `project.branches.initialize.structure` | Structure template | 结构模板 |
| `project.branches.initialize.required` | Select a structure template | 请选择结构模板 |
| `project.branches.initialize.success` | Initialize successfully | 初始化成功 |
| `project.branches.loadFailed` | Failed to load branches: {message} | 加载分支失败：{message} |
| `workspace.branch.active` | Active branch | 当前分支 |

`common.cancel` and `common.confirm` already exist — reuse them for the dialog buttons.

- [ ] **Step 1: Add the keys to `en.ts`**

Insert them in the project block, after `project.configuration.filepath.placeholder`.

- [ ] **Step 2: Add the same keys to `zhCN.ts`**

The same keys, same values as the table. `registry.test.ts` asserts the two catalogs have identical key sets.

- [ ] **Step 3: Verify**

Run: `pnpm --filter @aegis/ui test`
Expected: PASS — including `i18n registry › keeps both catalogs on the same key set`.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): add the project branch translations"
```

---

### Task 5: Persist the active branch in the settings store

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/settings/data/persist.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/settings/index.ts`
- Test: `apps/desktop/aegis-desktop/src/test/features/settings/settings-persist.test.tsx`

**Interfaces:**
- Consumes: the `settings.bin` store (already mocked in the existing test file).
- Produces (used by Task 6):
  - `readActiveBranch(projectCode: string): Promise<number | null>`
  - `writeActiveBranch(projectCode: string, branchId: number | null): Promise<void>`
  Both exported from `features/settings/index.ts`.

- [ ] **Step 1: Write the failing tests**

Add to `settings-persist.test.tsx` (the file already mocks `@tauri-apps/plugin-store` with an in-memory `Map`):

```ts
it("writes the active branch under the project key", async () => {
  await writeActiveBranch("DEMO-001", 4);
  expect(store.get("activeBranch.DEMO-001")).toBe(4);
});

it("reads back the active branch", async () => {
  await writeActiveBranch("DEMO-001", 4);
  expect(await readActiveBranch("DEMO-001")).toBe(4);
});

it("reports no active branch when the key is absent", async () => {
  expect(await readActiveBranch("DEMO-404")).toBeNull();
});

it("reports no active branch when the stored value is null", async () => {
  await writeActiveBranch("DEMO-001", null);
  expect(await readActiveBranch("DEMO-001")).toBeNull();
});
```

The last case matters because the store mock's `get` returns `undefined` for a missing key — `readActiveBranch` must map anything that is not a number to `null`, never leak `undefined` into a `number | null` type.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/settings/settings-persist.test.tsx`
Expected: FAIL — `writeActiveBranch is not a function`.

- [ ] **Step 3: Implement the two functions**

In `persist.ts`, next to the existing `getStore()` singleton:

```ts
const activeBranchKey = (projectCode: string) => `activeBranch.${projectCode}`;
```

`readActiveBranch` gets the store, reads the key, and returns the value only if it is a `number`, else `null`.
`writeActiveBranch` gets the store, sets the key, and calls `store.save()`.

Export both from `features/settings/index.ts`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/settings/settings-persist.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/settings/data/persist.ts apps/desktop/aegis-desktop/src/features/settings/index.ts apps/desktop/aegis-desktop/src/test/features/settings/settings-persist.test.tsx
git commit -m "feat(desktop): persist the active branch in the settings store"
```

---

### Task 6: Branch data hooks

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/data/project-branches.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-branches-data.test.ts`

**Interfaces:**
- Consumes: `api.listProjectBranches` / `api.createProjectBranch` / `api.updateProjectBranch` (Task 3), `queryKeys.project.branches` / `queryKeys.project.activeBranch` (Task 3), `readActiveBranch` / `writeActiveBranch` from the `features/settings` barrel (Task 5).
- Produces (used by Tasks 8 and 11):
  - `useProjectBranches(projectCode: string)` → `UseQueryResult<ProjectBranchView[], ApiError>`
  - `useCreateProjectBranch(projectCode: string)` → `UseMutationResult<ProjectBranchView, ApiError, CreateProjectBranchInput>`
  - `useUpdateProjectBranch(projectCode: string)` → `UseMutationResult<ProjectBranchView, ApiError, { id: number; body: UpdateProjectBranchInput }>`
  - `useActiveBranchId(projectCode: string)` → `UseQueryResult<number | null, Error>`
  - `useSetActiveBranchId(projectCode: string)` → `UseMutationResult<void, Error, number | null>`
  All five exported from `features/project-workspace/index.ts`.

- [ ] **Step 1: Write the failing tests**

Create `src/test/features/project-workspace/project-branches-data.test.ts`. Mock `@tauri-apps/api/core`'s `invoke` and `@tauri-apps/plugin-store`'s `load` (the same in-memory `Map` mock the settings test uses), then render the hooks with `renderHook` from `@testing-library/react` inside a `QueryClientProvider`.

Cover:
- `useProjectBranches` calls `invoke("list_project_branches", { code: "pr1" })` and returns the array.
- `useCreateProjectBranch` calls `invoke("create_project_branch", { code: "pr1", body: { branch: "2026Q1" } })` and, on success, invalidates `queryKeys.project.branches("pr1")`.
- `useUpdateProjectBranch` calls `invoke("update_project_branch", { code: "pr1", id: 4, body: { branch: "2026Q2" } })` and invalidates the same key.
- `useActiveBranchId` reads the store's `activeBranch.pr1` key.
- `useSetActiveBranchId` writes `activeBranch.pr1` and updates the cached value.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-branches-data.test.ts`
Expected: FAIL — `useProjectBranches is not a function`.

- [ ] **Step 3: Implement the hooks**

`useProjectBranches` is a `useQuery` on `queryKeys.project.branches(projectCode)`, `queryFn: () => api.listProjectBranches(projectCode)`, `enabled` when the code is non-empty (mirror `useProjectConfiguration`'s guard).

`useCreateProjectBranch` and `useUpdateProjectBranch` are `useMutation`s whose `onSuccess` calls `qc.invalidateQueries({ queryKey: queryKeys.project.branches(projectCode) })`.

`useActiveBranchId` is a `useQuery` on `queryKeys.project.activeBranch(projectCode)`, `queryFn: () => readActiveBranch(projectCode)`, `staleTime: Infinity` — the value only changes when this app writes it.

`useSetActiveBranchId` is a `useMutation` whose `mutationFn` calls `writeActiveBranch(projectCode, branchId)` and whose `onSuccess` calls `qc.setQueryData(queryKeys.project.activeBranch(projectCode), branchId)`.

Import `readActiveBranch` / `writeActiveBranch` from `"../../settings"` (the barrel, not a nested path).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-branches-data.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/data/project-branches.ts apps/desktop/aegis-desktop/src/features/project-workspace/index.ts apps/desktop/aegis-desktop/src/test/features/project-workspace/project-branches-data.test.ts
git commit -m "feat(desktop): add the project branch data hooks"
```

---

### Task 7: `titleAdornment` on the shared `Sidebar`

**Files:**
- Modify: `lib/packages/ui/src/components/Sidebar/types.ts`
- Modify: `lib/packages/ui/src/components/Sidebar/Sidebar.tsx`
- Test: `lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx`

**Interfaces:**
- Consumes: nothing.
- Produces: `SidebarProps.titleAdornment?: ReactNode`, rendered next to the title in the header row. Used by Task 11. `AppLayout` does not pass it, so the app-layout sidebar is unchanged.

- [ ] **Step 1: Write the failing test**

In `Sidebar.test.tsx`, add a case that renders the `Sidebar` with `title: "DEMO-001"` and `titleAdornment: <span>2026Q1</span>`, and asserts both the title and the adornment are in the document. Also assert the adornment is **not** rendered when the sidebar is collapsed, matching the title's own behaviour.

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter @aegis/ui test -- src/components/Sidebar/Sidebar.test.tsx`
Expected: FAIL — the adornment is not rendered.

- [ ] **Step 3: Implement the prop**

In `types.ts`, add `titleAdornment?: ReactNode;` to `SidebarProps`.

In `Sidebar.tsx`, render it inside the existing header `Box`, after the title `Typography`:

```tsx
{open && (
  <Typography variant="h6" sx={{ ml: 1 }} noWrap>
    {title}
  </Typography>
)}
{open && titleAdornment}
```

The adornment renders only when the sidebar is expanded, alongside the title.

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter @aegis/ui test`
Expected: PASS.

- [ ] **Step 5: Verify the app layout is unaffected**

Run: `pnpm --filter aegis-desktop test -- src/test/features/app`
Expected: PASS — the app-layout sidebar renders exactly as before.

- [ ] **Step 6: Commit**

```bash
git add lib/packages/ui/src/components/Sidebar/types.ts lib/packages/ui/src/components/Sidebar/Sidebar.tsx lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx
git commit -m "feat(ui): let the sidebar title carry an adornment"
```

---

### Task 8: The initialize-project-folder dialog

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/InitializeProjectFolderDialog.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/templates/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`

**Interfaces:**
- Consumes: `useListTemplateStructures` from `features/templates/data/structures.ts`, re-exported through the `features/templates` barrel (this task changes that barrel).
- Produces (used by Task 9): `InitializeProjectFolderDialog({ open: boolean; branch?: ProjectBranchView; onClose: () => void; onInitialized: (structureName: string) => void }): JSX.Element` — exported from the feature barrel.

- [ ] **Step 1: Export the hook from the templates barrel**

`features/templates/index.ts` currently exports only the two pages. Add `export { useListTemplateStructures } from "./data/structures";` so the project-workspace feature can import it through the barrel, as the guidelines require.

- [ ] **Step 2: Write the failing tests**

Create `src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`. Mock `@tauri-apps/api/core`'s `invoke` so that `invoke("list_structures", { level: "template" })` resolves with two template structures.

Cover:
- The dropdown lists the template structures by name.
- Confirm is disabled until a structure is selected.
- Confirm calls `onInitialized` with the selected structure's name and calls `onClose`.
- Cancel calls `onClose` and does **not** call `onInitialized`.
- **No** `invoke` other than the structure list is issued — the dialog makes no request.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`
Expected: FAIL — the component does not exist.

- [ ] **Step 4: Implement the dialog**

A MUI `Dialog` holding a `Select` of the template structures (`useListTemplateStructures()` gives `StructureSummaryView[]`; label each option by `name`), a required-selection message (`t("project.branches.initialize.required")`) when Confirm is pressed with nothing selected, and Cancel / Confirm buttons using `t("common.cancel")` and `t("common.confirm")`.

Confirm calls `onInitialized(selected.name)` then `onClose()`. The dialog makes no request — there is no server endpoint for it. When the dialog is given a `branch`, show its name in the dialog body so the user can see which branch they are initializing.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/InitializeProjectFolderDialog.tsx apps/desktop/aegis-desktop/src/features/templates/index.ts apps/desktop/aegis-desktop/src/features/project-workspace/index.ts apps/desktop/aegis-desktop/src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx
git commit -m "feat(desktop): add the initialize project folder dialog"
```

---

### Task 9: The branches table and the branch name dialog

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ProjectBranchesSection.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/BranchNameDialog.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-branches-section.test.tsx`

**Interfaces:**
- Consumes: the five hooks from Task 6, the i18n keys from Task 4, `errorMessage` from `src/shared/api/error.ts`, `InitializeProjectFolderDialog` from Task 8.
- Produces (used by Task 10):
  - `ProjectBranchesSection({ projectCode: string; isLeader: boolean }): JSX.Element` — exported from the feature barrel.
  - `BranchNameDialog({ open: boolean; mode: "create" | "edit"; branch?: ProjectBranchView; pending: boolean; error: ApiError | null; onClose: () => void; onSubmit: (name: string) => void }): JSX.Element`.

- [ ] **Step 1: Write the failing tests**

Create `src/test/features/project-workspace/project-branches-section.test.tsx`. Mock `@tauri-apps/api/core`'s `invoke` and `@tauri-apps/plugin-store`'s `load`. Render `<ProjectBranchesSection projectCode="pr1" isLeader />` inside a `QueryClientProvider` and the i18n provider.

Cover:
- The table lists one row per branch, showing the branch name.
- The switch is checked only for the branch whose id matches the stored `activeBranch.pr1`.
- Flipping the switch on a different branch writes the store's `activeBranch.pr1` to that branch's id and issues **no** `invoke` call.
- Flipping the active branch's switch off writes `null` to the store.
- A **non-leader** (`isLeader={false}`) sees no create button and no rename / initialize buttons.
- A **leader** sees the create button in the third column's header and the rename and initialize buttons in each row.
- A project with **zero branches** renders the empty-state text.
- A **stale** active-branch id (a store id that matches no branch) leaves every switch unchecked and does not throw.
- Opening the rename dialog pre-fills the name field with the current branch name.
- A **whitespace-only** name leaves Confirm disabled; a trimmed non-empty name enables it.
- A **duplicate** name (the mock throws `{ kind: "http", status: 409, code: "duplicate_branch", message: "…" }`) renders that message in the dialog and the dialog stays open.
- A **`403 forbidden`** (the server's admin/root gate — the common case for a project leader who is not an admin) renders in the dialog too, and the dialog stays open.
- Opening the initialize dialog and confirming shows the success snackbar with `t("project.branches.initialize.success")`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-branches-section.test.tsx`
Expected: FAIL — the component does not exist.

- [ ] **Step 3: Implement `BranchNameDialog`**

A MUI `Dialog` with a `TextField` bound to local state, seeded from `branch?.branch` when `mode` is `"edit"` and empty when it is `"create"`. An effect keyed on `[open, mode, branch]` resets the field when the dialog opens or the target changes.

The Confirm button is disabled when the trimmed value is empty or when `pending` is true. Confirm calls `onSubmit(trimmedValue)`.

When `error` is not null, render it in an `Alert severity="error"` inside the dialog, using `errorMessage(error)` — the server's `duplicate_branch` / `forbidden` / `validation_failed` message reaches the user verbatim. The dialog stays open on error, so the message appears where the user clicked; the parent closes it only on success.

- [ ] **Step 4: Implement `ProjectBranchesSection`**

A MUI `Table` with three columns:

- Column 0: an empty header cell; each row's cell holds a `Switch` checked when `branch.id === activeBranchId`, whose `onChange` calls `setActiveBranchId.mutate(checked ? branch.id : null)` (destructure the mutation from `useSetActiveBranchId`).
- Column 1: a header of `t("project.branches.name.label")`; each cell shows `branch.branch`.
- Column 2: rendered **only** when `isLeader` is true — the header cell holds a create `IconButton`, and each row's cell holds a rename `IconButton` and an initialize `IconButton`.

When the branches list is empty, render the `t("project.branches.empty")` empty state instead of the rows. When the list query errors, render `t("project.branches.loadFailed", { message: errorMessage(error) })` in an `Alert severity="error"`.

The section owns the dialog open/close state, the two mutations (`useCreateProjectBranch`, `useUpdateProjectBranch`), and a `Snackbar` that shows `t("project.branches.initialize.success")` when the initialize dialog reports success. This matches the sibling configuration sections, which own their own mutation hooks.

Give each interactive element a `data-testid` (`project-branches-table`, `branch-switch-<id>`, `branch-create`, `branch-rename-<id>`, `branch-initialize-<id>`) so the tests can target them.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-branches-section.test.tsx`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/ProjectBranchesSection.tsx apps/desktop/aegis-desktop/src/features/project-workspace/components/BranchNameDialog.tsx apps/desktop/aegis-desktop/src/features/project-workspace/index.ts apps/desktop/aegis-desktop/src/test/features/project-workspace/project-branches-section.test.tsx
git commit -m "feat(desktop): add the project branches table"
```

---

### Task 10: Wire the section into the configuration page and the sidebar tab

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectConfigurationPage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationSidebar.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`

**Interfaces:**
- Consumes: `ProjectBranchesSection` (Task 8), the `branches` configuration section key (Task 4).
- Produces: `ConfigurationSection` gains the `"branches"` member; the configuration page renders the section when it is selected.

- [ ] **Step 1: Write the failing tests**

In `project-configuration-page.test.tsx`, add:
- Clicking the `config-section-branches` tab renders the branches table (assert a test id from the section, e.g. `project-branches-table`).
- The tab order is General, Members, Branches, Filepath — assert the four `config-section-*` test ids appear in that DOM order.
- The section renders for a non-leader too (the switch is available to everyone).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-configuration-page.test.tsx`
Expected: FAIL — the `config-section-branches` tab does not exist.

- [ ] **Step 3: Add the tab**

In `ConfigurationSidebar.tsx`, widen the union:

```ts
export type ConfigurationSection = "general" | "members" | "branches" | "filepath";
```

and insert the entry **between** members and filepath in `ORDER`:

```ts
{ key: "branches", icon: AccountTree },
```

Import `AccountTree` from `@aegis/ui/icons`.

- [ ] **Step 4: Render the section**

In `ProjectConfigurationPage.tsx`, add a `branches` case alongside the existing `general` / `members` / `filepath` cases:

```tsx
{section === "branches" && (
  <ProjectBranchesSection
    projectCode={projectCode}
    isLeader={isLeader === true}
  />
)}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-configuration-page.test.tsx`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectConfigurationPage.tsx apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationSidebar.tsx apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx
git commit -m "feat(desktop): add the branches section to the configuration page"
```

---

### Task 11: The active-branch chip in the workspace header

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-workspace-layout.test.tsx`

**Interfaces:**
- Consumes: `useProjectBranches` and `useActiveBranchId` (Task 6), `titleAdornment` (Task 7).
- Produces: the workspace sidebar header shows a `Chip` with the active branch's name.

- [ ] **Step 1: Write the failing tests**

The layout now issues queries, so the existing test file must render inside a `QueryClientProvider` — use the existing `renderWithQueryClient` helper from `src/test/helpers/render-with-query-client.tsx`, and mock `@tauri-apps/api/core`'s `invoke` and `@tauri-apps/plugin-store`'s `load`.

Add:
- With one branch whose id matches the stored `activeBranch.DEMO-001`, the header shows a chip labelled with that branch's name.
- With no active branch (store returns `null`), no chip is rendered.
- With **zero** branches, no chip is rendered and nothing throws.
- With a **stale** active id (no branch matches), no chip is rendered.
- When the branches list returns a renamed branch (same id, new name), the chip shows the new name — the chip's label always comes from the list, never from the stored id.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-workspace-layout.test.tsx`
Expected: FAIL — no chip is rendered.

- [ ] **Step 3: Render the chip**

In `ProjectWorkspaceLayout.tsx`:

```tsx
const { data: branches } = useProjectBranches(projectCode);
const { data: activeBranchId } = useActiveBranchId(projectCode);
const activeBranch = branches?.find((b) => b.id === activeBranchId);
```

and add to `sidebarProps`:

```tsx
titleAdornment: activeBranch ? (
  <Chip size="small" label={activeBranch.branch} />
) : undefined,
```

Import `Chip` from `@aegis/ui/mui`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-workspace-layout.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx apps/desktop/aegis-desktop/src/test/features/project-workspace/project-workspace-layout.test.tsx
git commit -m "feat(desktop): show the active branch in the workspace header"
```

---

### Task 12: Full verification gate

**Files:**
- No file changes expected. If the gate surfaces a failure, fix it in the task that owns the code and amend that task's commit.

**Interfaces:**
- Consumes: everything.
- Produces: a green gate.

- [ ] **Step 1: Run the Rust gate**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```
Expected: no formatting diff, no clippy warnings, all tests pass.

- [ ] **Step 2: Run the frontend gate**

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
```
Expected: all pass.

- [ ] **Step 3: Confirm the working tree is clean**

Run: `git status --short`
Expected: no output (everything committed).

- [ ] **Step 4: Report the result**

Summarise what was built, the commands that were run and their results, and the two known rough edges carried over from the spec:
- a General-role project leader's rename returns `403 forbidden` from the server (the control stays visible and the error is surfaced);
- the initialize-project-folder dialog is a stub that makes no request.
