# Project Branch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `ProjectBranch` aggregate to the `project` lib crate — a named working branch of a project, of which at most one is active at a time — and expose it through the `apis::project::ProjectService` port and four HTTP routes.

**Architecture:** A new `project_branches` table backs a `ProjectBranch` domain model with its own repository port and its own usecase. The branch methods fold into the existing `apis::project::ProjectService` trait, so the server's `AppState` is unchanged; the crate's facade gains a third generic parameter and holds both usecases. Activation is a *switch*: setting `active = true` deactivates the project's other branches in the same transaction, enforced both in the repository and by a partial unique index.

**Tech Stack:** Rust 2024, SQLx (runtime query API), axum + utoipa-axum, PostgreSQL, `thiserror`, `chrono`, `async_trait`.

**Spec:** `docs/superpowers/specs/2026-11-05-project-branch-design.md` — the plan argues from the spec, so the spec travels with it; executors read both.

## Global Constraints

- The desktop app (`apps/desktop/aegis-desktop`) is **not** touched. No TS types, no API client, no UI.
- `ProjectView` does **not** gain a `branches` field. The project payload is unchanged.
- There is **no DELETE route and no `delete` on the repository port**. Branches are created, read, listed, renamed, and activated/deactivated — never removed.
- Activation is a switch, never a conflict: activating B deactivates A in the same transaction.
- A project may have zero branches. The invariant is "at most one active", not "exactly one".
- The branch name is unique **within its project**; the same name in two different projects is legal.
- The stored branch name is the **trimmed** name; a whitespace-only name is a validation error.
- Branch methods fold into `apis::project::ProjectService`. There is no `apis::project_branch` module.
- Every new migration file is created by the sqlx CLI (`sqlx migrate add --source lib/crates/project/migrations --timestamp <description>`), never hand-named. The CLI is at `/root/.cargo/bin/sqlx` (version 0.9.0).
- Live-DB tests read `AEGIS_DATABASE_URL` (present in the workspace-root `.env`) and drop `project_branches` before each run.
- Commit messages follow the repo's conventional-commit style: `feat(project): …`, `feat(apis): …`, `feat(server): …`, `docs(…): …`. End every commit body with the `Co-Authored-By: Claude Code <noreply@anthropic.com>` line.

## Review Focus

The spec implies these input classes and failure modes; each one has a test pinned to the task that owns the code.

1. **Creating a branch with `active: true` on a project that already has an active branch** — the previously active branch must become inactive. The switch rule applies on create, not only on update. → Task 4, Task 8.
2. **Renaming a branch to the name it already has** — must succeed. The unique constraint is on `(project_id, branch)`, and updating a row to its own value does not violate it. → Task 4, Task 8.
3. **A name that collides only after trimming** (`"2026Q1"` vs `" 2026Q1 "`) — must be rejected as a duplicate, because the trimmed form is what is stored and what the unique constraint compares. → Task 3, Task 4.
4. **Activating the branch that is already active** — must be a successful no-op that leaves the other branches alone, not an error. → Task 4.
5. **Creating a branch for a project id that does not exist** — the foreign key must reject it, and the failure must surface as a domain/repository error rather than an unhandled 500. → Task 8.

---

### Task 1: The `project_branches` migration

