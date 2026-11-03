# project crate

Workspace library implementing the `apis::project::ProjectService` port.
Manages the lifecycle of `Product` and `Project` aggregates — each
`Project` belongs to one `Product`, and carries two parallel teams
(`members` and `unblind_members`) that the facade hydrates from the
user-crate's `UserService`.

> See [docs/guidelines/lib-crate-development.md](../../docs/guidelines/lib-crate-development.md)
> for the cross-cutting conventions this crate follows (workspace deps,
> no `mod.rs`, ports-and-adapters layering, ignored live-DB tests, etc.).

## Layout

```text
src/
  domain/                       # Product, Project, ProjectMember, TeamType, RoleType,
                                # UserSummary, DomainError, and the ports
                                # (ProductRepository, ProjectRepository, UserService)
  usecase/                      # ProjectUsecase (combines product + project flows),
                                # command DTOs (Create*/Update*), view DTOs, errors
  adapter/
    service/
      user/                     # UserServiceImpl — adapts apis::user::UserService to
                                # the narrow domain UserService port
    persistence/
      postgres/                 # SQLx-backed ProductRepo, ProjectRepo
    facade/
      internal/                 # ProjectServiceImpl wiring usecase ->
                                # apis::project::ProjectService
migrations/                     # SQLx migrations applied to the database
```

The crate root re-exports the public surface (`Product`, `ProductNew`,
`ProductUpdate`, `Project`, `ProjectNew`, `ProjectUpdate`,
`ProjectConfiguration`, `ProjectLanguage`, `ProjectMember`,
`ProjectTag`, `TeamType`, `RoleType`, `UserSummary`, `UserService`,
`DomainError`, the ports `ProductRepository` / `ProjectRepository`,
the Postgres adapters `ProductRepo` / `ProjectRepo`, the apis
adapter `UserServiceImpl`, the usecase `ProjectUsecase` +
`ProjectUsecaseConfig`, the command DTOs `CreateProduct` /
`UpdateProduct` / `CreateProject` / `UpdateProject`, the view DTOs
`ProductView` / `ProjectView` / `ProjectConfigurationView` /
`ProjectMemberView` / `UserSummaryView`, the error `UsecaseError`,
and the facade `ProjectServiceImpl`) so consumers can
`use project::*;` without reaching into the sub-modules.

## Domain model

- `Product { id, code, name, description, active, created_at, updated_at }`
- `Project { id, code, description, product_id, members, unblind_members, configurations, active, created_at, updated_at }`
- `ProjectConfiguration { language: Option<ProjectLanguage>, tags: Vec<ProjectTag> }` — language hint + tag list, persisted as a single JSONB column on `projects.configuration`. `language` is optional (no default); `tags` defaults to empty.
- `ProjectLanguage { English, SimplifiedChinese }` — locale hint with a stable `as_str` (`"en"`, `"zh-CN"`). Unknown values on the wire surface as `DomainError::UnknownLanguage` after the migration lands.
- `ProjectMember { leaders: Vec<String>, workers: Vec<String> }` — user *codes* (not full user records). The usecase layer hydrates these into `UserSummaryView` on read.

`members` and `unblind_members` are two independent teams that share
the same `leaders` / `workers` shape. `create_project` accepts both as
optional (`None` and `Some(empty)` are equivalent on create). On
update, `None` leaves the team unchanged; `Some(empty)` wipes it
(whole-list replacement).

`configurations` follows the same optionality rules. On create,
`None` and `Some(ProjectConfiguration::default())` are equivalent. On
update, `None` leaves the configuration alone; `Some(_)` replaces the
whole configuration (language + tag list).

## Database setup

The crate ships two SQLx migrations that define `products`,
`projects`, and `project_members`. Apply them before pointing the
repositories at the database:

```bash
sqlx migrate run --source lib/crates/project/migrations
```

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/project/migrations --timestamp <description>
```

Once the migrations are applied, construct the repositories, the
`UserServiceImpl` adapter, the usecase, and the facade from a
`sqlx::PgPool`:

```rust
use std::sync::Arc;

use apis::user::UserService as ApiUserService;
use project::{
    ProductRepo, ProjectRepo, ProjectServiceImpl, ProjectUsecase,
    ProjectUsecaseConfig, UserServiceImpl,
};

let product_repo = ProductRepo::new(pool.clone());
let project_repo = ProjectRepo::new(pool.clone());

let api_user_service: Arc<dyn ApiUserService> = Arc::new(/* from the user crate */);
let users = UserServiceImpl::new(api_user_service);

let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
    product_repo,
    project_repo,
    users,
});

let project_service: Arc<dyn apis::project::ProjectService> =
    Arc::new(ProjectServiceImpl::new(usecase));
```

The `project` crate does not run migrations at runtime; a deployment
step is required.

## `UserService` port

The crate does not manage users directly. It defines a narrow
`project::domain::UserService` port that exposes only what the
usecase needs (`get_by_code`, `list`) and ships an adapter
(`UserServiceImpl`) that delegates to the user crate's full
`apis::user::UserService`. Wire a `UserServiceImpl` into
`ProjectUsecaseConfig::users`; swap it for an alternative
implementation in tests or in a different deployment.

## Integration tests

The crate ships live-database integration tests at
[`tests/integration_persistence.rs`](tests/integration_persistence.rs).
They connect to PostgreSQL, apply both migrations, and exercise the
full repository surface (product CRUD, project CRUD with membership
replacement, project creation with no membership). They are
`#[ignore]`-gated so the default `cargo test -p project` run stays
green without a database.

Run them against a local PostgreSQL with:

```bash
# .env at the workspace root is sourced automatically by the tests
# via dotenvy; AEGIS_DATABASE_URL (the workspace-shared env var,
# same as the auth / user crates) must point at a reachable server
# (and ideally an empty schema — the tests drop their own tables
# at the end of each run).
cargo test -p project -- --ignored
```

## Tests, by layer

The crate enforces the test-tier order from the guideline:

1. **domain** — `src/domain/tests.rs`: invariants, validation,
   duplicate detection in `ProjectMember`.
2. **adapter unit** — `src/adapter/persistence/postgres/tests.rs`:
   row conversions, schema snapshots.
3. **usecase unit** — `src/usecase/tests.rs`: mock repos + user
   service; covers validation paths, optional membership,
   `ProjectConfiguration` (language + tag-list) hydration, and
   whole-list replacement of membership + configuration.
4. **facade unit** — `src/adapter/facade/internal/tests.rs`: in-memory
   repo + user-service fakes; end-to-end CRUD through the apis port,
   including `Box<dyn ProjectService>` object-safe dispatch.
5. **public_api compile-only** — `tests/public_api.rs`: locks the
   documented surface so a rename, removed field, or lost object
   safety surfaces at `cargo test -p project` time.
6. **integration_persistence** — `tests/integration_persistence.rs`:
   live PostgreSQL, `#[ignore]`-gated.