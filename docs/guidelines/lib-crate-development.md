# Business Library Crate Development

Applies to every business lib crate under `lib/crates/` (e.g. `user`, `auth`).
Not every lib crate is a business lib crate — `apis` is a port-defining crate
and uses a different layout; the user will say "this is a business lib crate"
when one is to be added.

Two principles, then specific conventions:

1. **Domain-driven design.** Code is organised by *layer* (`domain`, `usecase`,
   `adapter`) and by *direction of dependency* inside the adapter layer. The
   domain layer never imports infrastructure crates; only `usecase` and
   `adapter` do.
2. **Clean architecture / ports-and-adapters.** All I/O goes through a port
   (trait) declared in `domain`. Concrete implementations live in `adapter`
   and are swappable per backend (e.g. PostgreSQL, in-memory, Redis). The
   `usecase` is generic over ports so tests can inject in-memory fakes.

## 1. Workspace wiring

- Register the crate in the root `Cargo.toml` `[workspace].members` array and
  inherit every shared dep via `{ workspace = true }`.
- Pin `edition = "2024"` and `resolver = "3"` at the workspace root.
- Drop unused deps after `cargo build -p <crate>`. Non-obvious deps (e.g.
  `chrono` for the timestamp column, `apis` for the outbound port the facade
  implements) get a one-line comment explaining *why* they are there.
- Path-deps between sibling workspace crates (`apis = { path = "../apis" }`)
  are allowed; the comment should say so explicitly.

## 2. Module layout (no `mod.rs`)

- `src/<module>.rs` plus a `src/<module>/` directory. `mod.rs` is deprecated
  on the 2024 edition.
- Each layer module (`src/domain.rs`, `src/usecase.rs`, `src/adapter.rs`)
  declares its children with `mod …;` and re-exports the public surface via
  `pub use …;` at the bottom. Consumers write `use crate::Foo`, never reach
  into nested paths.
- Private details are `pub(crate)`; a layer boundary may be `pub(crate)` when
  callers reach concrete types through a re-export at the layer above plus the
  crate root. Children intended for re-export must be `pub` so the upstream
  `pub use` compiles.

## 3. Three DDD layers

```
domain   ← pure types, value objects, ports (traits), domain errors. No I/O,
           no sqlx, no tokio. Validates inputs and enforces invariants.
usecase  ← orchestrates ports. Holds command DTOs, view DTOs, and a
           *UsecaseError that wraps *DomainError. Generic over the repository
           port so tests can inject in-memory fakes. Projects domain → view
           via From impls. Re-validates command inputs before reaching the
           repository.
adapter  ← concrete implementations of domain ports. Sub-organised by the
           *direction* of the dependency, not by feature:
             adapter/persistence/<backend>/   storage adapters (Postgres, …)
             adapter/facade/internal/        outbound-port adapters that
                                              adapt usecase → API-facing
                                              traits in other workspace crates.
                                              One per crate, generic over the
                                              storage ports — so a single impl
                                              serves every backend.
             adapter/cache/<backend>/         cache ports (in-memory, Redis, …)
             adapter/service/<other_port>/    adapt *another* workspace crate's
                                              port into a *domain* port, so the
                                              usecase never reaches the apis crate
```

Add a new backend by adding a sibling directory (`persistence/sqlite/`,
`cache/redis/`, …) and a re-export at the adapter boundary. No edits to
existing backends.

## 4. Public API surface at the crate root

`src/lib.rs` declares the three layers (`pub mod domain; pub mod usecase;
pub mod adapter;`) and then `pub use`s every type, error variant, trait,
command DTO, view DTO, and constructor that a consumer is allowed to name.
The crate-level doc-comment shows the canonical `use` line.

Constructors stay obvious: `UserRepo::new(pool)`, `UserUsecase::new(repo)`,
`AuthServiceImpl::new(usecase)`. No builders, no async constructors.

Sensitive fields are `pub(crate)` and redacted in hand-rolled `Debug` impls.
Today every field on every aggregate is safe to log; the manual `Debug` is
structural rather than redact-and-reveal, but the pattern is reserved for
when a future aggregate gains one.

## 5. Domain aggregates: two constructors

