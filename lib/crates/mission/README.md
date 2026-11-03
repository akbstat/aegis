# mission

Lifecycle for project-scoped `Mission` aggregates, their
`Assignee` join rows, and the `MissionIssue` thread attached to
each mission. Each mission belongs to exactly one project,
carries a `MissionKind` (`crf` | `sdtm` | `adam` | `tfl`), and
names a human-readable `mission_code` (e.g. `CRF-V1`).
Assignees attach users to a mission under a `MissionRole`
(`dev` | `qc`); the `(user_code, role)` pair is unique within a
mission. Each mission has an append-only thread of
`MissionIssue`s, each carrying an `IssueState` (`opened` |
`closed`), an optional `target_item`, an `issuer`, a mutable
`description`, and an ordered `Vec<IssueComment>` thread.

Mission write operations (create mission, delete mission, add
assignee, remove assignee) are gated on the caller being a
*leader* of the owning project — checked via the `project`
business crate's `ProjectLookup::is_leader`. There is no
separate role gate; the authorisation is purely
project-membership-based.

Mission-issue write operations use a finer-grained rule:
- **create / close / reopen** — project leader OR a mission QC
  assignee;
- **update description** — project leader OR a mission QC
  assignee (allowed even on closed issues);
- **append comment** — project leader OR any assignee (DEV or
  QC) on the mission.

This crate is a business lib crate; see
`docs/guidelines/lib-crate-development.md` for the cross-cutting
conventions (workspace wiring, DDD layout, error chain, the
five-tier test rule) and
`docs/superpowers/specs/2026-09-01-mission-crate-design.md` for
the data model + port surface.
`docs/superpowers/specs/2026-09-20-mission-issue-design.md`
covers the issue + comment aggregate.

## Source layout

    src/
    ├── lib.rs                                  # pub mod + re-exports
    ├── domain.rs                               # children, pub use
    ├── domain/
    │   ├── mission_kind.rs                     # Crf | Sdtm | Adam | Tfl
    │   ├── mission_role.rs                     # Dev | Qc
    │   ├── mission.rs                          # aggregate + DTOs
    │   ├── assignee.rs                         # aggregate + DTOs + uniqueness
    │   ├── issue_state.rs                      # Opened | Closed
    │   ├── issue_comment.rs                    # IssueComment (Serialize/Deserialize for jsonb)
    │   ├── issue.rs                            # MissionIssue aggregate
    │   ├── project_lookup.rs                   # is_leader lookup port
    │   ├── user_lookup.rs                      # get_by_code lookup port
    │   ├── mission_lookup.rs                   # MissionRepository port
    │   ├── assignee_lookup.rs                  # AssigneeRepository port (+ is_assignee)
    │   ├── issue_lookup.rs                     # MissionIssueRepository port
    │   ├── error.rs                            # DomainError
    │   └── tests.rs                            # domain unit tests
    ├── usecase.rs
    ├── usecase/
    │   ├── commands.rs                         # Create*/AssigneeData DTOs
    │   ├── views.rs                            # *View DTOs (MissionView + IssueView)
    │   ├── error.rs                            # UsecaseError + From<DomainError>
    │   ├── mission_usecase.rs                  # MissionUsecase<R, A, P, U>
    │   ├── issue_usecase.rs                    # MissionIssueUsecase<R, A, P, I>
    │   └── tests.rs                            # in-memory wire-up tests
    ├── adapter.rs
    ├── adapter/
    │   ├── facade/
    │   │   ├── facade.rs                       # MissionService facade module
    │   │   └── internal/
    │   │       ├── internal.rs                 # module index
    │   │       ├── service.rs                  # MissionServiceImpl composes both usecases
    │   │       └── issue_service.rs            # InMemoryIssueRepo for the facade
    │   ├── persistence.rs
    │   └── persistence/postgres/
    │       ├── postgres.rs                     # module index, re-exports
    │       ├── mission_repo.rs                 # MissionRepository impl
    │       ├── assignee_repo.rs                # AssigneeRepository impl (+ is_assignee)
    │       ├── issue_repo.rs                   # MissionIssueRepository impl
    │       └── row.rs                          # MissionRow + AssigneeRow + IssueRow
    ├── service/
    │   ├── project.rs                          # MissionProjectLookupImpl
    │   └── user.rs                             # MissionUserLookupImpl
    └── test_support.rs                         # in-memory fixtures for tests

## Database setup

Migrations live under `migrations/` and are applied via
`sqlx migrate run --source lib/crates/mission/migrations`.

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/mission/migrations --timestamp <description>
```

The live-DB URL comes from the
`AEGIS_MISSION_DATABASE_URL` environment variable (or `.env` at
the workspace root).

```rust
use sqlx::postgres::PgPoolOptions;
use mission::{
    AssigneeRepo, IssueRepo, MissionRepo, MissionServiceImpl,
};
use mission::service::project::ProjectLookupImpl;
use mission::service::user::UserLookupImpl;
use std::sync::Arc;

let pool = PgPoolOptions::new()
    .connect(&std::env::var("AEGIS_MISSION_DATABASE_URL")?)
    .await?;

// All five repos share the same pool; `from_repos` builds both
// usecases from these and returns a single `MissionServiceImpl`.
// `ProjectLookupImpl` + `UserLookupImpl` adapt the already-wired
// apis `ProjectService` + `UserService` to the mission domain
// ports.
let mission_repo = Arc::new(MissionRepo::new(pool.clone()));
let assignee_repo = Arc::new(AssigneeRepo::new(pool.clone()));
let issue_repo = Arc::new(IssueRepo::new(pool));
let projects = Arc::new(ProjectLookupImpl::new(project_service));
let users = Arc::new(UserLookupImpl::new(user_service));

let service: Arc<dyn apis::mission::MissionService> =
    Arc::new(MissionServiceImpl::from_repos(
        mission_repo,
        assignee_repo,
        projects,
        users,
        issue_repo,
    ));
```

## Mission issues

Each `MissionIssue` is the append-only thread attached to a
mission. The aggregate is hydrated from a single Postgres row:
the comment thread is stored as a `JSONB` column and read back
on every fetch. Issue state changes (`close` / `reopen`) and
description updates are individual row-level writes; comments
are appended via read-modify-write on the JSONB column.

The six new `apis::mission::MissionService` methods are:

- `list_issues_by_mission(mission_id, state: Option<IssueState>)`
- `create_issue(actor, { mission_id, target_item, description })`
- `close_issue(actor, issue_id)`
- `reopen_issue(actor, issue_id)`
- `update_issue_description(actor, issue_id, description)`
- `append_comment(actor, issue_id, content)`

Authorisation is enforced inside
`MissionIssueUsecase::ensure_can_create_or_close` /
`ensure_can_comment` (see `usecase/issue_usecase.rs`). The
state-change and description-update methods require the caller
to be either a project leader or a mission QC assignee; the
comment-append method additionally accepts a mission DEV
assignee.

## Tests

```bash
cargo test -p mission                                  # unit + public_api
cargo test -p mission -- --ignored --test-threads=1    # when AEGIS_MISSION_DATABASE_URL is set
```

The live-DB integration tests are destructive on purpose: they
drop the live `missions`, `assignees`, `mission_issues`, and
`_sqlx_migrations` tables before each run.

## HTTP surface (aegis-server)

The mission crate ships only the business lib; HTTP routes live
in `apps/server/aegis-server/src/transport/http/mission/` and
are mounted at `/api/mission`. See that module for the URL map.

## Guideline

See `docs/guidelines/lib-crate-development.md` for the
cross-cutting conventions every lib crate in this workspace
follows.