**Files:**
- Create: `lib/crates/project/migrations/<CLI-timestamp>_create_project_branches.sql`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/tests.rs` (add schema assertions)

**Interfaces:**
- Consumes: nothing — this task is self-contained.
- Produces: the `project_branches` table, relied on by Task 4's repository and Task 8's live-DB tests.

- [ ] **Step 1: Create the migration file with the sqlx CLI**

Run:

```bash
sqlx migrate add --source lib/crates/project/migrations --timestamp create_project_branches
```

The CLI prints the path it created, e.g.
`migrations/20261105120000_create_project_branches.sql`. **Use that exact
filename** in every later step — never guess it. Record it as `<MIGRATION>`.

- [ ] **Step 2: Write the failing schema test**

Append to `lib/crates/project/src/adapter/persistence/postgres/tests.rs`. The
file already has `load_migration(name: &str) -> String` and
`create_table_block(sql: &str) -> String` helpers — reuse them.

```rust
#[test]
fn project_branches_migration_creates_the_table_with_required_columns() {
    let block = create_table_block(&load_migration("<MIGRATION>"));
    let upper = block.to_uppercase();
    for required in [
        "ID INTEGER",
        "PROJECT_ID INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE",
        "BRANCH TEXT NOT NULL",
        "ACTIVE BOOLEAN NOT NULL DEFAULT FALSE",
        "CREATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "UPDATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
    ] {
        assert!(
            upper.contains(&required.to_uppercase()),
            "project_branches table must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn project_branches_migration_makes_the_name_unique_within_a_project() {
    let block = create_table_block(&load_migration("<MIGRATION>"));
    assert!(
        block.contains("UNIQUE (project_id, branch)"),
        "expected UNIQUE (project_id, branch); got:\n{block}"
    );
}

#[test]
fn project_branches_migration_rejects_a_blank_name() {
    let block = create_table_block(&load_migration("<MIGRATION>"));
    assert!(
        block.contains("length(btrim(branch)) > 0"),
        "expected a CHECK on the trimmed name; got:\n{block}"
    );
}

#[test]
fn project_branches_migration_allows_at_most_one_active_branch_per_project() {
    let sql = load_migration("<MIGRATION>");
    assert!(
        sql.contains("CREATE UNIQUE INDEX project_branches_one_active_per_project"),
        "the partial unique index is the database backstop for the switch rule; got:\n{sql}"
    );
    assert!(
        sql.contains("ON project_branches (project_id) WHERE active"),
        "the partial unique index must be scoped to active rows of one project; got:\n{sql}"
    );
}

#[test]
fn project_branches_migration_has_updated_at_trigger() {
    let sql = load_migration("<MIGRATION>");
    assert!(sql.contains("CREATE TRIGGER project_branches_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON project_branches"));
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run:

```bash
cargo test -p project project_branches_migration
```

Expected: FAIL — `migration file <MIGRATION> must exist` (the CLI created an
empty file, so `create_table_block` panics on the missing `CREATE TABLE`).

- [ ] **Step 4: Write the DDL into the generated file**

Replace the entire contents of `lib/crates/project/migrations/<MIGRATION>` with:

```sql
-- Create the `project_branches` table: the named working branches a
-- project owns over time, of which at most one is active at a time.
--
-- Layout:
--   * `project_branches`
--       - `id`          - surrogate primary key.
--       - `project_id`  - FK to `projects(id)`, ON DELETE CASCADE.
--       - `branch`      - the branch name. Unique within its project;
--                         the same name may exist under two projects.
--       - `active`      - whether this is the project's current branch.
--                         At most one row per project may be active;
--                         the partial unique index below enforces it.
--       - `created_at`  - DEFAULT NOW() at insert.
--       - `updated_at`  - DEFAULT NOW() at insert; the
--                         `project_branches_set_updated_at` trigger
--                         refreshes it.
--
-- The `active` flag is a switch, not a free-standing flag: the
-- application clears the project's other active rows before setting a
-- new one, inside one transaction. The partial unique index is the
-- database-level backstop for that rule.

CREATE TABLE project_branches (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    branch TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT project_branches_branch_unique UNIQUE (project_id, branch),
    CONSTRAINT project_branches_branch_not_empty CHECK (length(btrim(branch)) > 0)
);

CREATE UNIQUE INDEX project_branches_one_active_per_project
    ON project_branches (project_id) WHERE active;

CREATE OR REPLACE FUNCTION project_branches_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER project_branches_set_updated_at
    BEFORE UPDATE ON project_branches
    FOR EACH ROW
    EXECUTE FUNCTION project_branches_set_updated_at();
```

- [ ] **Step 5: Run the test to verify it passes**

Run:

```bash
cargo test -p project project_branches_migration
```

Expected: PASS — 5 tests.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/project/migrations lib/crates/project/src/adapter/persistence/postgres/tests.rs
git commit -m "feat(project): add the project_branches migration

Creates the branch table with the (project_id, branch) unique constraint,
the blank-name CHECK, the updated_at trigger, and the partial unique index
that backs the at-most-one-active-branch rule.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 2: The `ProjectBranch` domain model

**Files:**
- Create: `lib/crates/project/src/domain/project_branch.rs`
- Modify: `lib/crates/project/src/domain.rs` (module declaration + re-export)
- Modify: `lib/crates/project/src/domain/error.rs` (three new variants)
- Modify: `lib/crates/project/src/domain/tests.rs` (unit tests)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `domain::ProjectBranch { id: i32, project_id: i32, branch: String, active: bool, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`
  - `domain::ProjectBranch::new(id: i32, project_id: i32, branch: String, active: bool, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Result<Self, DomainError>`
  - `domain::ProjectBranch::for_repository(id: i32, project_id: i32, branch: String, active: bool, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self` (`pub(crate)`)
  - `domain::ProjectBranchNew { project_id: i32, branch: String, active: bool }`
  - `domain::ProjectBranchUpdate { id: i32, project_id: i32, branch: Option<String>, active: Option<bool> }`
  - `domain::ProjectBranchRepository` — `#[async_trait]`, `Send + Sync`, with `create`, `find_by_id`, `list_by_project`, `update`.
  - `DomainError::EmptyBranchName`, `DomainError::DuplicateBranch(String)`, `DomainError::BranchNotFound(i32)`.

- [ ] **Step 1: Write the failing domain test**

Append to `lib/crates/project/src/domain/tests.rs`. The file already has a
`test_now()` helper returning `chrono::DateTime<Utc>`.

```rust
#[test]
fn project_branch_new_rejects_an_empty_name() {
    let err = ProjectBranch::new(1, 1, "".into(), false, test_now(), test_now()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyBranchName));
}

#[test]
fn project_branch_new_rejects_a_whitespace_only_name() {
    let err =
        ProjectBranch::new(1, 1, "   ".into(), false, test_now(), test_now()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyBranchName));
}

#[test]
fn project_branch_new_accepts_a_valid_name() {
    let b = ProjectBranch::new(7, 3, "2026Q1".into(), true, test_now(), test_now()).unwrap();
    assert_eq!(b.id, 7);
    assert_eq!(b.project_id, 3);
    assert_eq!(b.branch, "2026Q1");
    assert!(b.active);
}

#[test]
fn project_branch_for_repository_bypasses_validation() {
    let b = ProjectBranch::for_repository(1, 1, "".into(), false, test_now(), test_now());
    assert_eq!(b.branch, "");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p project project_branch_new
```

Expected: FAIL — `cannot find type ProjectBranch in this scope`.

- [ ] **Step 3: Add the domain error variants**

In `lib/crates/project/src/domain/error.rs`, add to the `DomainError` enum
(place `EmptyBranchName` next to `EmptyCode`, and the other two next to
`NotFound` / `DuplicateCode`):

```rust
    #[error("branch name must not be empty")]
    EmptyBranchName,

    #[error("branch not found: {0}")]
    BranchNotFound(i32),

    #[error("branch already exists: {0}")]
    DuplicateBranch(String),
```

- [ ] **Step 4: Create the domain module**

Create `lib/crates/project/src/domain/project_branch.rs`:

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A named working branch of a project.
///
/// At most one branch of a project is active at a time. The switch is
/// performed by the repository (which can deactivate the project's other
/// branches in the same transaction), so the domain type itself only
/// carries the flag.
#[derive(Clone, PartialEq, Eq)]
pub struct ProjectBranch {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectBranch {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    ///
    /// Rejects a name that is empty or whitespace-only. The name is
    /// stored as given; trimming happens in the usecase layer, which is
    /// the only place that knows the value comes from a caller.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        project_id: i32,
        branch: String,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if branch.trim().is_empty() {
            return Err(DomainError::EmptyBranchName);
        }
        Ok(Self {
            id,
            project_id,
            branch,
            active,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i32,
        project_id: i32,
        branch: String,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            project_id,
            branch,
            active,
            created_at,
            updated_at,
        }
    }
}

impl std::fmt::Debug for ProjectBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectBranch")
            .field("id", &self.id)
            .field("project_id", &self.project_id)
            .field("branch", &self.branch)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Input DTO for `ProjectBranchRepository::create`.
#[derive(Debug, Clone)]
pub struct ProjectBranchNew {
    pub project_id: i32,
    pub branch: String,
    /// When true, the repository deactivates the project's other
    /// branches before inserting this one as active.
    pub active: bool,
}

/// Input DTO for `ProjectBranchRepository::update`. Every field is
/// optional so the usecase can pass only what changed.
///
/// `project_id` scopes the write: the repository only touches a branch
/// that belongs to that project, so a branch of another project is
/// reported as not found rather than updated.
#[derive(Debug, Clone, Default)]
pub struct ProjectBranchUpdate {
    pub id: i32,
    pub project_id: i32,
    pub branch: Option<String>,
    pub active: Option<bool>,
}

/// Outbound port for persistence of project branches.
///
/// The switch rule — "at most one active branch per project" — lives
/// in this contract rather than in the domain type, because making
/// branch B active requires deactivating the project's other branches,
/// which is a multi-row operation only the adapter can perform
/// atomically.
#[async_trait]
pub trait ProjectBranchRepository: Send + Sync {
    /// Creates a branch. When `input.active` is true, every currently
    /// active branch of `input.project_id` is deactivated first, in the
    /// same transaction.
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError>;

    /// Fetches one branch by its id. Returns
    /// [`DomainError::BranchNotFound`] when no such branch exists.
    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError>;

    /// Lists every branch of one project, ordered by id.
    async fn list_by_project(&self, project_id: i32) -> Result<Vec<ProjectBranch>, DomainError>;

    /// Updates a branch.
    ///
    /// - `branch: Some(name)` renames it. A name that collides with
    ///   another branch of the same project surfaces as
    ///   [`DomainError::DuplicateBranch`].
    /// - `active: Some(true)` makes this branch the project's active
    ///   one and deactivates the project's other branches, in the same
    ///   transaction.
    /// - `active: Some(false)` deactivates this branch only; the
    ///   project is then left with no active branch.
    /// - `active: None` leaves the flag alone.
    ///
    /// The update is scoped to `input.project_id`: a branch belonging to
    /// another project is reported as
    /// [`DomainError::BranchNotFound`].
    async fn update(&self, input: ProjectBranchUpdate) -> Result<ProjectBranch, DomainError>;
}
```

- [ ] **Step 5: Wire the module into the domain layer**

In `lib/crates/project/src/domain.rs`, add `mod project_branch;` to the
module list (alphabetically, before `mod project_configuration;`) and add to
the `pub use` block:

```rust
pub use project_branch::{
    ProjectBranch, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate,
};
```

- [ ] **Step 6: Run the test to verify it passes**

Run:

```bash
cargo test -p project project_branch
```

Expected: PASS — 4 tests. (`ProjectBranch::new` and `for_repository` are
`pub(crate)`, so they are visible to the in-crate `domain::tests` module.)

- [ ] **Step 7: Run the crate's checks**

Run:

```bash
cargo test -p project && cargo clippy -p project --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
```

Expected: PASS, no warnings, no formatting diff.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/project/src/domain.rs lib/crates/project/src/domain
git commit -m "feat(project): add the ProjectBranch domain model

Adds the branch aggregate, its repository port, and the three domain
errors the branch flow needs (EmptyBranchName, DuplicateBranch,
BranchNotFound). The at-most-one-active rule is documented on the port
because it is a cross-row operation the adapter performs atomically.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 3: The `ProjectBranchUsecase`

**Files:**
- Create: `lib/crates/project/src/usecase/project_branch_usecase.rs`
- Modify: `lib/crates/project/src/usecase/commands.rs`
- Modify: `lib/crates/project/src/usecase/views.rs`
- Modify: `lib/crates/project/src/usecase.rs` (module + re-exports)
- Modify: `lib/crates/project/src/usecase/tests.rs` (mock repo + tests)

**Interfaces:**
- Consumes: `domain::ProjectBranch`, `domain::ProjectBranchNew`,
  `domain::ProjectBranchUpdate`, `domain::ProjectBranchRepository`,
  `domain::DomainError` — all from Task 2.
- Produces:
  - `usecase::ProjectBranchUsecase<B: ProjectBranchRepository>` with
    `ProjectBranchUsecase::new(branches: B) -> Self`
  - `ProjectBranchUsecase::create_branch(cmd: CreateProjectBranch) -> Result<ProjectBranchView, UsecaseError>`
  - `ProjectBranchUsecase::get_branch_by_id(id: i32) -> Result<ProjectBranchView, UsecaseError>`
  - `ProjectBranchUsecase::list_branches(project_id: i32) -> Result<Vec<ProjectBranchView>, UsecaseError>`
  - `ProjectBranchUsecase::update_branch(cmd: UpdateProjectBranch) -> Result<ProjectBranchView, UsecaseError>`
  - `usecase::CreateProjectBranch { project_id: i32, branch: String, active: bool }`
  - `usecase::UpdateProjectBranch { id: i32, project_id: i32, branch: Option<String>, active: Option<bool> }`
  - `usecase::ProjectBranchView { id: i32, project_id: i32, branch: String, active: bool, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`

- [ ] **Step 1: Write the failing usecase test**

Append to `lib/crates/project/src/usecase/tests.rs`. The file already has a
`mock_now()` helper and a `MockProjectRepo` pattern to follow.

First add the mock repository right after the existing `MockProjectRepo`
implementation:

```rust
// ---------- mock branch repo ----------

#[derive(Default)]
struct MockBranchState {
    branches: HashMap<i32, ProjectBranch>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockBranchRepo {
    state: Arc<Mutex<MockBranchState>>,
}

impl MockBranchRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockBranchState {
                branches: HashMap::new(),
                next_id: 1,
            })),
        }
    }
}

#[async_trait]
impl ProjectBranchRepository for MockBranchRepo {
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s
            .branches
            .values()
            .any(|b| b.project_id == input.project_id && b.branch == input.branch)
        {
            return Err(DomainError::DuplicateBranch(input.branch));
        }
        let id = s.next_id;
        s.next_id += 1;
        let now = mock_now();
        let branch = ProjectBranch::for_repository(
            id,
            input.project_id,
            input.branch,
            input.active,
            now,
            now,
        );
        s.branches.insert(id, branch.clone());
        Ok(branch)
    }
    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError> {
        self.state
            .lock()
            .unwrap()
            .branches
            .get(&id)
            .cloned()
            .ok_or(DomainError::BranchNotFound(id))
    }
    async fn list_by_project(&self, project_id: i32) -> Result<Vec<ProjectBranch>, DomainError> {
        let mut out: Vec<ProjectBranch> = self
            .state
            .lock()
            .unwrap()
            .branches
            .values()
            .filter(|b| b.project_id == project_id)
            .cloned()
            .collect();
        out.sort_by_key(|b| b.id);
        Ok(out)
    }
    async fn update(&self, input: ProjectBranchUpdate) -> Result<ProjectBranch, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref name) = input.branch {
            let dup = s.branches.values().any(|o| {
                o.project_id == input.project_id && o.branch == *name && o.id != input.id
            });
            if dup {
                return Err(DomainError::DuplicateBranch(name.clone()));
            }
        }
        let b = s
            .branches
            .get_mut(&input.id)
            .filter(|b| b.project_id == input.project_id)
            .ok_or(DomainError::BranchNotFound(input.id))?;
        if let Some(ref name) = input.branch {
            b.branch = name.clone();
        }
        if let Some(a) = input.active {
            b.active = a;
        }
        Ok(b.clone())
    }
}
```

Then add the tests:

```rust
#[tokio::test]
async fn create_branch_rejects_a_blank_name() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let err = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "   ".into(),
            active: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Validation(DomainError::EmptyBranchName)));
}