Every aggregate root has two constructors:

- `Foo::new(...)` — the public-facing **validating** constructor. Returns
  `Result<Self, DomainError>` and rejects empty / whitespace inputs. Used by
  tests and any in-crate path that constructs from raw inputs.
- `Foo::for_repository(...)` — `pub(crate)`, reserved for the adapter layer.
  Skips validation because the data is assumed to have been validated on the
  way in. The doc-comment names the rule so future code does not call it
  from outside `adapter::`.

The `FromRow` row bridge calls `for_repository`. Domain invariants are
otherwise enforced in `domain`; the usecase re-validates command inputs;
the adapter translates driver errors into domain errors.

## 6. Errors: one enum per layer, with the chain preserved

- `DomainError` and `UsecaseError` are `#[derive(thiserror::Error)]` enums.
- Every variant that wraps an inner error carries `#[source] Inner` so
  `Error::source()` keeps the chain (`UsecaseError::Repository(#[source] DomainError)`).
- Implement `From<DomainError> for UsecaseError` so the usecase can `?`
  straight through repository calls. The `From` impl maps a domain
  validation error from the repository into `UsecaseError::Repository`
  (not `Validation`), because the contract was already broken upstream.
- Additional layer-specific variants are fine (`UsecaseError::Verification`
  for JWT decode failures, etc.) when no existing variant fits.
- Adapter `map_*_error` is the only place that knows about a driver's error
  enum. `sqlx::Error::RowNotFound → DomainError::NotFound`;
  `sqlx::Error::Database` with SQLSTATE `23505` →
  `DomainError::DuplicateCode(constraint_name)`; everything else →
  `DomainError::Repository(driver_message)`.
- Never panic on I/O failure.

## 7. Async and object safety

- Port traits that must be object-safe (`Box<dyn Repo>` for injection,
  `Arc<dyn Facade>` for cross-crate wiring) get `#[async_trait]` from a
  workspace dep and a `Send + Sync` bound.
- Concrete types use plain `pub async fn`.
- Configuration for a usecase that has too many constructor args goes
  through a `pub struct *UsecaseConfig<R, D> { /* pub fields */ }` so the
  call site stays readable without builder ceremony.

## 8. Database schema

- **Never create a migration file by hand.** The file name *is* the
  migration's version and the sqlx CLI owns it. Let the CLI create the
  file first, then write the DDL into the file it created:

  ```bash
  sqlx migrate add --source lib/crates/<crate>/migrations --timestamp <description>
  ```

  The CLI writes `migrations/<UTC timestamp>_<description>.sql` and
  prints the path. Put the DDL in that file. Do not rename it, do not
  hand-roll the version prefix, do not create the file with an editor
  and then patch in a name.

  Hand-rolled names like `0001_create_users.sql` look harmless but
  change how the CLI behaves afterwards: `sqlx migrate add` infers
  *sequential* versioning whenever the last two migrations differ by
  exactly 1, so the next migration added to a hand-numbered directory
  comes out as `0009_…` instead of a timestamp and the ordering stops
  reflecting when the migration was actually written. Every crate under
  `lib/crates/` uses timestamp versioning — keep it that way. Passing
  `--timestamp` explicitly pins the scheme even if a directory ever
  ends up with a consecutive-looking pair.

  Apply the new migration with
  `sqlx migrate run --source lib/crates/<crate>/migrations` before
  relying on it.

- One migration file per schema change, under `<crate>/migrations/`.
  Consumed via `sqlx::migrate!("./migrations")` so the file inventory is
  the source of truth.
- The migration file name is generated, so anything that reads a
  migration file by path — the `include_str!` calls in the adapter
  tests, the `read_to_string` helpers in §9.2 — has to spell the
  CLI-generated name. Look it up with
  `ls lib/crates/<crate>/migrations`; never guess it.
- SQLx runtime API (`sqlx::query_as`, `QueryBuilder`) until the workspace
  ships a `sqlx-data.json` cache or a live `DATABASE_URL` at build time.
  Document the choice in a module-level comment so the next reviewer can
  switch to compile-time macros when the cache is wired in.
