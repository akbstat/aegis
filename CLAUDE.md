# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace shape

This is a Cargo + pnpm monorepo. The Rust side is a Cargo workspace; the TS side is a pnpm workspace; the two coexist at the same root.

- **Rust workspace** (`Cargo.toml`): members under `apps/desktop/aegis-desktop/src-tauri`, `apps/server/aegis-server`, and `lib/crates/{apis, auth, domain-model, project, terminology, user, windows-utils}`. `edition = "2024"`, `resolver = "3"`. Shared deps (sqlx, tokio, serde, reqwest, thiserror, chrono, axum, jsonwebtoken, …) are declared once in `[workspace.dependencies]` and inherited via `{ workspace = true }`. Local-only deps need a one-line `why` comment.
- **pnpm workspace** (`pnpm-workspace.yaml`): `apps/desktop/*` and `lib/packages/*`. The desktop app pulls `@aegis/ui` via `"workspace:*"`. Package manager is `pnpm@10.33.0`.
- **Two layers of architectural docs** already live in [`docs/guidelines/`](docs/guidelines/):
  - `lib-crate-development.md` — the Rust business-lib convention (DDD layered, ports-and-adapters, per-crate schema, ignored live-DB tests). Every business lib crate follows it.
  - `aegis-desktop-development.md` — the Tauri shell convention (transport-only Rust backend, feature-module TS frontend, file-based routing, query-key factory, settings persistence). The desktop app follows it.

Read those two files first; they are the source of truth for "how do I add a thing here." CLAUDE.md covers only the cross-workspace shape and the commands.

## High-level architecture

The repo ships three runnable surfaces that share one codebase:

1. **`lib/crates/*` — Rust business libraries.** DDD-layered (domain → usecase → adapter), each behind a `domain::port` trait. `lib/crates/apis` is the exception: it only declares ports + DTOs (no `domain`/`usecase`/`adapter` split) and is consumed by every other crate. Each business lib crate ships SQLx migrations under `migrations/`, has a per-crate `README.md`, and exposes an `AuthUsecase`/`UserUsecase`/etc. that is generic over its repository port so tests can inject an in-memory fake.
2. **`apps/server/aegis-server` — axum HTTP service.** Thin binary (config + tracing + `run(config)`); all real work is in the lib crates. Wires `AuthServiceImpl`, `UserServiceImpl`, `ProjectService`, `TerminologyService`, `DomainModelService` against a Postgres pool. Mounts the OpenAPI doc at `/api-docs/openapi.json` and swagger-ui at `/swagger-ui`. Errors are always rendered as a typed `ErrorBody { code, message }` so the desktop client can dispatch on the stable `code`.
3. **`apps/desktop/aegis-desktop` — Tauri + React + TS shell.** Rust side is transport-only (`src-tauri/src/commands/` is a 1:1 `#[tauri::command]` shim over `src-tauri/src/http/`); TS side is feature-modular under `src/features/<name>/{data,components,pages}/`. State lives in TanStack Query (key factory in `src/shared/query/keys.ts`); auth-gated routes live under `src/routes/_authed/`. Persistent theme/locale sync across windows goes through `settings.bin` + an `aegis:settings-changed` event. A workspace window (`WebviewWindow` opened for `/project/<code>`) runs in its own `WebviewWindow` with its own auth check.

`lib/packages/ui` (`@aegis/ui`) is the shared MUI-based component library — sidebar, theme provider, i18n provider, icons, `@aegis/ui/dnd` for drag-and-drop. Imported by the desktop app and tested in isolation.

## Conventions worth knowing before touching anything

- **Wire DTOs are duplicated by hand, not generated.** The desktop's `src/shared/api/types.ts` mirrors the Rust DTOs in `src-tauri/src/http/*` 1:1. TS identifiers are camelCase; the wire is snake_case; the rename happens at the serde boundary. Any change to wire shape must update both halves in the same commit.
- **One error type per boundary.** Server: `ApiError → ErrorBody` (`src-tauri/src/http/dto.rs`). Desktop: `ApiError` (tagged, `#[serde(tag = "kind", rename_all = "camelCase")]`, struct-shaped variants). Per-layer error enums (`DomainError`, `UsecaseError`) wrap the inner one with `#[source]` so `Error::source()` keeps the chain.
- **Ignore-gated live-DB tests.** Every business lib crate and the server ship live-Postgres integration tests behind `#[ignore]`. They load `.env` via `dotenvy`, read the single `AEGIS_TEST_DATABASE_URL`, run migrations, and **drop the live table + `_sqlx_migrations`** before each run — destructive on purpose. Run with `cargo test -p <crate> -- --ignored --test-threads=1`.
- **Default `QueryClient` is non-retrying.** Tauri calls hit a local sidecar, not a flaky network; `staleTime: Infinity`, `retry: false`, `refetchOnWindowFocus/Reconnect: false` are the defaults for queries, and `retry: false` for mutations. Per-query overrides live in the hook file (e.g. `bootstrap.ts` pins `staleTime: 0`).
- **Bootstrap-then-route flow.** Desktop `main.tsx` synchronously redirects `/` to `/bootstrap`; `shouldRedirectToBootstrap` excludes `/project/<code>` so workspace windows skip the probes. `/_authed/route.tsx` is the pathless auth guard — any failure (including a broken token store) is treated as logged-out.

## Common commands

All commands run from the workspace root unless noted.

### Rust

```bash
cargo build -p <crate>                # build a single crate
cargo test -p <crate>                 # unit + doc tests
cargo test -p <crate> -- --ignored --test-threads=1   # live-DB integration (needs AEGIS_TEST_DATABASE_URL)
cargo clippy -p <crate> --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo doc -p <crate> --no-deps
cargo check --workspace               # cheap cross-crate sanity check
cargo test --workspace                # full Rust test run
```

Live-DB env var: every live-DB test — one per business lib crate plus the server's own integration tests — reads the single `AEGIS_TEST_DATABASE_URL` (a disposable test database, never the server's `AEGIS_DATABASE_URL`). All live-DB tests are destructive against that database; they intentionally drop tables on every run.

### Server (`apps/server/aegis-server`)

```bash
export AEGIS_DATABASE_URL=postgres://user:pass@localhost/aegis
export AEGIS_AUTH_SIGNING_KEY=$(openssl rand -hex 32)
cargo run -p aegis-server             # listens on 0.0.0.0:8080 by default
```

The server does not auto-migrate. Apply each crate's migrations first:
`sqlx migrate run --source lib/crates/<crate>/migrations`.

### Desktop app (`apps/desktop/aegis-desktop`)

```bash
# Frontend-only
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test                 # one-shot vitest
pnpm --filter aegis-desktop test:watch           # vitest watch
pnpm --filter aegis-desktop build                # tsc + vite build

# Full Tauri shell (frontend + Rust)
pnpm --filter aegis-desktop tauri dev            # vite dev + tauri shell
pnpm --filter aegis-desktop tauri build          # production bundle
```

To run a single frontend test file:
```bash
pnpm --filter aegis-desktop test -- src/test/features/auth/login.test.tsx
```

To run a single backend test:
```bash
cargo test -p aegis-desktop --lib http::auth::tests::login_persists_tokens
```

### Shared UI (`lib/packages/ui`)

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
```

## Spec & plan history

Past design decisions live in `docs/superpowers/` — one dated `-design.md` spec and `-plan.md` per feature. When working on an established area (`terminology`, `domain-model`, `aegis-desktop-sdtm-domain-list`, …) the spec from the same date range is the cheapest way to learn the *intent* behind the current shape.