#[tokio::test]
async fn create_branch_stores_the_trimmed_name() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let view = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "  2026Q1  ".into(),
            active: false,
        })
        .await
        .expect("create");
    assert_eq!(view.branch, "2026Q1");
}

#[tokio::test]
async fn update_branch_rejects_a_blank_name() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let err = usecase
        .update_branch(UpdateProjectBranch {
            id: 1,
            project_id: 1,
            branch: Some("".into()),
            active: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Validation(DomainError::EmptyBranchName)));
}

#[tokio::test]
async fn list_branches_returns_only_the_project_s_own_branches() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo.clone());
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "a".into(),
            active: false,
        })
        .await
        .expect("create a");
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 2,
            branch: "b".into(),
            active: false,
        })
        .await
        .expect("create b");
    let branches = usecase.list_branches(1).await.expect("list");
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].branch, "a");
}

#[tokio::test]
async fn update_branch_renames_the_branch() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let created = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "old".into(),
            active: false,
        })
        .await
        .expect("create");
    let updated = usecase
        .update_branch(UpdateProjectBranch {
            id: created.id,
            project_id: 1,
            branch: Some("new".into()),
            active: None,
        })
        .await
        .expect("update");
    assert_eq!(updated.branch, "new");
}

#[tokio::test]
async fn create_branch_rejects_a_name_that_collides_only_after_trimming() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "2026Q1".into(),
            active: false,
        })
        .await
        .expect("create");
    let err = usecase
        .create_branch(CreateProjectBranch {
            project_id: 1,
            branch: "  2026Q1  ".into(),
            active: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::DuplicateBranch(_))
    ));
}

#[tokio::test]
async fn list_branches_of_a_project_with_no_branches_is_empty() {
    let repo = MockBranchRepo::new();
    let usecase = ProjectBranchUsecase::new(repo);
    let branches = usecase.list_branches(1).await.expect("list");
    assert!(branches.is_empty());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p project branch
```

Expected: FAIL — `cannot find type ProjectBranchUsecase in this scope`.

- [ ] **Step 3: Add the command and view DTOs**

In `lib/crates/project/src/usecase/commands.rs`, append:

```rust
/// Command to create a branch of a project.
#[derive(Debug, Clone)]
pub struct CreateProjectBranch {
    pub project_id: i32,
    pub branch: String,
    /// When true the new branch becomes the project's active one and
    /// the project's other branches are deactivated.
    pub active: bool,
}

/// Command to rename a branch and / or change whether it is the
/// project's active branch. `branch: None` leaves the name alone;
/// `active: None` leaves the flag alone.
#[derive(Debug, Clone)]
pub struct UpdateProjectBranch {
    pub id: i32,
    /// Scopes the write to one project. A branch of another project is
    /// reported as not found.
    pub project_id: i32,
    pub branch: Option<String>,
    pub active: Option<bool>,
}
```

In `lib/crates/project/src/usecase/views.rs`, append:

```rust
/// Server-side projection of a single project branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBranchView {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<crate::domain::ProjectBranch> for ProjectBranchView {
    fn from(b: crate::domain::ProjectBranch) -> Self {
        Self {
            id: b.id,
            project_id: b.project_id,
            branch: b.branch,
            active: b.active,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }
    }
}
```

- [ ] **Step 4: Create the usecase module**

Create `lib/crates/project/src/usecase/project_branch_usecase.rs`:

```rust
use crate::domain::{
    DomainError, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate,
};

use super::commands::{CreateProjectBranch, UpdateProjectBranch};
use super::error::UsecaseError;
use super::views::ProjectBranchView;

/// Orchestrates the project-branch port.
///
/// The branch aggregate has its own lifecycle, so it gets its own
/// usecase rather than growing `ProjectUsecase`.
pub struct ProjectBranchUsecase<B: ProjectBranchRepository> {
    branches: B,
}

impl<B: ProjectBranchRepository> ProjectBranchUsecase<B> {
    pub fn new(branches: B) -> Self {
        Self { branches }
    }

    /// Creates a branch of a project.
    pub async fn create_branch(
        &self,
        cmd: CreateProjectBranch,
    ) -> Result<ProjectBranchView, UsecaseError> {
        let branch = cmd.branch.trim();
        if branch.is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyBranchName));
        }
        let created = self
            .branches
            .create(ProjectBranchNew {
                project_id: cmd.project_id,
                branch: branch.to_string(),
                active: cmd.active,
            })
            .await?;
        Ok(created.into())
    }

    /// Fetches one branch by its id.
    pub async fn get_branch_by_id(&self, id: i32) -> Result<ProjectBranchView, UsecaseError> {
        let branch = self.branches.find_by_id(id).await?;
        Ok(branch.into())
    }

    /// Lists every branch of one project.
    pub async fn list_branches(
        &self,
        project_id: i32,
    ) -> Result<Vec<ProjectBranchView>, UsecaseError> {
        let branches = self.branches.list_by_project(project_id).await?;
        Ok(branches.into_iter().map(Into::into).collect())
    }

    /// Renames a branch and / or changes whether it is the project's
    /// active branch.
    pub async fn update_branch(
        &self,
        cmd: UpdateProjectBranch,
    ) -> Result<ProjectBranchView, UsecaseError> {
        let branch = match cmd.branch {
            Some(ref name) => {
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    return Err(UsecaseError::Validation(DomainError::EmptyBranchName));
                }
                Some(trimmed.to_string())
            }
            None => None,
        };
        let updated = self
            .branches
            .update(ProjectBranchUpdate {
                id: cmd.id,
                project_id: cmd.project_id,
                branch,
                active: cmd.active,
            })
            .await?;
        Ok(updated.into())
    }
}
```

- [ ] **Step 5: Wire the module into the usecase layer**

In `lib/crates/project/src/usecase.rs`, add `mod project_branch_usecase;` to
the module list and extend the `pub use` blocks:

```rust
pub use commands::{CreateProject, CreateProjectBranch, UpdateProject, UpdateProjectBranch};
pub use project_branch_usecase::ProjectBranchUsecase;
pub use views::{
    ModelVersionView, ProjectBranchView, ProjectConfigurationView, ProjectMemberView, ProjectView,
    TagView, TerminologyVersionView, UserSummaryView,
};
```

- [ ] **Step 6: Add the new imports to the usecase test module**

In `lib/crates/project/src/usecase/tests.rs`, extend the imports:

```rust
use crate::domain::{
    DomainError, ModelVersion, Project, ProjectBranch, ProjectBranchNew, ProjectBranchRepository,
    ProjectBranchUpdate, ProjectConfiguration, ProjectMember, ProjectNew, ProjectRepository,
    ProjectTag, ProjectUpdate, UserService, UserSummary,
};
use crate::usecase::commands::{
    CreateProject, CreateProjectBranch, UpdateProject, UpdateProjectBranch,
};
use crate::usecase::ProjectBranchUsecase;
```

- [ ] **Step 7: Run the test to verify it passes**

Run:

```bash
cargo test -p project branch
```

Expected: PASS — 6 new usecase tests plus the domain tests from Task 2.

- [ ] **Step 8: Run the crate's checks**

Run:

```bash
cargo test -p project && cargo clippy -p project --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
```

Expected: PASS, no warnings, no formatting diff.

- [ ] **Step 9: Commit**

```bash
git add lib/crates/project/src/usecase.rs lib/crates/project/src/usecase
git commit -m "feat(project): add the ProjectBranchUsecase

Orchestrates the branch port: trims and validates the branch name,
scopes writes to one project, and projects the domain type into a
ProjectBranchView.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 4: The PostgreSQL branch repository

**Files:**
- Create: `lib/crates/project/src/adapter/persistence/postgres/branch_repo.rs`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/row.rs`
- Modify: `lib/crates/project/src/adapter/persistence/postgres.rs`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/tests.rs` (row-conversion tests)

**Interfaces:**
- Consumes: `domain::ProjectBranch`, `domain::ProjectBranchNew`,
  `domain::ProjectBranchUpdate`, `domain::ProjectBranchRepository` (Task 2);
  the `project_branches` table (Task 1).
- Produces: `adapter::persistence::postgres::ProjectBranchRepo` —
  `ProjectBranchRepo::new(pool: PgPool) -> Self`, implementing
  `ProjectBranchRepository`.

- [ ] **Step 1: Write the failing row-conversion test**

In `lib/crates/project/src/adapter/persistence/postgres/tests.rs`, inside the
existing `mod row_tests`, add:

```rust
    #[test]
    fn project_branch_row_converts_to_a_project_branch() {
        let row = ProjectBranchRow {
            id: 5,
            project_id: 2,
            branch: "2026Q1".into(),
            active: true,
            created_at: ts(),
            updated_at: ts(),
        };
        let b: crate::domain::ProjectBranch = row.try_into().expect("convert");
        assert_eq!(b.id, 5);
        assert_eq!(b.project_id, 2);
        assert_eq!(b.branch, "2026Q1");
        assert!(b.active);
    }
```

and extend the module's `use` line to include `ProjectBranchRow`:

```rust
    use super::super::row::{ProjectBranchRow, ProjectMemberRow, ProjectRow};
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p project project_branch_row
```

Expected: FAIL — `cannot find struct ProjectBranchRow in this scope`.

- [ ] **Step 3: Add the row bridge**

In `lib/crates/project/src/adapter/persistence/postgres/row.rs`, append:

```rust
/// One row from `project_branches`.
#[derive(Clone, FromRow)]
pub struct ProjectBranchRow {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectBranchRow> for ProjectBranch {
    type Error = DomainError;

    fn try_from(row: ProjectBranchRow) -> Result<Self, Self::Error> {
        Ok(ProjectBranch::for_repository(
            row.id,
            row.project_id,
            row.branch,
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}
```

and extend that file's imports:

```rust
use crate::domain::{
    DomainError, Project, ProjectBranch, ProjectConfiguration, ProjectMember,
};
```

- [ ] **Step 4: Create the repository**