- Auto-managed timestamps: prefer `DEFAULT NOW()` on `created_at` /
  `updated_at` and a `BEFORE UPDATE` trigger that refreshes `updated_at`.
  The trigger covers every code path including direct SQL.
- Enum-shaped columns get a `CHECK` constraint. The Rust `Role::try_from`
  is the source of truth for the allowed values; the CHECK is
  belt-and-braces against out-of-band inserts.

## 9. Tests, in this order

1. **Domain unit tests** in `src/domain/tests.rs`. Cover value-object
   conversions (`Role::try_from`), invariant enforcement
   (`Foo::new` rejects empty inputs), and any pure logic.
2. **Adapter unit tests** in `src/adapter/<direction>/<backend>/tests.rs`.
   Cover the row-to-domain `TryFrom` impl, and when the adapter owns a
   schema, read the migration file as a string (via
   `std::fs::read_to_string` + `env!("CARGO_MANIFEST_DIR")`) and assert the
   column / constraint / trigger set so the schema cannot regress silently.
3. **Facade unit tests** in `src/adapter/facade/internal/tests.rs` that
   wire the adapter on top of an in-memory port (`Arc<Mutex<Vec<Foo>>>`
   + an `AtomicI32` for ids). They exercise the public-facing behaviour
   without touching infrastructure and lock in object-safety / `Send + Sync`.
4. **`tests/` directory**:
   - `tests/public_api.rs` — compile-only. Names every documented consumer
     import (`use <crate>::Foo`), pins the constructor chain
     (`fn(PgPool) -> _`, `fn(R) -> _` as function pointers), and asserts the
     trait bounds the usecase relies on. The safety net for re-export /
     trait-bound refactors.
   - `tests/integration_persistence.rs` — live-database round-trips,
     `#[ignore]`-gated.

Live-DB tests load `.env` via `dotenvy::dotenv()`, read the single
`AEGIS_TEST_DATABASE_URL` (panic with a clear message if missing), apply
the migrations, then **drop the live table and the `_sqlx_migrations`
bookkeeping table** so each run starts clean. That URL must point at a
disposable test database — never the server's `AEGIS_DATABASE_URL`. This is
destructive against whatever database it is pointed at; that is intentional.
Run ignored tests with
`cargo test -p <crate> -- --ignored --test-threads=1`.

Generate a per-run unique value (atomic counter + wall-clock nanoseconds)
for any column with a `UNIQUE` constraint so concurrent runs do not collide.

## 10. README at the crate root

Every business lib crate gets a `README.md` covering:

- One-sentence purpose.
- A `src/` tree matching the actual module shape (e.g. `domain/`,
  `usecase/`, `adapter/persistence/postgres/`, `adapter/cache/in_memory/`,
  `adapter/facade/internal/`).
- Database setup if the crate owns a schema: the `sqlx migrate run --source
  lib/crates/<crate>/migrations` command, the env var
  (`AEGIS_TEST_DATABASE_URL` for the live-DB tests), and a small constructor
  snippet.
- How to add a migration: `sqlx migrate add --source
  lib/crates/<crate>/migrations --timestamp <description>`, then write the
  DDL into the file the CLI created (see §8).
- How to run the ignored tests (`cargo test -p <crate> -- --ignored`).
- A back-link to this guideline so newcomers find the cross-cutting
  conventions.

## 11. Verification gate, before any PR

```bash
cargo fmt --all -- --check
cargo clippy -p <crate> --all-targets --all-features -- -D warnings
cargo test -p <crate>
cargo doc -p <crate> --no-deps
cargo test -p <crate> -- --ignored --test-threads=1   # when AEGIS_TEST_DATABASE_URL is set
```

Run `cargo check --workspace` / `cargo clippy --workspace` /
`cargo test --workspace` when the crate is the only (or only-added)
workspace member. If unrelated workspace members fail because of system
libraries, document that rather than working around it.

## 12. Commits and review

- One commit per logical change (scaffolding, domain, usecase,
  infrastructure, public-API integration, follow-up fix). Lockfile drift
  gets its own `chore:` commit.
- Each commit message lists the spec coverage and the verification commands
  at the bottom so reviewers can run the same gate locally.
