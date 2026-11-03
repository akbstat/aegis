# sdtm-project

Owns the **per-project SDTM model tree** — the domains and variables a project
declares on top of the CDISC SDTM standard — behind five PostgreSQL tables and
an `apis::sdtm_project::SdtmProjectService` port.

The crate deliberately shares no types with `domain-model`: the CDISC standard
and a project's overlay are allowed to drift independently, so the value objects
(`DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole` and the
description DTOs) are **copied** rather than imported.

## Layout

```
src/
├── lib.rs                        # three layer modules + crate-root re-exports
├── domain.rs                     # declares + re-exports the domain children
├── domain/
│   ├── domain_category.rs        # copied from domain-model
│   ├── variable_type.rs          # copied from domain-model
│   ├── sdtm_domain_description.rs
│   ├── sdtm_variable_description.rs
│   ├── sdtm_project_version.rs
│   ├── sdtm_project_domain.rs
│   ├── sdtm_project_domain_override.rs
│   ├── sdtm_project_variable.rs
│   ├── sdtm_project_variable_override.rs
│   ├── project_lookup.rs         # cross-crate port onto the `project` crate
│   ├── repository.rs             # the five repository ports
│   ├── error.rs                  # DomainError
│   └── tests.rs
├── usecase.rs
├── usecase/
│   ├── commands.rs               # ten Create/Update command DTOs
│   ├── error.rs                  # UsecaseError
│   ├── views.rs                  # five view DTOs + From impls
│   ├── sdtm_project_usecase.rs   # the usecase + its config
│   └── tests.rs
└── adapter.rs
└── adapter/
    ├── persistence.rs
    ├── persistence/postgres.rs
    ├── persistence/postgres/     # one *RepoPg per port
    ├── service.rs
    ├── service/project/          # ProjectLookupImpl -> apis::project::ProjectService
    ├── facade.rs
    └── facade/internal/          # SdtmProjectServiceImpl -> the apis port
```

## The data model

| Aggregate | Table | Uniqueness |
| --- | --- | --- |
| `SdtmProjectVersion` | `sdtm_project_versions` | `(project_code, version_name)` |
| `SdtmProjectDomain` | `sdtm_project_domains` | `(version_id, domain_code)` |
| `SdtmProjectDomainOverride` | `sdtm_project_domain_overrides` | one per domain |
| `SdtmProjectVariable` | `sdtm_project_variables` | `(domain_id, variable_code)` |
| `SdtmProjectVariableOverride` | `sdtm_project_variable_overrides` | one per variable |

`project_code` is a **soft reference** to `projects.code` in the `project`
crate. It is validated at the application level through a
`domain::ProjectLookup` port, not by a database foreign key, so the two
crates' schemas stay independent and either can be migrated first.

`key_variables` is an **ordered** list: the order the caller supplies is
preserved through the JSONB column and out to the wire. Duplicates collapse to
their first occurrence; nothing is ever re-sorted.

## Database setup

The server does not auto-migrate. To create the five `sdtm_project_*` tables
in the server's database:

```bash
export AEGIS_DATABASE_URL=postgres://user:pass@localhost/aegis
sqlx migrate run --source lib/crates/sdtm-project/migrations
```

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/sdtm-project/migrations --timestamp <description>
```

### Running the live tests

```bash
export AEGIS_SDTM_PROJECT_DATABASE_URL=postgres://user:pass@localhost/aegis_sdtm_project_test
cargo test -p sdtm-project -- --ignored --test-threads=1
```

> **The URL must point at a DEDICATED database.**
>
> The live test drops the `_sqlx_migrations` bookkeeping table so each run
> starts clean. SQLx keeps exactly one `_sqlx_migrations` table *per database*,
> shared by every crate that migrates it — so pointing this at the server's
> database would erase the applied-migration record for auth, user, project,
> terminology, mission and crf alike, and the next `sqlx migrate run` would
> try to re-apply every one of them.
>
> The test is destructive against whatever database it is pointed at. That is
> intentional.

The test panics with a clear message when the variable is unset — it never
silently skips.

## Constructing the service

```rust
use std::sync::Arc;

use sdtm_project::{
    ProjectLookupImpl, SdtmProjectDomainOverrideRepoPg, SdtmProjectDomainRepoPg,
    SdtmProjectServiceImpl, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
    SdtmProjectVariableOverrideRepoPg, SdtmProjectVariableRepoPg, SdtmProjectVersionRepoPg,
};

let usecase = SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
    version_repo: SdtmProjectVersionRepoPg::new(pool.clone()),
    domain_repo: SdtmProjectDomainRepoPg::new(pool.clone()),
    domain_override_repo: SdtmProjectDomainOverrideRepoPg::new(pool.clone()),
    variable_repo: SdtmProjectVariableRepoPg::new(pool.clone()),
    variable_override_repo: SdtmProjectVariableOverrideRepoPg::new(pool),
    projects: Arc::new(ProjectLookupImpl::new(project_service)),
});
let service = SdtmProjectServiceImpl::new(usecase);
```

`project_service` is an `Arc<dyn apis::project::ProjectService>` — the same
service the server already builds — so the project-existence check reuses the
canonical `projects` table.

## Optional overrides

An override is **optional**: a domain or variable that has never been
overridden is a normal state, not a broken one. So `get_domain_override` and
`get_variable_override` return `Result<Option<…>, _>` — the repository still
reports a missing row as `SdtmProject{Domain,Variable}OverrideNotFound`, but
the usecase turns exactly that variant into `Ok(None)`.

On the wire, `GET …/override` on a domain or variable with no override is
**200 with `{"override": null}`**, not 404. The response is always an
envelope with an `override` key — `null` when there is no override, the
override itself when there is one — so a client reads `body.override`
without first null-checking the body.

`update_*_override` and `delete_*_override` keep their 404: asking to change
or remove something that does not exist is still a client error.

## Conventions

This crate follows
[`docs/guidelines/lib-crate-development.md`](../../docs/guidelines/lib-crate-development.md):
DDD layering (`domain` → `usecase` → `adapter`), ports-and-adapters with every
I/O behind a `domain` port, one error enum per layer, and a `README.md` per
crate.