Create `lib/crates/project/src/adapter/persistence/postgres/branch_repo.rs`:

```rust
use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, ProjectBranch, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate,
};

use super::row::ProjectBranchRow;

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

/// The name of the constraint that enforces branch-name uniqueness
/// within a project.
const BRANCH_UNIQUE_CONSTRAINT: &str = "project_branches_branch_unique";

/// PostgreSQL-backed implementation of `ProjectBranchRepository`.
///
/// The switch rule — at most one active branch per project — is
/// enforced here, inside a transaction: the project's other active
/// branches are cleared before a new branch is activated. The
/// `project_branches_one_active_per_project` partial unique index is
/// the database-level backstop.
pub struct ProjectBranchRepo {
    pool: PgPool,
}

impl ProjectBranchRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectBranchRepository for ProjectBranchRepo {
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // The switch: a branch created as active deactivates the
        // project's other active branches first, in the same
        // transaction.
        if input.active {
            sqlx::query(
                "UPDATE project_branches SET active = FALSE \
                 WHERE project_id = $1 AND active",
            )
            .bind(input.project_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let row: ProjectBranchRow = sqlx::QueryBuilder::new(
            "INSERT INTO project_branches (project_id, branch, active) VALUES (",
        )
        .push_bind(input.project_id)
        .push(", ")
        .push_bind(&input.branch)
        .push(", ")
        .push_bind(input.active)
        .push(") RETURNING id, project_id, branch, active, created_at, updated_at")
        .build_query_as::<ProjectBranchRow>()
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        tx.commit().await.map_err(map_db_error)?;

        row.try_into()
    }

    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError> {
        let row: ProjectBranchRow = sqlx::QueryBuilder::new(
            "SELECT id, project_id, branch, active, created_at, updated_at \
             FROM project_branches WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<ProjectBranchRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::BranchNotFound(id))?;
        row.try_into()
    }

    async fn list_by_project(&self, project_id: i32) -> Result<Vec<ProjectBranch>, DomainError> {
        let rows: Vec<ProjectBranchRow> = sqlx::QueryBuilder::new(
            "SELECT id, project_id, branch, active, created_at, updated_at \
             FROM project_branches WHERE project_id = ",
        )
        .push_bind(project_id)
        .push(" ORDER BY id")
        .build_query_as::<ProjectBranchRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(|row| row.try_into()).collect()
    }

    async fn update(&self, input: ProjectBranchUpdate) -> Result<ProjectBranch, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // The write is scoped to the owning project: a branch of
        // another project is reported as not found rather than
        // updated.
        let owner: Option<(i32,)> =
            sqlx::QueryBuilder::new("SELECT project_id FROM project_branches WHERE id = ")
                .push_bind(input.id)
                .build_query_as::<(i32,)>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        match owner {
            None => return Err(DomainError::BranchNotFound(input.id)),
            Some((project_id,)) if project_id != input.project_id => {
                return Err(DomainError::BranchNotFound(input.id));
            }
            Some(_) => {}
        }

        // The switch: activating this branch deactivates the
        // project's other active branches, in the same transaction.
        if input.active == Some(true) {
            sqlx::query(
                "UPDATE project_branches SET active = FALSE \
                 WHERE project_id = $1 AND id <> $2 AND active",
            )
            .bind(input.project_id)
            .bind(input.id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let mut qb = sqlx::QueryBuilder::new("UPDATE project_branches SET ");
        let mut first = true;
        if let Some(ref branch) = input.branch {
            qb.push("branch = ").push_bind(branch);
            first = false;
        }
        if let Some(active) = input.active {
            if !first {
                qb.push(", ");
            }
            qb.push("active = ").push_bind(active);
            first = false;
        }
        if !first {
            qb.push(" WHERE id = ")
                .push_bind(input.id)
                .push(" AND project_id = ")
                .push_bind(input.project_id)
                .push(" RETURNING id, project_id, branch, active, created_at, updated_at");
            let row: ProjectBranchRow = qb
                .build_query_as::<ProjectBranchRow>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?
                .ok_or(DomainError::BranchNotFound(input.id))?;
            tx.commit().await.map_err(map_db_error)?;
            return row.try_into();
        }

        // Nothing to change: return the branch as it stands.
        tx.commit().await.map_err(map_db_error)?;
        self.find_by_id(input.id).await
    }
}

/// Translate a driver error into a domain error.
///
/// A unique violation on the branch-name constraint is a duplicate
/// branch; any other unique violation keeps the generic
/// `DuplicateCode` behaviour.
fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                if db_err.constraint() == Some(BRANCH_UNIQUE_CONSTRAINT) {
                    DomainError::DuplicateBranch(db_err.message().to_string())
                } else {
                    DomainError::DuplicateCode(format!("(constraint {})", db_err.constraint().unwrap_or("code")))
                }
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}
```

- [ ] **Step 5: Register the repository module**

In `lib/crates/project/src/adapter/persistence/postgres.rs`, add
`pub(crate) mod branch_repo;` to the module list and add to the re-export
block:

```rust
pub use branch_repo::ProjectBranchRepo;
```

- [ ] **Step 6: Run the test to verify it passes**

Run:

```bash
cargo test -p project project_branch_row
```

Expected: PASS.

- [ ] **Step 7: Run the crate's checks**

Run:

```bash
cargo test -p project && cargo clippy -p project --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
```

Expected: PASS, no warnings, no formatting diff.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/project/src/adapter/persistence
git commit -m "feat(project): add the PostgreSQL branch repository

Implements ProjectBranchRepository against the project_branches table.
The switch rule - at most one active branch per project - is applied
inside a transaction: the project's other active branches are cleared
before a branch is activated, on create and on update alike. Writes are
scoped to the owning project, so a branch of another project is reported
as not found.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 5: The `apis` branch contract

**Files:**
- Modify: `lib/crates/apis/src/project.rs`
- Modify: `lib/crates/apis/tests/public_api.rs` (add an `apis::project` section)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `apis::project::ProjectBranchView { id: i32, project_id: i32, branch: String, active: bool, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`
  - `apis::project::CreateProjectBranchRequest { project_id: i32, branch: String, active: bool }`
  - `apis::project::UpdateProjectBranchRequest { id: i32, project_id: i32, branch: Option<String>, active: Option<bool> }`
  - `apis::project::ProjectApiError::DuplicateBranch(String)` and `::BranchNotFound(i32)`

- [ ] **Step 1: Write the failing public-API test**

Append to `lib/crates/apis/tests/public_api.rs`:

```rust
// ---- project ----------------------------------------------------------

use apis::project::{
    CreateProjectBranchRequest, ProjectApiError, ProjectBranchView, ProjectService,
    UpdateProjectBranchRequest,
};

/// Every new branch type in `apis::project` is nameable from the test.
#[test]
fn project_branch_public_types_are_nameable() {
    fn assert_view(_: ProjectBranchView) {}
    fn assert_create(_: CreateProjectBranchRequest) {}
    fn assert_update(_: UpdateProjectBranchRequest) {}
    fn assert_err(_: ProjectApiError) {}

    let _view = ProjectBranchView {
        id: 1,
        project_id: 2,
        branch: "2026Q1".into(),
        active: true,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    };
    let _create = CreateProjectBranchRequest {
        project_id: 2,
        branch: "2026Q1".into(),
        active: false,
    };
    let _update = UpdateProjectBranchRequest {
        id: 1,
        project_id: 2,
        branch: Some("2026Q2".into()),
        active: Some(true),
    };
    assert_err(ProjectApiError::DuplicateBranch("2026Q1".into()));
    assert_err(ProjectApiError::BranchNotFound(7));
}

/// The branch methods do not break the trait's object safety.
#[test]
fn project_service_with_branch_methods_is_object_safe() {
    fn assert_box_dyn_service<S: ProjectService + 'static>() {}
    assert_box_dyn_service::<Box<dyn ProjectService>>();
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p apis project_branch_public_types
```

Expected: FAIL — `cannot find type ProjectBranchView in this scope`.

- [ ] **Step 3: Add the error variants**

In `lib/crates/apis/src/project.rs`, extend `ProjectApiError`:

```rust
    #[error("branch not found: {0}")]
    BranchNotFound(i32),

    #[error("branch already exists: {0}")]
    DuplicateBranch(String),
```

- [ ] **Step 4: Add the branch DTOs**

In `lib/crates/apis/src/project.rs`, append:

