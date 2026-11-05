# terminology

CRUD over the CDISC terminology aggregates
(`TerminologyVersion`, `CodeList`, `CodeItem`) with full-text
search, backed by PostgreSQL.

This crate is a business lib crate; see
`docs/guidelines/lib-crate-development.md` for the cross-cutting
conventions (workspace wiring, DDD layout, error chain, the
five-tier test rule) and
`docs/superpowers/specs/2026-08-18-terminology-crate-design.md`
for the data model + port surface.

## Source layout

    src/
    ├── lib.rs                                  # pub mod + re-exports
    ├── domain.rs                               # children, pub use
    ├── domain/
    │   ├── terminology_kind.rs                 # SDTM | ADAM enum
    │   ├── terminology_version.rs             # aggregate + DTOs
    │   ├── code_list.rs                        # aggregate + DTOs + search
    │   ├── code_item.rs                        # aggregate + DTOs + search
    │   ├── repository.rs                       # the three #[async_trait] ports
    │   ├── error.rs                            # DomainError
    │   └── tests.rs                            # domain unit tests
    ├── usecase.rs
    ├── usecase/
    │   ├── commands.rs                         # Create*/Update* DTOs
    │   ├── views.rs                            # *View DTOs + From impls
    │   ├── error.rs                            # UsecaseError + From<DomainError>
    │   ├── terminology_usecase.rs              # TerminologyUsecase<V, L, I>
    │   └── tests.rs                            # in-memory wire-up tests
    ├── adapter.rs
    └── adapter/
        ├── persistence.rs
        └── persistence/postgres/
            ├── postgres.rs                     # module index, re-exports
            ├── terminology_version_repo.rs
            ├── code_list_repo.rs
            └── code_item_repo.rs

## Database setup

Migrations live under `migrations/` and are applied via
`sqlx migrate run --source lib/crates/terminology/migrations`.

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/terminology/migrations --timestamp <description>
```

The runtime URL comes from the server's `AEGIS_DATABASE_URL`. The
live-DB integration tests use the disposable `AEGIS_TEST_DATABASE_URL`
(or `.env` at the workspace root).

```rust
use sqlx::postgres::PgPoolOptions;
use terminology::{
    CodeItemRepo, CodeListRepo, TerminologyUsecase, TerminologyUsecaseConfig,
    TerminologyVersionRepo,
};

let pool = PgPoolOptions::new()
    .connect(&std::env::var("AEGIS_DATABASE_URL")?)
    .await?;

let v_repo = TerminologyVersionRepo::new(pool.clone());
let l_repo = CodeListRepo::new(pool.clone());
let i_repo = CodeItemRepo::new(pool.clone());

let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
    version_repo: v_repo,
    code_list_repo: l_repo,
    code_item_repo: i_repo,
});
```

## Tests

```bash
cargo test -p terminology                                # cargo unit + ignored-free tests
cargo test -p terminology -- --ignored --test-threads=1  # when AEGIS_TEST_DATABASE_URL is set
```

## Guideline

See `docs/guidelines/lib-crate-development.md` for the
cross-cutting conventions every lib crate in this workspace
follows.