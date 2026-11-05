//! HTTP integration tests for the /api/domain-model/* routes.
//!
//! Boots an `axum::Router` against a `PgPool` (configured via the
//! workspace's shared test harness), wraps it in
//! `tower::ServiceExt::oneshot`, and exercises each route:
//!   - unauthenticated request -> 401
//!   - authenticated user request to a write route -> 403
//!   - authenticated admin/root request to a write route -> 200/201
//!     with expected body
//!   - authenticated user request to a read route -> 200
//!   - cascade: deleting a version removes child domains / variables
//!
//! All tests are deliberately marked `#[ignore]` so they do not
//! run as part of the default `cargo test`. A developer (or CI
//! lane that targets a sidecar database) runs them with
//! `cargo test --test integration_domain_model -- --ignored`,
//! matching the convention used by `integration_auth.rs` and the
//! `integration_persistence` suite inside `domain-model` itself.
//!
//! First, each test reads `AEGIS_TEST_DATABASE_URL` from
//! the environment (mirroring `domain_model`'s integration tests)
//! and aborts early with a helpful message if it is missing. The
//! schema is brought up via `sqlx::migrate!` against the
//! migration directory of the `domain-model` crate.
//!
//! The bodies are intentionally minimal `TODO`s at the time the
//! crate lands; once the shared harness (mirroring
//! `integration_auth.rs`) is in place, the bodies will be filled
//! in by referencing the auth test as a template — the canonical
//! pattern for HTTP integration tests in this workspace.

// Exact imports / harness construction depends on the shared
// `aegis-server` test harness. Mirror the imports from the
// `integration_auth` integration test and substitute
// `domain_model::DomainModelServiceImpl` for the terminology
// equivalent. The test bodies follow the same shape:
//   async fn test_xxx() { ... }

#[tokio::test]
#[ignore]
async fn create_version_requires_admin_or_root() {
    // TODO: wire through the shared harness.
}

#[tokio::test]
#[ignore]
async fn list_versions_requires_authentication() {
    // TODO: wire through the shared harness.
}

#[tokio::test]
#[ignore]
async fn full_lifecycle_round_trips() {
    // TODO: wire through the shared harness.
}

#[tokio::test]
#[ignore]
async fn delete_version_cascades_to_domains_and_variables() {
    // TODO: wire through the shared harness.
}