```rust
/// Server-side projection of a single project branch.
///
/// Branches are a standalone resource: `ProjectView` does not carry
/// them, and they are read through the branch endpoints only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBranchView {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request-side payload for creating a branch of a project.
///
/// `project_id` is resolved from the project's code by the transport
/// layer before the request reaches the backend.
#[derive(Debug, Clone)]
pub struct CreateProjectBranchRequest {
    pub project_id: i32,
    pub branch: String,
    /// Whether the new branch becomes the project's active one. When
    /// true, the project's other active branches are deactivated.
    pub active: bool,
}

/// Request-side payload for renaming a branch and / or changing
/// whether it is the project's active branch.
///
/// `None` for `branch` leaves the name alone; `None` for `active`
/// leaves the flag alone. `project_id` scopes the write, so a branch
/// of another project is reported as not found.
#[derive(Debug, Clone, Default)]
pub struct UpdateProjectBranchRequest {
    pub id: i32,
    pub project_id: i32,
    pub branch: Option<String>,
    pub active: Option<bool>,
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run:

```bash
cargo test -p apis project
```

Expected: PASS.

- [ ] **Step 6: Run the crate's checks**

Run:

```bash
cargo test -p apis && cargo clippy -p apis --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
```

Expected: PASS, no warnings, no formatting diff.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/apis
git commit -m "feat(apis): add the project branch contract

Adds the branch view and request DTOs plus the two branch error
variants. The trait methods themselves land in the next commit, once
every implementation of ProjectService can satisfy them.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 6: The branch methods on `ProjectService`, and every implementation

**Files:**
- Modify: `lib/crates/apis/src/project.rs` (four trait methods)
- Modify: `lib/crates/project/src/adapter/facade/internal/service.rs`
- Modify: `lib/crates/project/src/adapter/facade/internal/tests.rs`
- Modify: `lib/crates/project/src/lib.rs` (crate-root re-exports)
- Modify: `lib/crates/project/tests/public_api.rs`
- Modify: `lib/crates/mission/src/adapter/service/project/tests.rs`
- Modify: `apps/server/aegis-server/tests/integration_auth.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/handlers.rs` (`MockProjectService`)
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (`NullProjectService`, `StubProjectService`)
- Modify: `apps/server/aegis-server/src/state.rs` (`NullProjectService`)
- Modify: `apps/server/aegis-server/src/transport/http/auth/middleware.rs` (`NullProjectService`)
- Modify: `apps/server/aegis-server/src/transport/http/auth/handlers.rs` (`NullProjectService`)
- Modify: `apps/server/aegis-server/src/transport/http/auth/user_credential/handlers.rs` (`NullProjectService`)
- Modify: `apps/server/aegis-server/src/transport/http/user/handlers.rs` (`NullProjectService`)

**Interfaces:**
- Consumes: `apis::project::ProjectBranchView`,
  `CreateProjectBranchRequest`, `UpdateProjectBranchRequest` (Task 5);
  `ProjectBranchUsecase` (Task 3).
- Produces: four methods on `apis::project::ProjectService`:
  - `create_project_branch(&self, req: CreateProjectBranchRequest) -> Result<ProjectBranchView, ProjectApiError>`
  - `get_project_branch_by_id(&self, id: i32) -> Result<ProjectBranchView, ProjectApiError>`
  - `list_project_branches(&self, project_id: i32) -> Result<Vec<ProjectBranchView>, ProjectApiError>`
  - `update_project_branch(&self, req: UpdateProjectBranchRequest) -> Result<ProjectBranchView, ProjectApiError>`

- [ ] **Step 1: Add the four trait methods**

In `lib/crates/apis/src/project.rs`, add to the `ProjectService` trait:

```rust
    /// Creates a branch of a project. When `req.active` is true the new
    /// branch becomes the project's active one and the project's other
    /// active branches are deactivated.
    async fn create_project_branch(
        &self,
        req: CreateProjectBranchRequest,
    ) -> Result<ProjectBranchView, ProjectApiError>;

    /// Fetches one branch by its id.
    async fn get_project_branch_by_id(
        &self,
        id: i32,
    ) -> Result<ProjectBranchView, ProjectApiError>;

    /// Lists every branch of one project.
    async fn list_project_branches(
        &self,
        project_id: i32,
    ) -> Result<Vec<ProjectBranchView>, ProjectApiError>;

    /// Renames a branch and / or changes whether it is the project's
    /// active branch.
    async fn update_project_branch(
        &self,
        req: UpdateProjectBranchRequest,
    ) -> Result<ProjectBranchView, ProjectApiError>;
```

- [ ] **Step 2: Implement the four methods in the facade**

In `lib/crates/project/src/adapter/facade/internal/service.rs`:

Extend the `use apis::project::{…}` import with `CreateProjectBranchRequest`,
`ProjectBranchView as ApiProjectBranchView`, `UpdateProjectBranchRequest`.

Extend the `use crate::usecase::{…}` import with `CreateProjectBranch`,
`ProjectBranchView`, `UpdateProjectBranch`.

Extend the `use crate::domain::{…}` import with `ProjectBranchRepository`.

Change the struct and its constructor to hold both usecases:

```rust
pub struct ProjectServiceImpl<R, U, B>
where
    R: ProjectRepository,
    U: UserService,
    B: ProjectBranchRepository,
{
    usecase: ProjectUsecase<R, U>,
    branch_usecase: ProjectBranchUsecase<B>,
}

impl<R, U, B> ProjectServiceImpl<R, U, B>
where
    R: ProjectRepository,
    U: UserService,
    B: ProjectBranchRepository,
{
    pub fn new(usecase: ProjectUsecase<R, U>, branch_usecase: ProjectBranchUsecase<B>) -> Self {
        Self {
            usecase,
            branch_usecase,
        }
    }
}
```

Change the `impl<R, U> ProjectService for ProjectServiceImpl<R, U>` header to
`impl<R, U, B> ProjectService for ProjectServiceImpl<R, U, B>` and extend its
`where` clause with `B: ProjectBranchRepository + 'static`.

Add the four methods to that impl block:

```rust
    async fn create_project_branch(
        &self,
        req: CreateProjectBranchRequest,
    ) -> Result<ApiProjectBranchView, ProjectApiError> {
        let view = self
            .branch_usecase
            .create_branch(CreateProjectBranch {
                project_id: req.project_id,
                branch: req.branch,
                active: req.active,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_branch_by_id(
        &self,
        id: i32,
    ) -> Result<ApiProjectBranchView, ProjectApiError> {
        let view = self
            .branch_usecase
            .get_branch_by_id(id)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn list_project_branches(
        &self,
        project_id: i32,
    ) -> Result<Vec<ApiProjectBranchView>, ProjectApiError> {
        let views = self
            .branch_usecase
            .list_branches(project_id)
            .await
            .map_err(map_error)?;
        Ok(views.into_iter().map(Into::into).collect())
    }

    async fn update_project_branch(
        &self,
        req: UpdateProjectBranchRequest,
    ) -> Result<ApiProjectBranchView, ProjectApiError> {
        let view = self
            .branch_usecase
            .update_branch(UpdateProjectBranch {
                id: req.id,
                project_id: req.project_id,
                branch: req.branch,
                active: req.active,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }
```

- [ ] **Step 3: Extend the facade's error mapping**

In the same file, extend the `map_error` function's inner match:

```rust
            DomainError::DuplicateBranch(name) => ProjectApiError::DuplicateBranch(name),
            DomainError::BranchNotFound(id) => ProjectApiError::BranchNotFound(id),
```

Add the `From` impl for the branch view, using the names already imported
in this file (`ProjectBranchView` is the usecase view, `ApiProjectBranchView`
is the apis view):

```rust
impl From<ProjectBranchView> for ApiProjectBranchView {
    fn from(v: ProjectBranchView) -> Self {
        Self {
            id: v.id,
            project_id: v.project_id,
            branch: v.branch,
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}
```

- [ ] **Step 4: Extend the crate's public surface**

In `lib/crates/project/src/lib.rs`, extend the `pub use domain::{…}` block
with `ProjectBranch`, `ProjectBranchNew`, `ProjectBranchRepository`,
`ProjectBranchUpdate`, and the `pub use usecase::{…}` block with
`CreateProjectBranch`, `ProjectBranchUsecase`, `ProjectBranchView`,
`UpdateProjectBranch`. Also re-export the Postgres adapter:

```rust
pub use adapter::persistence::postgres::ProjectBranchRepo;
```

- [ ] **Step 5: Update the facade's own tests**

In `lib/crates/project/src/adapter/facade/internal/tests.rs`, add an
in-memory branch repository next to the existing `InMemProjectRepo`:

```rust
#[derive(Default)]
struct InMemBranchState {
    branches: HashMap<i32, ProjectBranch>,
    next_id: AtomicI32,
}

#[derive(Clone, Default)]
struct InMemBranchRepo {
    state: Arc<Mutex<InMemBranchState>>,
}

impl InMemBranchRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemBranchState {
                branches: HashMap::new(),
                next_id: AtomicI32::new(1),
            })),
        }
    }
}

#[async_trait]
impl ProjectBranchRepository for InMemBranchRepo {
    async fn create(&self, input: ProjectBranchNew) -> Result<ProjectBranch, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s
            .branches
            .values()
            .any(|b| b.project_id == input.project_id && b.branch == input.branch)
        {
            return Err(DomainError::DuplicateBranch(input.branch));
        }
        let id = s.next_id.fetch_add(1, Ordering::SeqCst);
        let now = mock_now();
        let branch = ProjectBranch::for_repository(
            id,
            input.project_id,
            input.branch,
            input.active,
            now,
            now,
        );
        s.branches.insert(id, branch.clone());
        Ok(branch)
    }
    async fn find_by_id(&self, id: i32) -> Result<ProjectBranch, DomainError> {
        let s = self.state.lock().unwrap();
        let b = s.branches.get(&id).cloned().ok_or(DomainError::BranchNotFound(id))?;
        Ok(b)
    }
    async fn list_by_project(&self, project_id: i32) -> Result<Vec<ProjectBranch>, DomainError> {
        let mut out: Vec<ProjectBranch> = self
            .state
            .lock()
            .unwrap()
            .branches
            .values()
            .filter(|b| b.project_id == project_id)
            .cloned()
            .collect();
        out.sort_by_key(|b| b.id);
        Ok(out)
    }
    async fn update(&self, input: ProjectBranchUpdate) -> Result<ProjectBranch, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref name) = input.branch {
            let dup = s.branches.values().any(|o| {
                o.project_id == input.project_id && o.branch == *name && o.id != input.id
            });
            if dup {
                return Err(DomainError::DuplicateBranch(name.clone()));
            }
        }
        let b = s
            .branches
            .get_mut(&input.id)
            .filter(|b| b.project_id == input.project_id)
            .ok_or(DomainError::BranchNotFound(input.id))?;
        if let Some(ref name) = input.branch {
            b.branch = name.clone();
        }
        if let Some(a) = input.active {
            b.active = a;
        }
        Ok(b.clone())
    }
}
```

Extend that file's imports with `ProjectBranch`, `ProjectBranchNew`,
`ProjectBranchRepository`, `ProjectBranchUpdate` (from `crate::domain`) and
`ProjectBranchUsecase` (from `crate::usecase`).

Every place that constructs `ProjectServiceImpl::new(usecase)` in that file
becomes:

```rust
    let branch_usecase = ProjectBranchUsecase::new(InMemBranchRepo::new());
    let service = ProjectServiceImpl::new(usecase, branch_usecase);
```

- [ ] **Step 6: Update the project's public-API lock**

