# user crate

Workspace library providing a SQLx/PostgreSQL-backed DDD user
repository and an async `UserUsecase`.

> See [docs/guidelines/lib-crate-development.md](../../docs/guidelines/lib-crate-development.md)
> for the cross-cutting conventions this crate follows (workspace
> deps, no `mod.rs`, ports-and-adapters layering, ignored live-DB
> tests, etc.).

## Layout

```text
src/
  domain/                       # User, Role, validation, errors, repository port
  usecase/                      # UserUsecase, command DTOs, errors
  adapter/                      # persistence adapters
    persistence/
      postgres/                 # SQLx-backed UserRepo, UserRow
migrations/                     # SQLx migrations applied to the database
```

The crate root re-exports the public surface (`User`, `Role`,
`UserRepo`, `UserUsecase`, `CreateUser`, `UpdateUser`, `UserView`,
`UserNew`, `UserUpdate`, `DomainError`, `UsecaseError`) so consumers
can `use user::...` without reaching into the sub-modules.

## Database setup

The crate ships a single SQLx migration that defines the `users`
table. Apply it before pointing `UserRepo` (or any other consumer) at
the database:

```bash
sqlx migrate run --source lib/crates/user/migrations
```

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/user/migrations --timestamp <description>
```

Once the migration is applied, construct the repository and usecase
from a `sqlx::PgPool`:

```rust
use user::{UserRepo, UserUsecase};

let user_repo = UserRepo::new(pool);
let user_usecase = UserUsecase::new(user_repo);
```

The `user` crate does not run migrations at runtime; a deployment
step is required.

## Integration tests

The crate ships live-database integration tests at
[`tests/integration_persistence.rs`](tests/integration_persistence.rs).
They connect to PostgreSQL, apply the migration, and exercise the
full CRUD surface (including unique-violation and `NotFound`
round-trips) plus a smoke test of the usecase layer. They are
`#[ignore]`-gated so the default `cargo test -p user` run stays green
without a database.

Run them against a local PostgreSQL with:

```bash
# .env at the workspace root is sourced automatically by the tests
# via dotenvy; AEGIS_TEST_DATABASE_URL must point at a reachable server.
cargo test -p user -- --ignored
```
