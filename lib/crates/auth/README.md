# auth crate

Workspace library implementing the `apis::auth::AuthService` port. Mints
HS256 JWTs, validates them against an in-memory `token_version` cache
backed by Postgres, and adapts the usecase layer into the `AuthService`
contract through an in-memory facade.

> See [docs/guidelines/lib-crate-development.md](../../docs/guidelines/lib-crate-development.md)
> for the cross-cutting conventions this crate follows (workspace deps,
> no `mod.rs`, ports-and-adapters layering, ignored live-DB tests, etc.).

## Layout

```text
src/
  domain/                       # UserCredentials, DomainIdentity, Role, ports
                                # (UserCredentialsRepository, DomainIdentityRepository,
                                # TokenVersionCache)
  usecase/                      # AuthUsecase, command DTOs, errors, JWT mint/verify
  adapter/
    persistence/
      postgres/                 # SQLx-backed UserCredentialsRepo, DomainIdentityRepo
    cache/
      in_memory/                # InMemoryTokenVersionCache (Arc<RwLock<HashMap>>)
    facade/
      in_memory/                # AuthServiceImpl wiring usecase -> apis::auth::AuthService
migrations/                     # SQLx migrations applied to the database
```

The crate root re-exports the public surface (`UserCredentials`,
`DomainIdentity`, `Role`, `DomainError`, `UserCredentialsRepository`,
`DomainIdentityRepository`, `TokenVersionCache`,
`InMemoryTokenVersionCache`, `UserCredentialsRepo`, `DomainIdentityRepo`,
`AuthUsecase`, `AuthUsecaseConfig`, `AuthServiceImpl`, the command DTOs
`LoginWithPassword` / `LoginWithDomainUserInfo` / `VerifyAccessToken` /
`RefreshAccessToken` / `Logout`, and the view DTOs `TokenPairView` /
`AuthClaimsView` / `AccessTokenView` / `LogoutAck`) so consumers can
`use auth::*;` without reaching into the sub-modules.

## Database setup

The crate ships two SQLx migrations that define `auth_user_credentials`
and `auth_user_domain_identities`. Apply them before pointing the
repositories at the database:

```bash
sqlx migrate run --source lib/crates/auth/migrations
```

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/auth/migrations --timestamp <description>
```

Once the migrations are applied, construct the repositories and usecase
from a `sqlx::PgPool`:

```rust
use std::sync::Arc;
use std::time::Duration;

use auth::{
    AuthServiceImpl, AuthUsecase, AuthUsecaseConfig, DomainIdentityRepo,
    InMemoryTokenVersionCache, TokenVersionCache, UserCredentialsRepo,
};
use apis::user::UserService; // production wiring uses the user crate's facade

let credentials_repo = UserCredentialsRepo::new(pool.clone());
let identities_repo = DomainIdentityRepo::new(pool);
let user_service: Arc<dyn UserService> = Arc::new(/* … */);
let cache: Arc<dyn TokenVersionCache> = Arc::new(InMemoryTokenVersionCache::new());
let signing_key = b"<32 random bytes>".to_vec();

let usecase = AuthUsecase::new(AuthUsecaseConfig {
    credentials: credentials_repo,
    identities: identities_repo,
    user_service,
    cache,
    signing_key,
    access_ttl: Duration::from_secs(15 * 60),
    refresh_ttl: Duration::from_secs(7 * 24 * 60 * 60),
    // Comma-separated list of domain names the administrator-only
    // `register_user` endpoint will accept. Domain matching is
    // case-insensitive after trimming. An empty allowlist rejects
    // every registration.
    allow_domains: vec!["aegis.local".to_string()],
});

let auth_service: Arc<dyn apis::auth::AuthService> =
    Arc::new(AuthServiceImpl::new(usecase));
```

The `auth` crate does not run migrations at runtime; a deployment step
is required.

## Integration tests

The crate ships live-database integration tests at
[`tests/integration_persistence.rs`](tests/integration_persistence.rs).
They connect to PostgreSQL, apply both migrations, and exercise the
full repository surface plus a smoke test of `AuthUsecase::new`. They
are `#[ignore]`-gated so the default `cargo test -p auth` run stays
green without a database.

Run them against a local PostgreSQL with:

```bash
# .env at the workspace root is sourced automatically by the tests
# via dotenvy; AEGIS_DATABASE_URL must point at a reachable server.
cargo test -p auth -- --ignored
```

## Token-version cache

`AuthUsecase` plumbs through `Arc<dyn TokenVersionCache>`. The
in-memory backend
([`InMemoryTokenVersionCache`](adapter/cache/in_memory/)) is the default;
a future Redis backend can be added under `adapter/cache/redis.rs`
without touching the usecase, the repository, or any consumer.

`verify` and `refresh` go `cache.get → credentials.find_by_code (on
miss) + cache.put`. Login paths warm the cache with
`row.token_version`. `logout` decodes the supplied refresh token to
extract the user code, calls `credentials.bump_token_version`, then
writes the returned version into the cache via `cache.put`, so
subsequent verifies in the same process reject tokens minted before
the bump. Cross-process revocation still relies on a shared backend
(Redis) when one is wired in; in a single-process deployment the
in-memory cache is the source of truth within that process.

## Administrator-only user registration

`AuthUsecase::register_user` (exposed via
[`apis::auth::AuthService::register_user`]) creates a new general-role,
inactive user together with their initial credential and the requested
`DomainIdentity`. The flow is idempotent: re-running with the same
`user_code` and `domain_name`+`hostname`+`sid` returns the existing
view without re-inserting.

The usecase rejects the call when the normalised `domain_name` (lower
cased, whitespace trimmed) is not in `AuthUsecaseConfig::allow_domains`.
An empty `allow_domains` therefore rejects every registration —
production deployments must populate it (the `aegis-server` binary
reads it from the `AEGIS_AUTH_ALLOW_DOMAINS` env var, comma separated).
The `aegis-server` HTTP layer additionally gates the route on the
caller's role: only `Root` and `Admin` callers can reach the handler.