In `lib/crates/project/tests/public_api.rs`:

Add `ProjectBranch`, `ProjectBranchNew`, `ProjectBranchRepository`,
`ProjectBranchUpdate`, `ProjectBranchRepo`, `ProjectBranchUsecase`,
`ProjectBranchView`, `CreateProjectBranch`, `UpdateProjectBranch` to the
`use project::{…}` list. In the `use apis::project::{…}` list add
`CreateProjectBranchRequest`, `UpdateProjectBranchRequest`, and
`ProjectBranchView as ApiProjectBranchView` — the alias is required,
because `ProjectBranchView` exists in both crates and the unaliased
`project::ProjectBranchView` is already imported.

Change the object-safety pin:

```rust
    let _: fn(
        ProjectServiceImpl<ProjectRepo, UserServiceImpl, ProjectBranchRepo>,
    ) -> Box<dyn ProjectService> = |s| Box::new(s);
```

Add the new domain-error and api-error variants to the existing
nameability tests:

```rust
    assert_dom(DomainError::EmptyBranchName);
    assert_dom(DomainError::DuplicateBranch("2026Q1".into()));
    assert_dom(DomainError::BranchNotFound(7));
```

```rust
    assert_err(ProjectApiError::DuplicateBranch("2026Q1".into()));
    assert_err(ProjectApiError::BranchNotFound(7));
```

- [ ] **Step 7: Update every other `ProjectService` implementation**

Adding methods to the trait breaks **every** implementation of it, not
just the server's. A repo-wide grep for `ProjectService for` (excluding
`SdtmProjectService for`) finds these, and each needs the four methods
added:

| File | Implementor |
| --- | --- |
| `lib/crates/mission/src/adapter/service/project/tests.rs` | `FakeProject`, `NotFound` |
| `apps/server/aegis-server/tests/integration_auth.rs` | `NullProjectService` |
| `apps/server/aegis-server/src/transport/http/project/handlers.rs` | `MockProjectService` |
| `apps/server/aegis-server/src/transport/http/router.rs` | `NullProjectService`, `StubProjectService` |
| `apps/server/aegis-server/src/state.rs` | `NullProjectService` |
| `apps/server/aegis-server/src/transport/http/auth/middleware.rs` | `NullProjectService` |
| `apps/server/aegis-server/src/transport/http/auth/handlers.rs` | `NullProjectService` |
| `apps/server/aegis-server/src/transport/http/auth/user_credential/handlers.rs` | `NullProjectService` |
| `apps/server/aegis-server/src/transport/http/user/handlers.rs` | `NullProjectService` |

Every one of them except `MockProjectService` is a null double whose
other methods are already `unimplemented!()`, so the four new methods get
the same treatment. The block to add is:

```rust
        async fn create_project_branch(
            &self,
            _req: apis::project::CreateProjectBranchRequest,
        ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_branch_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn list_project_branches(
            &self,
            _project_id: i32,
        ) -> Result<Vec<apis::project::ProjectBranchView>, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn update_project_branch(
            &self,
            _req: apis::project::UpdateProjectBranchRequest,
        ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
            unimplemented!()
        }
```

Adapt the path prefix to the file: the server's own modules write
`apis::project::…` in full, while the mission crate's test module
imports the names at the top of the file and writes them bare
(`CreateProjectBranchRequest`, `ProjectBranchView`,
`UpdateProjectBranchRequest`, `ProjectApiError`). Match whatever the
surrounding methods in that file already do.

`MockProjectService` is handled in Task 7 Step 1, which replaces these
stubs with real behaviour; add the `unimplemented!()` versions here and
let Task 7 replace them.

Note that the crates which *consume* the trait — `crf`, `structures`,
`sdtm-project`, `mission`'s production code — hold an
`Arc<dyn apis::project::ProjectService>` and call methods on it. Adding
methods to a trait does not break its consumers, so those files are
untouched.

- [ ] **Step 8: Update the server's project wiring**

In `apps/server/aegis-server/src/run.rs`, change `build_project_service`:

```rust
fn build_project_service(
    pool: PgPool,
    user: Arc<dyn apis::user::UserService>,
) -> Arc<dyn apis::project::ProjectService> {
    let project_repo = project::ProjectRepo::new(pool.clone());
    let branch_repo = project::ProjectBranchRepo::new(pool);
    let users = project::UserServiceImpl::new(user);
    let usecase = project::ProjectUsecase::new(project::ProjectUsecaseConfig {
        project_repo,
        users,
    });
    let branch_usecase = project::ProjectBranchUsecase::new(branch_repo);
    Arc::new(project::ProjectServiceImpl::new(usecase, branch_usecase))
}
```

- [ ] **Step 9: Run the whole workspace's checks**

Run:

```bash
cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
```

Expected: PASS — every crate compiles, the four new trait methods are
implemented by all four implementations, no warnings, no formatting diff.

- [ ] **Step 10: Commit**

```bash
git add lib/crates/apis lib/crates/project apps/server
git commit -m "feat(project): add the branch methods to ProjectService

The port gains four branch methods. The crate's facade holds a second
usecase and a third generic parameter for it; every other implementation
of the trait (the server's mock and its two null doubles) gains the
matching stubs.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 7: The server's HTTP surface

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (OpenAPI assertions)

**Interfaces:**
- Consumes: the four `apis::project::ProjectService` branch methods (Task 6).
- Produces: four routes —
  `POST /api/project/{code}/branches`,
  `GET /api/project/{code}/branches`,
  `GET /api/project/{code}/branches/{id}`,
  `PATCH /api/project/{code}/branches/{id}` —
  and the wire DTOs `CreateProjectBranchRequest`, `UpdateProjectBranchRequest`,
  `ProjectBranchViewResponse`, `ProjectBranchListResponse`.

- [ ] **Step 1: Write the failing handler test**

In `apps/server/aegis-server/src/transport/http/project/handlers.rs`, inside
the existing `mod tests`, extend `MockProjectService` with the fields and
behaviour the branch handlers need:

```rust
    pub struct MockProjectService {
        // … the existing fields …
        pub create_project_branch: Option<apis::project::ProjectBranchView>,
        pub create_project_branch_err: Option<apis::project::ProjectApiError>,
        pub get_project_branch_by_id: Option<apis::project::ProjectBranchView>,
        pub get_project_branch_by_id_err: Option<apis::project::ProjectApiError>,
        pub list_project_branches: Option<Vec<apis::project::ProjectBranchView>>,
        pub list_project_branches_err: Option<apis::project::ProjectApiError>,
        pub update_project_branch: Option<apis::project::ProjectBranchView>,
        pub update_project_branch_err: Option<apis::project::ProjectApiError>,
        pub last_update_project_branch_args:
            Arc<Mutex<Option<apis::project::UpdateProjectBranchRequest>>>,
    }
```

Replace the four `unimplemented!()` stubs from Task 6 with:

```rust
        async fn create_project_branch(
            &self,
            _req: apis::project::CreateProjectBranchRequest,
        ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
            if let Some(err) = self.create_project_branch_err.clone() {
                return Err(err);
            }
            Ok(self
                .create_project_branch
                .clone()
                .expect("create_project_branch result configured"))
        }
        async fn get_project_branch_by_id(
            &self,
            id: i32,
        ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
            if let Some(err) = self.get_project_branch_by_id_err.clone() {
                return Err(err);
            }
            Ok(self
                .get_project_branch_by_id
                .clone()
                .expect("get_project_branch_by_id result configured"))
        }
        async fn list_project_branches(
            &self,
            _project_id: i32,
        ) -> Result<Vec<apis::project::ProjectBranchView>, apis::project::ProjectApiError> {
            if let Some(err) = self.list_project_branches_err.clone() {
                return Err(err);
            }
            Ok(self
                .list_project_branches
                .clone()
                .expect("list_project_branches result configured"))
        }
        async fn update_project_branch(
            &self,
            req: apis::project::UpdateProjectBranchRequest,
        ) -> Result<apis::project::ProjectBranchView, apis::project::ProjectApiError> {
            *self.last_update_project_branch_args.lock().unwrap() = Some(req);
            if let Some(err) = self.update_project_branch_err.clone() {
                return Err(err);
            }
            Ok(self
                .update_project_branch
                .clone()
                .expect("update_project_branch result configured"))
        }
```

Add a fixture and the tests:

```rust
    fn sample_branch_view(id: i32, project_id: i32, branch: &str) -> apis::project::ProjectBranchView {
        apis::project::ProjectBranchView {
            id,
            project_id,
            branch: branch.to_string(),
            active: false,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[tokio::test]
    async fn create_project_branch_returns_201() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            create_project_branch: Some(sample_branch_view(4, 9, "2026Q1")),
            ..Default::default()
        };
        let app = app(test_state(project.clone(), MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project/pr1/branches",
                Some(r#"{"branch":"2026Q1","active":true}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CREATED);
        assert_eq!(body["branch"], "2026Q1");
    }

    #[tokio::test]
    async fn create_project_branch_general_returns_403() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            ..Default::default()
        };
        let auth = MockAuth {
            role: apis::user::Role::General,
        };
        let app = app(test_state(project, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project/pr1/branches",
                Some(r#"{"branch":"2026Q1"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");
    }

    #[tokio::test]
    async fn list_project_branches_returns_200_with_array() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            list_project_branches: Some(vec![sample_branch_view(4, 9, "2026Q1")]),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request("GET", "/api/project/pr1/branches", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let branches = body["branches"].as_array().unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0]["branch"], "2026Q1");
    }

    #[tokio::test]
    async fn get_project_branch_of_another_project_returns_404() {
        // The branch exists, but it belongs to project 12 while the
        // URL names project 9. The handler must not leak it.
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            get_project_branch_by_id: Some(sample_branch_view(4, 12, "2026Q1")),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/project/pr1/branches/4",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "branch_not_found");
    }

    #[tokio::test]
    async fn update_project_branch_resolves_the_project_and_forwards_the_flag() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            update_project_branch: Some(sample_branch_view(4, 9, "2026Q1")),
            ..Default::default()
        };
        let app = app(test_state(project.clone(), MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/project/pr1/branches/4",
                Some(r#"{"active":true}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, _body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let captured = project
            .last_update_project_branch_args
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        assert_eq!(captured.id, 4);
        assert_eq!(captured.project_id, 9);
        assert_eq!(captured.active, Some(true));
        assert!(captured.branch.is_none());
    }

    #[tokio::test]
    async fn create_project_branch_duplicate_maps_to_409() {
        let project = MockProjectService {
            get_project_by_code: Some(sample_project_view(9, "pr1")),
            create_project_branch_err: Some(apis::project::ProjectApiError::DuplicateBranch(
                "2026Q1".into(),
            )),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project/pr1/branches",
                Some(r#"{"branch":"2026Q1"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CONFLICT);
        assert_eq!(body["code"], "duplicate_branch");
    }

    #[tokio::test]
    async fn create_project_branch_unknown_project_returns_404() {
        let project = MockProjectService {
            get_project_by_code_err: Some(apis::project::ProjectApiError::NotFound),
            ..Default::default()
        };
        let app = app(test_state(project, MockAuth::default()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/project/nope/branches",
                Some(r#"{"branch":"2026Q1"}"#.to_string()),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }
```

Extend the test app's router so the branch routes are reachable:

```rust
    pub fn app(state: AppState) -> Router {
        Router::new()
            .route("/api/project", post(create_project).get(list_projects))
            .route(
                "/api/project/{code}",
                get(get_project_by_code).patch(update_project),
            )
            .route(
                "/api/project/{code}/branches",
                post(create_project_branch).get(list_project_branches),
            )
            .route(
                "/api/project/{code}/branches/{id}",
                get(get_project_branch_by_id).patch(update_project_branch),
            )
            .with_state(state)
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p aegis-server project_branch
```

Expected: FAIL — `cannot find function create_project_branch in this scope`.

- [ ] **Step 3: Add the wire DTOs**

In `apps/server/aegis-server/src/transport/http/dto.rs`, append:

```rust
// -- project branch DTOs ------------------------------------------------

/// Wire-level extractor for the `{code}` and `{id}` URL parameters of
/// the branch routes.
///
/// One extractor, not two: axum's `Path` deserializes from the whole
/// parameter map, so a bare `Path<i32>` cannot pick a single parameter
/// out of `{code, id}`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathProjectBranch {
    pub code: String,
    pub id: i32,
}

/// Wire-level request body for `POST /api/project/{code}/branches`.
///
/// The project is named by the URL, not by the body, so the body
/// carries only the branch name and whether it becomes the project's
/// active branch. `active` defaults to false when absent.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectBranchRequest {
    pub branch: String,
    #[serde(default)]
    pub active: bool,
}

/// Wire-level request body for `PATCH /api/project/{code}/branches/{id}`.
///
/// Every field is optional: absent `branch` leaves the name alone,
/// absent `active` leaves the flag alone.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectBranchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

/// Wire-level projection of a project branch. Mirrors
/// `apis::project::ProjectBranchView` field-for-field.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBranchViewResponse {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::project::ProjectBranchView> for ProjectBranchViewResponse {
    fn from(view: apis::project::ProjectBranchView) -> Self {
        Self {
            id: view.id,
            project_id: view.project_id,
            branch: view.branch,
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

/// Wire-level wrapper for `GET /api/project/{code}/branches`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBranchListResponse {
    pub branches: Vec<ProjectBranchViewResponse>,
}
```

- [ ] **Step 4: Add the handlers**

In `apps/server/aegis-server/src/transport/http/project/handlers.rs`, add
the four handlers. Each one resolves `{code}` to a project id through
`state.project.get_project_by_code(&code)`.

Extend the file's `use crate::transport::http::dto::{self, PathCode};` import
to `use crate::transport::http::dto::{self, PathCode, PathProjectBranch};`.

```rust
/// `POST /api/project/{code}/branches` — create a branch of a project.
#[utoipa::path(
    post, path = "/{code}/branches", tag = "project",
    operation_id = "project_branch_create",
    params(
        ("code" = String, Path, description = "Project code that owns the branch"),
    ),
    request_body = dto::CreateProjectBranchRequest,
    responses(
        (status = 201, description = "Branch created", body = dto::ProjectBranchViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Branch name already used in this project", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_project_branch(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
    Json(req): Json<dto::CreateProjectBranchRequest>,
) -> Result<(StatusCode, Json<dto::ProjectBranchViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let project_id = state.project.get_project_by_code(&code).await?.id;
    let view = state
        .project
        .create_project_branch(apis::project::CreateProjectBranchRequest {
            project_id,
            branch: req.branch,
            active: req.active,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/project/{code}/branches` — list a project's branches.
#[utoipa::path(
    get, path = "/{code}/branches", tag = "project",
    operation_id = "project_branch_list",
    params(
        ("code" = String, Path, description = "Project code to list branches for"),
    ),
    responses(
        (status = 200, description = "Branches list", body = dto::ProjectBranchListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_project_branches(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
) -> Result<Json<dto::ProjectBranchListResponse>, ApiError> {
    let project_id = state.project.get_project_by_code(&code).await?.id;
    let views = state.project.list_project_branches(project_id).await?;
    let branches = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::ProjectBranchListResponse { branches }))
}

/// `GET /api/project/{code}/branches/{id}` — fetch one branch.
///
/// A branch that belongs to another project is reported as not found,
/// so the response never leaks a branch outside the named project.
#[utoipa::path(
    get, path = "/{code}/branches/{id}", tag = "project",
    operation_id = "project_branch_get",
    params(
        ("code" = String, Path, description = "Project code that owns the branch"),
        ("id" = i32, Path, description = "Branch id"),
    ),
    responses(
        (status = 200, description = "Branch found", body = dto::ProjectBranchViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project or branch not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_project_branch_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathProjectBranch { code, id }): Path<PathProjectBranch>,
) -> Result<Json<dto::ProjectBranchViewResponse>, ApiError> {
    let project_id = state.project.get_project_by_code(&code).await?.id;
    let view = state.project.get_project_branch_by_id(id).await?;
    if view.project_id != project_id {
        return Err(apis::project::ProjectApiError::BranchNotFound(id).into());
    }
    Ok(Json(view.into()))
}

/// `PATCH /api/project/{code}/branches/{id}` — rename a branch and / or
/// change whether it is the project's active branch.
///
/// Setting `active: true` switches: the branch becomes active and the
/// project's other branches are deactivated.
#[utoipa::path(
    patch, path = "/{code}/branches/{id}", tag = "project",
    operation_id = "project_branch_update",
    params(
        ("code" = String, Path, description = "Project code that owns the branch"),
        ("id" = i32, Path, description = "Branch id"),
    ),
    request_body = dto::UpdateProjectBranchRequest,
    responses(
        (status = 200, description = "Branch updated", body = dto::ProjectBranchViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project or branch not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Branch name already used in this project", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_project_branch(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathProjectBranch { code, id }): Path<PathProjectBranch>,
    Json(req): Json<dto::UpdateProjectBranchRequest>,
) -> Result<Json<dto::ProjectBranchViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let project_id = state.project.get_project_by_code(&code).await?.id;
    let view = state
        .project
        .update_project_branch(apis::project::UpdateProjectBranchRequest {
            id,
            project_id,
            branch: req.branch,
            active: req.active,
        })
        .await?;
    Ok(Json(view.into()))
}
```

- [ ] **Step 5: Register the routes**

In `apps/server/aegis-server/src/transport/http/project/router.rs`, extend
the router:

```rust
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create_project))
        .routes(routes!(handlers::list_projects))
        .routes(routes!(handlers::get_project_by_code))
        .routes(routes!(handlers::update_project))
        .routes(routes!(handlers::create_project_branch))
        .routes(routes!(handlers::list_project_branches))
        .routes(routes!(handlers::get_project_branch_by_id))
        .routes(routes!(handlers::update_project_branch))
}
```

- [ ] **Step 6: Map the new error variants**

In `apps/server/aegis-server/src/transport/http/error.rs`, extend
`project_status` and `project_code`:

```rust
        ProjectApiError::BranchNotFound(_) => StatusCode::NOT_FOUND,
        ProjectApiError::DuplicateBranch(_) => StatusCode::CONFLICT,
```

```rust
        ProjectApiError::BranchNotFound(_) => "branch_not_found",
        ProjectApiError::DuplicateBranch(_) => "duplicate_branch",
```

Add the mapping tests next to the other `ProjectApiError` tests:

```rust
    #[tokio::test]
    async fn project_branch_not_found_maps_to_404() {
        let (status, body) = render_project(apis::project::ProjectApiError::BranchNotFound(7)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "branch_not_found");
    }

    #[tokio::test]
    async fn project_duplicate_branch_maps_to_409() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::DuplicateBranch("2026Q1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_branch");
    }
```

- [ ] **Step 7: Run the handler tests to verify they pass**

Run:

```bash
cargo test -p aegis-server project_branch
```

Expected: PASS — 7 new handler tests plus the two error-mapping tests.

- [ ] **Step 8: Assert the routes in the OpenAPI document**

In `apps/server/aegis-server/src/transport/http/router.rs`, extend the
`/api/project` assertions in `openapi_json_returns_200_with_valid_doc`:

```rust
        for (method, path) in [
            ("post", "/api/project/{code}/branches"),
            ("get", "/api/project/{code}/branches"),
            ("get", "/api/project/{code}/branches/{id}"),
            ("patch", "/api/project/{code}/branches/{id}"),
        ] {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }
        for (method, path) in [
            ("post", "/api/project/{code}/branches"),
            ("patch", "/api/project/{code}/branches/{id}"),
        ] {
            let op = &doc["paths"][path][method];
            let response_keys: Vec<&str> = op["responses"]
                .as_object()
                .expect("responses object")
                .keys()
                .map(|s| s.as_str())
                .collect();
            assert!(
                response_keys.contains(&"403"),
                "{method} {path} must advertise a 403 response (got {response_keys:?})",
            );
        }
```

- [ ] **Step 9: Run the server's checks**

Run:

```bash
cargo test -p aegis-server && cargo clippy -p aegis-server --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
```

Expected: PASS, no warnings, no formatting diff.

- [ ] **Step 10: Commit**

```bash
git add apps/server
git commit -m "feat(server): add the project branch routes

Four routes under /api/project/{code}/branches. Reads are open to any
authenticated caller, writes go through the admin-or-root guard. A branch
that belongs to another project is reported as not found, so the named
project is the boundary the response respects.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 8: Live-database tests for the branch repository

**Files:**
- Modify: `lib/crates/project/tests/integration_persistence.rs`

**Interfaces:**
- Consumes: `ProjectBranchRepo` (Task 4), the `project_branches` table
  (Task 1).
- Produces: no new production API — this task is test-only.

- [ ] **Step 1: Extend the destructive cleanup**

In `lib/crates/project/tests/integration_persistence.rs`, add the branch
table to the cleanup in `with_pool`, **before** the `projects` drop (the
branch table references `projects`):

```rust
    sqlx::query("DROP TABLE IF EXISTS project_branches CASCADE")
        .execute(&pool)
        .await
        .expect("drop project_branches");
```

- [ ] **Step 2: Write the live-DB tests**

Append to the same file:

```rust
#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_create_then_activate_switches_the_active_branch() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-branch"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");

        let first = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "2026Q1".into(),
                active: true,
            })
            .await
            .expect("create first branch");
        assert!(first.active);

        // Creating a second *active* branch switches: the first one
        // must be deactivated.
        let second = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "2026Q2".into(),
                active: true,
            })
            .await
            .expect("create second branch");
        assert!(second.active);

        let listed = branches
            .list_by_project(project.id)
            .await
            .expect("list branches");
        assert_eq!(listed.len(), 2);
        let active: Vec<&ProjectBranch> = listed.iter().filter(|b| b.active).collect();
        assert_eq!(active.len(), 1, "at most one branch may be active");
        assert_eq!(active[0].branch, "2026Q2");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_activate_via_update_switches_the_active_branch() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-switch"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");

        let first = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "a".into(),
                active: false,
            })
            .await
            .expect("create a");
        let second = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "b".into(),
                active: false,
            })
            .await
            .expect("create b");

        branches
            .update(ProjectBranchUpdate {
                id: first.id,
                project_id: project.id,
                branch: None,
                active: Some(true),
            })
            .await
            .expect("activate a");
        // Activating the branch that is already active is a no-op, not
        // an error.
        let again = branches
            .update(ProjectBranchUpdate {
                id: first.id,
                project_id: project.id,
                branch: None,
                active: Some(true),
            })
            .await
            .expect("activate a again");
        assert!(again.active);

        let updated = branches
            .update(ProjectBranchUpdate {
                id: second.id,
                project_id: project.id,
                branch: None,
                active: Some(true),
            })
            .await
            .expect("activate b");
        assert!(updated.active);

        let listed = branches
            .list_by_project(project.id)
            .await
            .expect("list branches");
        let active: Vec<&ProjectBranch> = listed.iter().filter(|b| b.active).collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, second.id);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_rename_to_its_own_name_succeeds() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-rename"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");

        let branch = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "keep".into(),
                active: false,
            })
            .await
            .expect("create branch");

        let renamed = branches
            .update(ProjectBranchUpdate {
                id: branch.id,
                project_id: project.id,
                branch: Some("keep".into()),
                active: None,
            })
            .await
            .expect("rename to the same name");
        assert_eq!(renamed.branch, "keep");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_name_is_unique_within_a_project_but_free_across_projects() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-dup"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");
        let other = projects
            .create(ProjectNew {
                code: unique_code("proj-dup"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create other project");

        branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "shared".into(),
                active: false,
            })
            .await
            .expect("create branch");

        // The same name under another project is legal.
        branches
            .create(ProjectBranchNew {
                project_id: other.id,
                branch: "shared".into(),
                active: false,
            })
            .await
            .expect("create branch under the other project");

        // The same name under the same project is not.
        let err = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "shared".into(),
                active: false,
            })
            .await
            .expect_err("duplicate branch name");
        assert!(
            matches!(err, DomainError::DuplicateBranch(_)),
            "expected DuplicateBranch, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_of_another_project_is_reported_as_not_found() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let branches = ProjectBranchRepo::new(pool.clone());
        let project = projects
            .create(ProjectNew {
                code: unique_code("proj-owner"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create project");
        let other = projects
            .create(ProjectNew {
                code: unique_code("proj-owner"),
                description: "".into(),
                members: None,
                unblind_members: None,
                configuration: None,
            })
            .await
            .expect("create other project");
        let branch = branches
            .create(ProjectBranchNew {
                project_id: project.id,
                branch: "mine".into(),
                active: false,
            })
            .await
            .expect("create branch");

        let err = branches
            .update(ProjectBranchUpdate {
                id: branch.id,
                project_id: other.id,
                branch: Some("stolen".into()),
                active: None,
            })
            .await
            .expect_err("cross-project write");
        assert!(
            matches!(err, DomainError::BranchNotFound(_)),
            "expected BranchNotFound, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn branch_for_an_unknown_project_is_rejected() {
    with_pool(|pool| async move {
        let branches = ProjectBranchRepo::new(pool);
        let err = branches
            .create(ProjectBranchNew {
                project_id: 999_999_999,
                branch: "orphan".into(),
                active: false,
            })
            .await
            .expect_err("foreign key violation");
        assert!(
            !matches!(err, DomainError::DuplicateBranch(_)),
            "expected a foreign-key failure, got {err:?}"
        );
    })
    .await;
}
```

- [ ] **Step 3: Extend the file's imports**

At the top of `lib/crates/project/tests/integration_persistence.rs`:

```rust
use project::domain::{
    DomainError, ProjectBranch, ProjectBranchNew, ProjectBranchRepository, ProjectBranchUpdate,
    ProjectConfiguration, ProjectMember, ProjectNew, ProjectTag, ProjectUpdate,
};
use project::{ProjectBranchRepo, ProjectRepo, ProjectRepository};
```

- [ ] **Step 4: Run the live-DB tests**

Run:

```bash
cargo test -p project -- --ignored --test-threads=1
```

Expected: PASS — the six new branch tests plus the four existing project
tests. (This drops and recreates the tables in the database named by
`AEGIS_DATABASE_URL`; that is the suite's documented, intentional
behaviour.)

- [ ] **Step 5: Commit**

```bash
git add lib/crates/project/tests/integration_persistence.rs
git commit -m "test(project): cover the branch repository against a live database

Pins the switch rule on create and on update, the rename-to-its-own-name
case, name uniqueness within a project but not across projects, the
cross-project write refusal, and the foreign-key rejection of a branch
for an unknown project.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 9: The crate README

**Files:**
- Modify: `lib/crates/project/README.md`

**Interfaces:**
- Consumes: everything built in Tasks 1-8.
- Produces: no new code — documentation only.

- [ ] **Step 1: Update the layout section**

In `lib/crates/project/README.md`, extend the `src/` tree so it mentions
the new modules:

```text
src/
  domain/                       # Project, ProjectMember, ProjectBranch, TeamType, RoleType,
                                # UserSummary, DomainError, and the ports
                                # (ProjectRepository, ProjectBranchRepository, UserService)
  usecase/                      # ProjectUsecase (combines product + project flows),
                                # ProjectBranchUsecase, command DTOs (Create*/Update*),
                                # view DTOs, errors
  adapter/
    service/
      user/                     # UserServiceImpl — adapts apis::user::UserService to
                                # the narrow domain UserService port
    persistence/
      postgres/                 # SQLx-backed ProjectRepo, ProjectBranchRepo
    facade/
      internal/                 # ProjectServiceImpl wiring usecase ->
                                # apis::project::ProjectService
migrations/                     # SQLx migrations applied to the database
```

- [ ] **Step 2: Add the branch to the domain model section**

Append to the "Domain model" section:

```markdown
- `ProjectBranch { id, project_id, branch, active, created_at, updated_at }` — a named
  working branch of a project. The name is unique within its project (the same name may
  exist under two different projects). At most one branch of a project is active at a
  time: activating a branch *switches* — it becomes active and the project's other
  branches are deactivated in the same transaction. The rule is backed by a partial
  unique index (`project_branches_one_active_per_project`). A project may have zero
  branches, and branches are never deleted.
```

- [ ] **Step 3: Update the migration count and the constructor snippet**

The "Database setup" section says the crate ships "two SQLx migrations"; it
now ships five (`projects` + `project_members`, the three `configuration`
migrations, and `project_branches`). Change that sentence to "five SQLx
migrations".

In the constructor snippet, add the branch repository and the second usecase:

```rust
let project_repo = ProjectRepo::new(pool.clone());
let branch_repo = ProjectBranchRepo::new(pool.clone());
```

```rust
let branch_usecase = ProjectBranchUsecase::new(branch_repo);
let project_service: Arc<dyn apis::project::ProjectService> =
    Arc::new(ProjectServiceImpl::new(usecase, branch_usecase));
```

- [ ] **Step 4: Update the "Tests, by layer" section**

Extend the list so it mentions the branch coverage:

```markdown
6. **integration_persistence** — `tests/integration_persistence.rs`:
   live PostgreSQL, `#[ignore]`-gated. Covers the project surface and
   the branch repository (the switch rule, name uniqueness, the
   cross-project write refusal).
```

- [ ] **Step 5: Commit**

```bash
git add lib/crates/project/README.md
git commit -m "docs(project): document the branch model in the crate README

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Final verification

Run after Task 9:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc -p project --no-deps
cargo test -p project -- --ignored --test-threads=1
```

Expected: everything passes. The live-DB run needs `AEGIS_DATABASE_URL`
pointing at a reachable PostgreSQL — it is present in the workspace-root
`.env`.
