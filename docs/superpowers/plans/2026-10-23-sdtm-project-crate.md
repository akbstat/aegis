# SDTM Project Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `lib/crates/sdtm-project` — a business lib crate owning the per-project SDTM model tree (versions → domains → variables, each with a one-to-one override) — plus the `apis::sdtm_project::SdtmProjectService` port and the server's `/api/sdtm-project` HTTP surface.

**Architecture:** DDD-layered business crate (`domain` → `usecase` → `adapter`) with five aggregates, five PostgreSQL repository ports, one cross-crate `ProjectLookup` port, and a generic `SdtmProjectUsecase` that a facade adapts onto the new `apis` port. The server exposes 22 routes over that port.

**Tech Stack:** Rust edition 2024, SQLx 0.9 (runtime API), axum 0.8 + utoipa-axum 0.2, `async-trait`, `thiserror`, `serde`/`serde_json`, PostgreSQL.

**Spec:** `docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md` — read it before Task 1; the plan argues from it, so the spec travels with it.

## Global Constraints

- Rust **edition 2024**, `resolver = "3"`, workspace root is `Cargo.toml` at the repo root.
- New crate package name **`sdtm-project`**, directory **`lib/crates/sdtm-project`**.
- Every shared dependency is inherited as `{ workspace = true }`. Local path deps need a one-line `#` comment saying why.
- **No `mod.rs`.** Layout is `src/<module>.rs` + `src/<module>/<child>.rs`; each layer module declares `mod …;` and re-exports with `pub use …;`.
- **The crate must not depend on `domain-model`.** `DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole`, `SdtmDomainDescription(+Detail)` and `SdtmVariableDescription(+Detail)` are **copied** into this crate's own `domain/` layer.
- **The desktop app is out of scope.** Do not touch anything under `apps/desktop/aegis-desktop`.
- SQLx **runtime API only** (`sqlx::query_as`, `sqlx::query`, `FromRow`). No compile-time-checked macros — the workspace ships no `.sqlx/` cache and no build-time `DATABASE_URL`.
- Every table has `created_at` / `updated_at` with `DEFAULT NOW()` plus a `BEFORE UPDATE` trigger that refreshes `updated_at`.
- Every enum-shaped column has a `CHECK` constraint listing the allowed values.
- The override field is spelled **`domain_override`** / **`variable_override`** in Rust (`override` is a reserved word and cannot be a field name) and serialises as `"override"` on the wire.
- Error types are exactly three: `DomainError` (domain), `UsecaseError` (usecase), `SdtmProjectApiError` (apis port). Per-layer enums wrap the inner one with `#[source]`; `impl From<DomainError> for UsecaseError` exists.
- Ports that must be object-safe carry `#[async_trait]` and `Send + Sync`.
- Migration files live in `lib/crates/sdtm-project/migrations/`, numbered `0001_…` upward, and are applied in the live test with `sqlx::migrate!("./migrations")`.
- A missing row must surface as the **per-aggregate** `*NotFound` variant (e.g. `DomainError::SdtmProjectVersionNotFound(id)`), never the generic `DomainError::NotFound`. The generic variant is the `sqlx::Error::RowNotFound` fallback only.
- `key_variables` is an **ordered** `Vec<String>`. The caller's order is preserved end to end. Duplicates are removed on write (first occurrence wins); blank entries are a validation error. Nothing is ever re-sorted.
- Every commit message ends with the verification commands that were run.

## Review Focus

Five input classes or failure modes the spec implies but that no task's tests would otherwise pin. Each has a test added to the task that owns the code.

1. **The parent is deleted between the usecase's existence check and the INSERT.** The pre-check cannot prevent it; the DB foreign key fires and the repository maps SQLSTATE `23503` to `Fk*NotFound`. The client must get **404**, not a 500. → Task 5 (integration test `domain_create_when_version_vanished_maps_to_fk_not_found`).
2. **DELETE on an id that does not exist.** Must be **404**, not a silent `204`. Applies to all five aggregates. → Task 3 (`version_delete_of_missing_row_is_not_found`), Task 4 (`domain_delete_of_missing_row_is_not_found`, `domain_override_delete_of_missing_row_is_not_found`), Task 5 (`variable_delete_of_missing_row_is_not_found`, `variable_override_delete_of_missing_row_is_not_found`).
3. **UPDATE on an id that does not exist.** Must be **404**, not a silent no-op that reports success. → Task 3 (`version_update_of_missing_row_is_not_found`), Task 4 (`domain_update_of_missing_row_is_not_found`), Task 5 (`variable_update_of_missing_row_is_not_found`).
4. **`key_variables` edge cases.** An empty list is legal. Duplicate entries collapse to the first occurrence. A blank entry is a validation error, not a silently dropped row. The order the caller supplied is exactly the order that comes back. → Task 1 (`normalize_key_variables_*`), Task 4 (`domain_key_variables_preserve_caller_order`), Task 7 (`key_variables_order_survives_the_api`).
5. **Cascade delete.** Deleting a version removes its domains, their overrides, their variables and their variable overrides; deleting a domain removes its override and its variables. → Task 4 (`version_delete_cascades_to_the_whole_subtree`).

## File Structure

**Created — the crate:**

```
lib/crates/sdtm-project/
├── Cargo.toml
├── README.md
├── migrations/
│   ├── 0001_create_sdtm_project_versions.sql
│   ├── 0002_create_sdtm_project_domains.sql
│   ├── 0003_create_sdtm_project_domain_overrides.sql
│   ├── 0004_create_sdtm_project_variables.sql
│   └── 0005_create_sdtm_project_variable_overrides.sql
├── src/
│   ├── lib.rs                       # three layer modules + crate-root re-exports
│   ├── domain.rs                    # declares + re-exports the domain children
│   ├── domain/
│   │   ├── domain_category.rs       # copied from domain-model
│   │   ├── variable_type.rs         # copied from domain-model
│   │   ├── sdtm_domain_description.rs
│   │   ├── sdtm_variable_description.rs
│   │   ├── sdtm_project_version.rs
│   │   ├── sdtm_project_domain.rs
│   │   ├── sdtm_project_domain_override.rs
│   │   ├── sdtm_project_variable.rs
│   │   ├── sdtm_project_variable_override.rs
│   │   ├── project_lookup.rs
│   │   ├── repository.rs            # the five repository port traits
│   │   ├── error.rs                 # DomainError
│   │   └── tests.rs
│   ├── usecase.rs
│   ├── usecase/
│   │   ├── commands.rs              # ten Create/Update command DTOs
│   │   ├── error.rs                 # UsecaseError
│   │   ├── views.rs                 # five view DTOs + From impls
│   │   ├── sdtm_project_usecase.rs  # the usecase + its config
│   │   └── tests.rs                 # in-memory fakes + usecase tests
│   └── adapter.rs
│   └── adapter/
│       ├── persistence.rs
│       ├── persistence/postgres.rs
│       ├── persistence/postgres/
│       │   ├── sdtm_project_version_repo.rs
│       │   ├── sdtm_project_domain_repo.rs
│       │   ├── sdtm_project_domain_override_repo.rs
│       │   ├── sdtm_project_variable_repo.rs
│       │   └── sdtm_project_variable_override_repo.rs
│       ├── service.rs
│       ├── service/project.rs
│       ├── service/project/project_lookup_impl.rs
│       ├── facade.rs
│       └── facade/in_memory.rs
│       └── facade/in_memory/
│           ├── service.rs
│           └── tests.rs
├── tests/
│   ├── public_api.rs
│   └── integration_persistence.rs
```

**Created — the apis crate:** `lib/crates/apis/src/sdtm_project.rs`

**Created — the server:**

```
apps/server/aegis-server/src/transport/http/sdtm_project.rs
apps/server/aegis-server/src/transport/http/sdtm_project/router.rs
apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs
```

**Modified:** `Cargo.toml` (workspace member), `lib/crates/apis/src/lib.rs` (module), `lib/crates/apis/tests/public_api.rs`, `apps/server/aegis-server/Cargo.toml`, `apps/server/aegis-server/src/state.rs`, `apps/server/aegis-server/src/run.rs`, `apps/server/aegis-server/src/transport/http/{dto.rs,error.rs,router.rs,openapi.rs}`.

---

### Task 1: Domain layer

**Files:**
- Modify: `Cargo.toml` — add `"lib/crates/sdtm-project"` to `[workspace].members`
- Create: `lib/crates/sdtm-project/Cargo.toml`
- Create: `lib/crates/sdtm-project/src/lib.rs`
- Create: `lib/crates/sdtm-project/src/domain.rs`
- Create: `lib/crates/sdtm-project/src/domain/{domain_category,variable_type,sdtm_domain_description,sdtm_variable_description,sdtm_project_version,sdtm_project_domain,sdtm_project_domain_override,sdtm_project_variable,sdtm_project_variable_override,project_lookup,repository,error,tests}.rs`

**Interfaces:**
- Consumes: nothing (this is the first task).
- Produces (names later tasks rely on, all re-exported from the crate root `sdtm_project::…`):
  - `DomainCategory` — 7 variants `SpecialPurpose | Interventions | Events | Findings | TrialDesign | Relationships | StudyReference`; `pub fn as_str(&self) -> &'static str`; `impl TryFrom<&str> for DomainCategory { type Error = DomainError; }`. Serde renames: `"Special Purpose"`, `"Interventions"`, `"Events"`, `"Findings"`, `"Trial Design"`, `"Relationships"`, `"Study Reference"`.
  - `SdtmVariableType` — `Numeric | Character`; `as_str()`; `TryFrom<&str>`. `#[serde(rename_all = "PascalCase")]`.
  - `SdtmVariableCore` — `Req | Exp | Perm | Supp`; `as_str()`; `TryFrom<&str>`. `#[serde(rename_all = "PascalCase")]`.
  - `SdtmRole` — `Identifier | Topic | Timing | RecordQualifier | SynonymQualifier | VariableQualifier | GroupingQualifier | Rule`; `as_str()`; `TryFrom<&str>`. Serde renames: `"Identifier"`, `"Topic"`, `"Timing"`, `"Record Qualifier"`, `"Synonym Qualifier"`, `"Variable Qualifier"`, `"Grouping Qualifier"`, `"Rule"`.
  - `SdtmDomainDescription { pub lang: String, pub details: SdtmDomainDescriptionDetail }` and `SdtmDomainDescriptionDetail { pub description: String, pub structure: String }` — both `Serialize + Deserialize`.
  - `SdtmVariableDescription { pub lang: String, pub details: SdtmVariableDescriptionDetail }` and `SdtmVariableDescriptionDetail { pub label: String }` — both `Serialize + Deserialize`.
  - `SdtmProjectVersion { pub id: i64, pub project_code: String, pub version_name: String, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }` with `pub fn new(project_code: String, version_name: String) -> Result<Self, DomainError>` and `pub(crate) fn for_repository(id: i64, project_code: String, version_name: String, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self`. Manual `Debug` impl (structural, like `domain-model`).
  - `SdtmProjectVersionNew { pub project_code: String, pub version_name: String }` — `Debug + Clone`.
  - `SdtmProjectVersionUpdate { pub id: i64, pub version_name: Option<String> }` — `Debug + Clone + Default`.
  - `SdtmProjectDomain { pub id: i64, pub version_id: i64, pub domain_code: String, pub key_variables: Vec<String>, pub domain_override: Option<SdtmProjectDomainOverride>, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }` with:
    - `pub fn new(version_id: i64, domain_code: String, key_variables: Vec<String>) -> Result<Self, DomainError>`
    - `pub fn normalize_key_variables(raw: &[String]) -> Result<Vec<String>, DomainError>` — trims each entry, rejects a blank entry with `DomainError::EmptyKeyVariable`, removes duplicates keeping the first occurrence, preserves the caller's order.
    - `pub(crate) fn for_repository(id: i64, version_id: i64, domain_code: String, key_variables: Vec<String>, domain_override: Option<SdtmProjectDomainOverride>, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self`
  - `SdtmProjectDomainNew { pub version_id: i64, pub domain_code: String, pub key_variables: Vec<String> }`
  - `SdtmProjectDomainUpdate { pub id: i64, pub domain_code: Option<String>, pub key_variables: Option<Vec<String>> }`
  - `SdtmProjectDomainOverride { pub id: i64, pub domain_id: i64, pub category: DomainCategory, pub descriptions: Vec<SdtmDomainDescription>, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }` with `pub fn new(domain_id: i64, category: DomainCategory, descriptions: Vec<SdtmDomainDescription>) -> Result<Self, DomainError>` and `pub(crate) fn for_repository(id: i64, domain_id: i64, category: DomainCategory, descriptions: Vec<SdtmDomainDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self`.
  - `SdtmProjectDomainOverrideNew { pub domain_id: i64, pub category: DomainCategory, pub descriptions: Vec<SdtmDomainDescription> }`
  - `SdtmProjectDomainOverrideUpdate { pub domain_id: i64, pub category: Option<DomainCategory>, pub descriptions: Option<Vec<SdtmDomainDescription>> }`
  - `SdtmProjectVariable { pub id: i64, pub domain_id: i64, pub variable_code: String, pub variable_sequence: i64, pub variable_override: Option<SdtmProjectVariableOverride>, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }` with `pub fn new(domain_id: i64, variable_code: String, variable_sequence: i64) -> Result<Self, DomainError>` and `pub(crate) fn for_repository(id: i64, domain_id: i64, variable_code: String, variable_sequence: i64, variable_override: Option<SdtmProjectVariableOverride>, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self`.
  - `SdtmProjectVariableNew { pub domain_id: i64, pub variable_code: String, pub variable_sequence: i64 }`
  - `SdtmProjectVariableUpdate { pub id: i64, pub variable_code: Option<String>, pub variable_sequence: Option<i64> }`
  - `SdtmProjectVariableOverride { pub id: i64, pub variable_id: i64, pub variable_type: SdtmVariableType, pub variable_core: SdtmVariableCore, pub variable_role: Option<SdtmRole>, pub descriptions: Vec<SdtmVariableDescription>, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }` with `pub fn new(variable_id: i64, variable_type: SdtmVariableType, variable_core: SdtmVariableCore, variable_role: Option<SdtmRole>, descriptions: Vec<SdtmVariableDescription>) -> Result<Self, DomainError>` and `pub(crate) fn for_repository(id: i64, variable_id: i64, variable_type: SdtmVariableType, variable_core: SdtmVariableCore, variable_role: Option<SdtmRole>, descriptions: Vec<SdtmVariableDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self`.
  - `SdtmProjectVariableOverrideNew { pub variable_id: i64, pub variable_type: SdtmVariableType, pub variable_core: SdtmVariableCore, pub variable_role: Option<SdtmRole>, pub descriptions: Vec<SdtmVariableDescription> }`
  - `SdtmProjectVariableOverrideUpdate { pub variable_id: i64, pub variable_type: Option<SdtmVariableType>, pub variable_core: Option<SdtmVariableCore>, pub variable_role: Option<Option<SdtmRole>>, pub descriptions: Option<Vec<SdtmVariableDescription>> }` — `variable_role` is three-state: outer `None` = leave, `Some(None)` = clear to NULL, `Some(Some(r))` = replace.
  - `DomainError` — variants exactly: `EmptyProjectCode`, `EmptyVersionName`, `EmptyDomainCode`, `EmptyVariableCode`, `EmptyKeyVariable`, `InvalidDomainCategory(String)`, `InvalidVariableType(String)`, `InvalidVariableCore(String)`, `InvalidVariableRole(String)`, `ProjectNotFound(String)`, `SdtmProjectVersionNotFound(i64)`, `SdtmProjectDomainNotFound(i64)`, `SdtmProjectDomainOverrideNotFound(i64)`, `SdtmProjectVariableNotFound(i64)`, `SdtmProjectVariableOverrideNotFound(i64)`, `DuplicateSdtmProjectVersion { project_code: String, version_name: String }`, `DuplicateSdtmProjectDomain { version_id: i64, domain_code: String }`, `DuplicateSdtmProjectDomainOverride { domain_id: i64 }`, `DuplicateSdtmProjectVariable { domain_id: i64, variable_code: String }`, `DuplicateSdtmProjectVariableOverride { variable_id: i64 }`, `FkSdtmProjectVersionNotFound(i64)`, `FkSdtmProjectDomainNotFound(i64)`, `FkSdtmProjectVariableNotFound(i64)`, `NotFound`, `Repository(String)`. `#[derive(Debug, Clone, Error)]`.
  - `ProjectLookup` port: `#[async_trait] pub trait ProjectLookup: Send + Sync { async fn get_by_code(&self, code: &str) -> Result<(), DomainError>; }`
  - Five repository ports (exact method sets):
    - `SdtmProjectVersionRepository`: `create(SdtmProjectVersionNew) -> Result<SdtmProjectVersion, DomainError>`, `find_by_id(i64)`, `list_by_project(&str) -> Result<Vec<SdtmProjectVersion>, DomainError>`, `update(SdtmProjectVersionUpdate)`, `delete(i64) -> Result<(), DomainError>`
    - `SdtmProjectDomainRepository`: `create(SdtmProjectDomainNew)`, `find_by_id(i64)`, `list_by_version(i64) -> Result<Vec<SdtmProjectDomain>, DomainError>`, `update(SdtmProjectDomainUpdate)`, `delete(i64)`
    - `SdtmProjectDomainOverrideRepository`: `create(SdtmProjectDomainOverrideNew)`, `find_by_domain_id(i64)`, `update(SdtmProjectDomainOverrideUpdate)`, `delete(i64)`
    - `SdtmProjectVariableRepository`: `create(SdtmProjectVariableNew)`, `find_by_id(i64)`, `list_by_domain(i64) -> Result<Vec<SdtmProjectVariable>, DomainError>`, `update(SdtmProjectVariableUpdate)`, `delete(i64)`
    - `SdtmProjectVariableOverrideRepository`: `create(SdtmProjectVariableOverrideNew)`, `find_by_variable_id(i64)`, `update(SdtmProjectVariableOverrideUpdate)`, `delete(i64)`

- [ ] **Step 1: Register the crate in the workspace and create its manifest**

Add `"lib/crates/sdtm-project",` to `[workspace].members` in the root `Cargo.toml`, keeping the existing list's formatting (two crate names per line where they already are).

Create `lib/crates/sdtm-project/Cargo.toml`:

```toml
[package]
name = "sdtm-project"
version = "0.1.0"
edition = "2024"

[dependencies]
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
# `chrono` provides the `DateTime<Utc>` carried on every aggregate's
# `created_at` / `updated_at` columns.
chrono = { workspace = true }
# `serde` + `serde_json` are required for the JSONB round-trip of
# `Vec<String>` (key_variables) and the `Vec<…Description>` columns.
serde = { workspace = true }
serde_json = { workspace = true }
# `apis` provides the outbound `SdtmProjectService` port the facade
# implements. Path-dep because both crates share the workspace.
apis = { path = "../apis" }

[dev-dependencies]
dotenvy = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
```

- [ ] **Step 2: Write the failing domain tests**

Create `lib/crates/sdtm-project/src/domain/tests.rs` with these tests (names are load-bearing — later tasks and the public-API test reference them):

`domain_category_parses_known_strings` — for each of `("Special Purpose", DomainCategory::SpecialPurpose)`, `("Interventions", …)`, `("Events", …)`, `("Findings", …)`, `("Trial Design", …)`, `("Relationships", …)`, `("Study Reference", …)`: `DomainCategory::try_from(raw).unwrap()` equals the expected variant and `as_str()` equals `raw`.

`domain_category_rejects_unknown_string` — `DomainCategory::try_from("Bogus")` is `DomainError::InvalidDomainCategory("Bogus")`.

`variable_type_parses_known_strings` / `variable_type_rejects_unknown_string` — `"Numeric"` / `"Character"` round-trip; `"Date"` → `InvalidVariableType`.

`variable_core_parses_known_strings` / `variable_core_rejects_unknown_string` — `"Req"` / `"Exp"` / `"Perm"` / `"Supp"` round-trip; `"Bad"` → `InvalidVariableCore`.

`role_parses_known_strings` / `role_rejects_unknown_string` — `"Identifier"`, `"Topic"`, `"Timing"`, `"Record Qualifier"`, `"Synonym Qualifier"`, `"Variable Qualifier"`, `"Grouping Qualifier"`, `"Rule"` round-trip; `"Bad"` → `InvalidVariableRole`.

`sdtm_project_version_new_rejects_blank_project_code` — `SdtmProjectVersion::new("   ".into(), "v1".into())` → `DomainError::EmptyProjectCode`.

`sdtm_project_version_new_rejects_blank_version_name` — `SdtmProjectVersion::new("P".into(), "  ".into())` → `DomainError::EmptyVersionName`.

`sdtm_project_version_new_accepts_valid_input` — `SdtmProjectVersion::new("P1".into(), "2024-09-27".into())` has `project_code == "P1"` and `version_name == "2024-09-27"`.

`sdtm_project_domain_new_rejects_blank_domain_code` — `SdtmProjectDomain::new(1, "  ".into(), Vec::new())` → `DomainError::EmptyDomainCode`.

`sdtm_project_domain_new_rejects_blank_key_variable` — `SdtmProjectDomain::new(1, "AE".into(), vec!["  ".into()])` → `DomainError::EmptyKeyVariable`.

`sdtm_project_domain_new_accepts_valid_input` — `SdtmProjectDomain::new(1, "AE".into(), vec!["AETERM".into()])` yields `key_variables == vec!["AETERM".to_string()]`.

`normalize_key_variables_preserves_caller_order` — input `["AETERM", "AESEV", "AETERM"]`… no: use `["AESEV", "AETERM"]` → output is exactly `["AESEV", "AETERM"]` (not sorted).

`normalize_key_variables_dedups_preserving_first_occurrence` — input `["AETERM", "AESEV", "AETERM"]` → output `["AETERM", "AESEV"]`.

`normalize_key_variables_accepts_empty_list` — `&[]` → `Ok(vec![])`.

`normalize_key_variables_rejects_blank_entry` — `["AETERM", " "]` → `DomainError::EmptyKeyVariable`.

`sdtm_project_variable_new_rejects_blank_variable_code` — `SdtmProjectVariable::new(1, " ".into(), 1)` → `DomainError::EmptyVariableCode`.

`sdtm_project_variable_new_accepts_valid_input` — `SdtmProjectVariable::new(1, "AETERM".into(), 11)` yields `variable_code == "AETERM"` and `variable_sequence == 11`.

`domain_override_new_accepts_valid_input` — `SdtmProjectDomainOverride::new(1, DomainCategory::Events, vec![desc])` yields `category == DomainCategory::Events` and one description.

`variable_override_new_accepts_valid_input` — `SdtmProjectVariableOverride::new(1, SdtmVariableType::Character, SdtmVariableCore::Req, Some(SdtmRole::Topic), vec![vdesc])` yields the three enum fields plus one description.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p sdtm-project`
Expected: compile failure — the crate has no `domain` module yet.

- [ ] **Step 4: Implement the copied value objects**

Create `src/domain/domain_category.rs`, `src/domain/variable_type.rs`, `src/domain/sdtm_domain_description.rs`, `src/domain/sdtm_variable_description.rs`. Copy the enum variants, `as_str()` bodies, `TryFrom<&str>` bodies and serde attributes verbatim from `lib/crates/domain-model/src/domain/domain_category.rs` and `lib/crates/domain-model/src/domain/variable_type.rs`, changing only the error type to this crate's `DomainError` and the error variants to `InvalidDomainCategory` / `InvalidVariableType` / `InvalidVariableCore` / `InvalidVariableRole`.

Edition 2024 puts `TryFrom` in the prelude — do **not** add `use std::convert::TryFrom;`.

The description DTOs are plain `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` structs with the fields listed in Interfaces.

- [ ] **Step 5: Implement `DomainError`**

Create `src/domain/error.rs` with the exact variant list from Interfaces. Each variant carries an `#[error("…")]` message; the `Duplicate*` variants use named fields in their message (e.g. `"sdtm project version already exists: {project_code} / {version_name}"`).

- [ ] **Step 6: Implement the five aggregates and their input DTOs**

Create `src/domain/sdtm_project_version.rs`, `src/domain/sdtm_project_domain.rs`, `src/domain/sdtm_project_domain_override.rs`, `src/domain/sdtm_project_variable.rs`, `src/domain/sdtm_project_variable_override.rs`.

Each aggregate: `#[derive(Clone, PartialEq, Eq)]` plus a manual `std::fmt::Debug` impl that lists every field structurally (mirror `domain-model`'s aggregates — no redaction, but do not derive `Debug`).

Each `new(...)` sets `id: 0` and both timestamps to `DateTime::<Utc>::from_timestamp(0, 0).unwrap()`, then validates:

- `SdtmProjectVersion::new` — blank `project_code` → `EmptyProjectCode`; blank `version_name` → `EmptyVersionName`.
- `SdtmProjectDomain::new` — blank `domain_code` → `EmptyDomainCode`; `key_variables` run through `normalize_key_variables`, whose error propagates. Store the **normalised** vector.
- `SdtmProjectDomain::normalize_key_variables(raw: &[String]) -> Result<Vec<String>, DomainError>` — for each entry: `let t = entry.trim(); if t.is_empty() { return Err(DomainError::EmptyKeyVariable); }` then push `t.to_string()` unless that exact value is already present (first occurrence wins). Return the vector in the caller's order.
- `SdtmProjectVariable::new` — blank `variable_code` → `EmptyVariableCode`.
- The two overrides have no string fields to validate; `new` returns `Ok`.

Each `for_repository(...)` takes the full field list (including the `domain_override` / `variable_override` option for the two parents) and skips validation.

- [ ] **Step 7: Implement the ports**

Create `src/domain/repository.rs` with the five traits exactly as listed in Interfaces, each `#[async_trait]` + `Send + Sync`, with `use async_trait::async_trait;` at the top.

Create `src/domain/project_lookup.rs` with the `ProjectLookup` trait.

- [ ] **Step 8: Wire up the module tree and crate root**

`src/domain.rs`:

```rust
mod domain_category;
mod error;
mod project_lookup;
mod repository;
mod sdtm_domain_description;
mod sdtm_project_domain;
mod sdtm_project_domain_override;
mod sdtm_project_variable;
mod sdtm_project_variable_override;
mod sdtm_project_version;
mod sdtm_variable_description;
mod variable_type;

#[cfg(test)]
mod tests;

pub use domain_category::DomainCategory;
pub use error::DomainError;
pub use project_lookup::ProjectLookup;
pub use repository::{
    SdtmProjectDomainOverrideRepository, SdtmProjectDomainRepository,
    SdtmProjectVariableOverrideRepository, SdtmProjectVariableRepository,
    SdtmProjectVersionRepository,
};
pub use sdtm_domain_description::{SdtmDomainDescription, SdtmDomainDescriptionDetail};
pub use sdtm_project_domain::{SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainUpdate};
pub use sdtm_project_domain_override::{
    SdtmProjectDomainOverride, SdtmProjectDomainOverrideNew, SdtmProjectDomainOverrideUpdate,
};
pub use sdtm_project_variable::{
    SdtmProjectVariable, SdtmProjectVariableNew, SdtmProjectVariableUpdate,
};
pub use sdtm_project_variable_override::{
    SdtmProjectVariableOverride, SdtmProjectVariableOverrideNew, SdtmProjectVariableOverrideUpdate,
};
pub use sdtm_project_version::{SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionUpdate};
pub use sdtm_variable_description::{SdtmVariableDescription, SdtmVariableDescriptionDetail};
pub use variable_type::{SdtmRole, SdtmVariableCore, SdtmVariableType};
```

`src/lib.rs` — declare `pub mod domain;` plus empty `pub mod usecase;` / `pub mod adapter;` placeholders for now (Task 2 and Task 7 fill them), and re-export the domain surface:

```rust
pub use domain::{
    DomainCategory, DomainError, ProjectLookup, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmProjectDomain, SdtmProjectDomainNew,
    SdtmProjectDomainOverride, SdtmProjectDomainOverrideNew,
    SdtmProjectDomainOverrideRepository, SdtmProjectDomainOverrideUpdate,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectVariable,
    SdtmProjectVariableNew, SdtmProjectVariableOverride, SdtmProjectVariableOverrideNew,
    SdtmProjectVariableOverrideRepository, SdtmProjectVariableOverrideUpdate,
    SdtmProjectVariableRepository, SdtmProjectVariableUpdate, SdtmProjectVersion,
    SdtmProjectVersionNew, SdtmProjectVersionRepository, SdtmProjectVersionUpdate, SdtmRole,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableType,
};
```

`src/usecase.rs` and `src/adapter.rs` are one-line placeholders (`// Filled in by later tasks.` plus an empty `pub mod` body is not allowed — use a doc comment and no items, or `pub use` nothing). Simplest valid form: a file containing only a `//!` doc comment.

- [ ] **Step 9: Run the tests to verify they pass**

Run: `cargo test -p sdtm-project`
Expected: PASS — all 21 domain tests green.

- [ ] **Step 10: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings`
Expected: clean. If clippy complains that `for_repository` is unused so far, add `#[allow(dead_code)]` — Task 2 and Task 3 use them.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
feat(sdtm-project): add the domain layer for the project SDTM model tree

Five aggregates (version, domain, domain override, variable, variable
override), the value objects copied from domain-model, the five
repository ports, the ProjectLookup port and DomainError.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p sdtm-project
         cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Usecase layer

**Files:**
- Create: `lib/crates/sdtm-project/src/usecase.rs`
- Create: `lib/crates/sdtm-project/src/usecase/{commands,error,views,sdtm_project_usecase,tests}.rs`
- Modify: `lib/crates/sdtm-project/src/lib.rs` — replace the placeholder `usecase` module and add the usecase re-exports

**Interfaces:**
- Consumes: everything Task 1 produced (the five aggregates, the `*New` / `*Update` DTOs, the five repository ports, `ProjectLookup`, `DomainError`).
- Produces (crate-root names):
  - Commands: `CreateSdtmProjectVersion { project_code: String, version_name: String }`, `UpdateSdtmProjectVersion { id: i64, version_name: Option<String> }`, `CreateSdtmProjectDomain { version_id: i64, domain_code: String, key_variables: Vec<String> }`, `UpdateSdtmProjectDomain { id: i64, domain_code: Option<String>, key_variables: Option<Vec<String>> }`, `CreateSdtmProjectDomainOverride { domain_id: i64, category: DomainCategory, descriptions: Vec<SdtmDomainDescription> }`, `UpdateSdtmProjectDomainOverride { domain_id: i64, category: Option<DomainCategory>, descriptions: Option<Vec<SdtmDomainDescription>> }`, `CreateSdtmProjectVariable { domain_id: i64, variable_code: String, variable_sequence: i64 }`, `UpdateSdtmProjectVariable { id: i64, variable_code: Option<String>, variable_sequence: Option<i64> }`, `CreateSdtmProjectVariableOverride { variable_id: i64, variable_type: SdtmVariableType, variable_core: SdtmVariableCore, variable_role: Option<SdtmRole>, descriptions: Vec<SdtmVariableDescription> }`, `UpdateSdtmProjectVariableOverride { variable_id: i64, variable_type: Option<SdtmVariableType>, variable_core: Option<SdtmVariableCore>, variable_role: Option<Option<SdtmRole>>, descriptions: Option<Vec<SdtmVariableDescription>> }`. All `#[derive(Debug, Clone)]`; the `Update*` ones also derive `Default`.
  - Views: `SdtmProjectVersionView { id: i64, project_code: String, version_name: String, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectDomainView { id: i64, version_id: i64, domain_code: String, key_variables: Vec<String>, domain_override: Option<SdtmProjectDomainOverrideView>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectDomainOverrideView { id: i64, domain_id: i64, category: DomainCategory, descriptions: Vec<SdtmDomainDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectVariableView { id: i64, domain_id: i64, variable_code: String, variable_sequence: i64, variable_override: Option<SdtmProjectVariableOverrideView>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectVariableOverrideView { id: i64, variable_id: i64, variable_type: SdtmVariableType, variable_core: SdtmVariableCore, variable_role: Option<SdtmRole>, descriptions: Vec<SdtmVariableDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`. All `#[derive(Debug, Clone, PartialEq, Eq)]`, each with a `From<Aggregate>` impl.
  - `UsecaseError { Validation(#[source] DomainError), Repository(#[source] DomainError) }` with `#[derive(Debug, Error)]` and `impl From<DomainError> for UsecaseError` mapping every `DomainError` to `UsecaseError::Repository`.
  - `SdtmProjectUsecaseConfig<V: SdtmProjectVersionRepository, D: SdtmProjectDomainRepository, DO: SdtmProjectDomainOverrideRepository, Va: SdtmProjectVariableRepository, VO: SdtmProjectVariableOverrideRepository, P: ProjectLookup>` — public fields `version_repo: V`, `domain_repo: D`, `domain_override_repo: DO`, `variable_repo: Va`, `variable_override_repo: VO`, `projects: Arc<P>`.
  - `SdtmProjectUsecase<V, D, DO, Va, VO, P>` with `pub fn new(cfg: SdtmProjectUsecaseConfig<V, D, DO, Va, VO, P>) -> Self` and the twenty methods:

    ```
    async fn create_version(&self, cmd: CreateSdtmProjectVersion) -> Result<SdtmProjectVersionView, UsecaseError>
    async fn get_version_by_id(&self, id: i64) -> Result<SdtmProjectVersionView, UsecaseError>
    async fn list_versions_by_project(&self, project_code: &str) -> Result<Vec<SdtmProjectVersionView>, UsecaseError>
    async fn update_version(&self, cmd: UpdateSdtmProjectVersion) -> Result<SdtmProjectVersionView, UsecaseError>
    async fn delete_version(&self, id: i64) -> Result<(), UsecaseError>

    async fn create_domain(&self, cmd: CreateSdtmProjectDomain) -> Result<SdtmProjectDomainView, UsecaseError>
    async fn get_domain_by_id(&self, id: i64) -> Result<SdtmProjectDomainView, UsecaseError>
    async fn list_domains_by_version(&self, version_id: i64) -> Result<Vec<SdtmProjectDomainView>, UsecaseError>
    async fn update_domain(&self, cmd: UpdateSdtmProjectDomain) -> Result<SdtmProjectDomainView, UsecaseError>
    async fn delete_domain(&self, id: i64) -> Result<(), UsecaseError>

    async fn create_domain_override(&self, cmd: CreateSdtmProjectDomainOverride) -> Result<SdtmProjectDomainOverrideView, UsecaseError>
    async fn get_domain_override(&self, domain_id: i64) -> Result<Option<SdtmProjectDomainOverrideView>, UsecaseError>
    async fn update_domain_override(&self, cmd: UpdateSdtmProjectDomainOverride) -> Result<SdtmProjectDomainOverrideView, UsecaseError>
    async fn delete_domain_override(&self, domain_id: i64) -> Result<(), UsecaseError>

    async fn create_variable(&self, cmd: CreateSdtmProjectVariable) -> Result<SdtmProjectVariableView, UsecaseError>
    async fn get_variable_by_id(&self, id: i64) -> Result<SdtmProjectVariableView, UsecaseError>
    async fn list_variables_by_domain(&self, domain_id: i64) -> Result<Vec<SdtmProjectVariableView>, UsecaseError>
    async fn update_variable(&self, cmd: UpdateSdtmProjectVariable) -> Result<SdtmProjectVariableView, UsecaseError>
    async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError>

    async fn create_variable_override(&self, cmd: CreateSdtmProjectVariableOverride) -> Result<SdtmProjectVariableOverrideView, UsecaseError>
    async fn get_variable_override(&self, variable_id: i64) -> Result<Option<SdtmProjectVariableOverrideView>, UsecaseError>
    async fn update_variable_override(&self, cmd: UpdateSdtmProjectVariableOverride) -> Result<SdtmProjectVariableOverrideView, UsecaseError>
    async fn delete_variable_override(&self, variable_id: i64) -> Result<(), UsecaseError>
    ```

- [ ] **Step 1: Write the failing usecase tests**

Create `lib/crates/sdtm-project/src/usecase/tests.rs`.

The in-memory fakes use **one shared store** so the domain/variable fakes can hydrate the override the way the SQL does. Shape:

```rust
#[derive(Default)]
struct InMemoryStore {
    versions: Mutex<Vec<SdtmProjectVersion>>,
    domains: Mutex<Vec<SdtmProjectDomain>>,
    domain_overrides: Mutex<Vec<SdtmProjectDomainOverride>>,
    variables: Mutex<Vec<SdtmProjectVariable>>,
    variable_overrides: Mutex<Vec<SdtmProjectVariableOverride>>,
    next_id: AtomicI64,
}
```

Five repo structs (`InMemoryVersionRepo`, `InMemoryDomainRepo`, `InMemoryDomainOverrideRepo`, `InMemoryVariableRepo`, `InMemoryVariableOverrideRepo`), each holding `Arc<InMemoryStore>` and implementing its port. The domain repo's `find_by_id` / `list_by_version` / `update` hydrate `domain_override` by looking up the matching row in `store.domain_overrides`; the variable repo does the same against `store.variable_overrides`, hydrating `variable_override`. The store's own `SdtmProjectDomain` / `SdtmProjectVariable` rows always carry `domain_override: None` / `variable_override: None` — the override lives in its own collection.

A `FakeProjectLookup` implements `ProjectLookup`; it holds `Vec<String>` of known codes and returns `Err(DomainError::ProjectNotFound(code))` for anything else.

Tests (names are load-bearing):

`version_crud_round_trips` — create a version for `"P1"`; `list_versions_by_project("P1")` returns exactly one entry with the same id; `update_version` with `version_name: Some("v2")` reports `"v2"`; `delete_version` then leaves the list empty.

`version_create_rejects_unknown_project` — creating a version for a code the lookup does not know is `UsecaseError::Repository(DomainError::ProjectNotFound(code))`.

`version_create_rejects_blank_version_name` — `UsecaseError::Validation(DomainError::EmptyVersionName)`.

`domain_crud_round_trips` — create a domain on a known version; `list_domains_by_version` returns one; `update_domain` changes `domain_code`; `delete_domain` empties the list.

`domain_create_rejects_missing_version` — creating a domain on version id `999` is `UsecaseError::Repository(DomainError::FkSdtmProjectVersionNotFound(999))`.

`domain_list_carries_the_override` — create a domain, then a domain override for it; `list_domains_by_version` returns a domain whose `domain_override` is `Some` with the right `category`.

`domain_update_keeps_the_override` — after the override exists, `update_domain` (changing only `domain_code`) still returns a domain whose `domain_override` is `Some`.

`domain_override_crud_round_trips` — create → `get_domain_override(domain_id)` returns `Some(the override)` → `update_domain_override` with a new `category` returns the new category → `delete_domain_override` → `get_domain_override` is `Ok(None)`. The override is optional, so a missing one is `None`, not an error.

`domain_override_create_rejects_missing_domain` — creating an override on domain id `999` is `UsecaseError::Repository(DomainError::FkSdtmProjectDomainNotFound(999))`.

`variable_crud_round_trips` — create a variable; `list_variables_by_domain` returns one; `update_variable` changes `variable_sequence`; `delete_variable` empties the list.

`variable_create_rejects_missing_domain` — creating a variable on domain id `999` is `UsecaseError::Repository(DomainError::FkSdtmProjectDomainNotFound(999))`.

`variable_override_crud_round_trips` — create → get → update → delete round trip for the variable override.

`variable_override_create_rejects_missing_variable` — creating a variable override on variable id `999` is `UsecaseError::Repository(DomainError::FkSdtmProjectVariableNotFound(999))`.

`variable_override_update_clears_the_role` — create with `variable_role: Some(SdtmRole::Topic)`; `update_variable_override` with `variable_role: Some(None)` returns `variable_role == None`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sdtm-project`
Expected: compile failure — no `usecase` module items yet.

- [ ] **Step 3: Implement `UsecaseError`**

Create `src/usecase/error.rs`:

```rust
use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(#[source] DomainError),

    #[error("repository error: {0}")]
    Repository(#[source] DomainError),
}

impl From<DomainError> for UsecaseError {
    fn from(err: DomainError) -> Self {
        UsecaseError::Repository(err)
    }
}
```

- [ ] **Step 4: Implement the command DTOs**

Create `src/usecase/commands.rs` with the ten command structs listed in Interfaces. Field names and types exactly as listed.

- [ ] **Step 5: Implement the view DTOs**

Create `src/usecase/views.rs` with the five view structs and their `From` impls. Each `From` copies fields one-for-one; `key_variables` moves across as-is (already normalised by the domain layer).

- [ ] **Step 6: Implement the usecase**

Create `src/usecase/sdtm_project_usecase.rs`.

Structures:

```rust
pub struct SdtmProjectUsecaseConfig<V, D, DO, Va, VO, P>
where
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
{
    pub version_repo: V,
    pub domain_repo: D,
    pub domain_override_repo: DO,
    pub variable_repo: Va,
    pub variable_override_repo: VO,
    pub projects: Arc<P>,
}
```

`SdtmProjectUsecase` mirrors it with private fields and `pub fn new(cfg: SdtmProjectUsecaseConfig<V, D, DO, Va, VO, P>) -> Self`.

Method bodies, in order:

- `create_version` — validate `version_name` non-blank and `project_code` non-blank (blank → `UsecaseError::Validation(DomainError::EmptyVersionName)` / `EmptyProjectCode`); call `self.projects.get_by_code(&cmd.project_code).await?` **before** the insert (a missing project surfaces as `UsecaseError::Repository(DomainError::ProjectNotFound(code))` through the `?`); then `self.version_repo.create(SdtmProjectVersionNew { project_code: cmd.project_code, version_name: cmd.version_name }).await?` and map the result with `Into::into`.
- `get_version_by_id` — `self.version_repo.find_by_id(id).await?` then `Into::into`.
- `list_versions_by_project` — `self.version_repo.list_by_project(project_code).await?` then map each with `Into::into`. **No project existence check** — an unknown project yields an empty list.
- `update_version` — validate `version_name` when present (blank → `Validation(EmptyVersionName)`); `self.version_repo.update(SdtmProjectVersionUpdate { id: cmd.id, version_name: cmd.version_name }).await?`. `project_code` is not updatable.
- `delete_version` — `self.version_repo.delete(id).await?`.
- `create_domain` — validate `domain_code` non-blank → `Validation(EmptyDomainCode)`; normalise `key_variables` via `SdtmProjectDomain::normalize_key_variables(&cmd.key_variables)` and map its `DomainError` to `UsecaseError::Validation`; **check the parent first**: `self.version_repo.find_by_id(cmd.version_id).await?` — the repository's `SdtmProjectVersionNotFound` becomes `UsecaseError::Repository`, but the usecase must translate it to `UsecaseError::Repository(DomainError::FkSdtmProjectVersionNotFound(cmd.version_id))`. Implement by matching the error: `match self.version_repo.find_by_id(cmd.version_id).await { Ok(_) => {}, Err(DomainError::SdtmProjectVersionNotFound(_)) => return Err(UsecaseError::Repository(DomainError::FkSdtmProjectVersionNotFound(cmd.version_id))), Err(e) => return Err(UsecaseError::Repository(e)) }`; then `self.domain_repo.create(SdtmProjectDomainNew { version_id: cmd.version_id, domain_code: cmd.domain_code, key_variables: normalised }).await?`.
- `get_domain_by_id` / `list_domains_by_version` — delegate to `domain_repo` and map with `Into::into`. The override is already hydrated by the repository.
- `update_domain` — validate `domain_code` when present; `self.domain_repo.update(SdtmProjectDomainUpdate { id: cmd.id, domain_code: cmd.domain_code, key_variables: cmd.key_variables.map(|k| SdtmProjectDomain::normalize_key_variables(&k)).transpose().map_err(UsecaseError::Validation)? }).await?`.
- `delete_domain` — delegate.
- `create_domain_override` — **check the parent first**: `domain_repo.find_by_id(cmd.domain_id)`; a missing domain becomes `UsecaseError::Repository(DomainError::FkSdtmProjectDomainNotFound(cmd.domain_id))`; then `self.domain_override_repo.create(SdtmProjectDomainOverrideNew { domain_id: cmd.domain_id, category: cmd.category, descriptions: cmd.descriptions }).await?`.
- `get_domain_override` — the override is **optional**: match on the repository result and turn `DomainError::SdtmProjectDomainOverrideNotFound(_)` into `Ok(None)`; every other error is `Err(UsecaseError::Repository(e))`.
- `update_domain_override` — `self.domain_override_repo.update(SdtmProjectDomainOverrideUpdate { domain_id: cmd.domain_id, category: cmd.category, descriptions: cmd.descriptions }).await?`.
- `delete_domain_override` — `self.domain_override_repo.delete(domain_id).await?`.
- `create_variable` — validate `variable_code` non-blank → `Validation(EmptyVariableCode)`; **check the parent first** via `domain_repo.find_by_id(cmd.domain_id)`, mapping a missing domain to `UsecaseError::Repository(DomainError::FkSdtmProjectDomainNotFound(cmd.domain_id))`; then `self.variable_repo.create(SdtmProjectVariableNew { domain_id: cmd.domain_id, variable_code: cmd.variable_code, variable_sequence: cmd.variable_sequence }).await?`.
- `get_variable_by_id` / `list_variables_by_domain` / `update_variable` / `delete_variable` — mirror the domain ones. `update_variable` validates `variable_code` when present.
- `create_variable_override` — **check the parent first** via `variable_repo.find_by_id(cmd.variable_id)`, mapping a missing variable to `UsecaseError::Repository(DomainError::FkSdtmProjectVariableNotFound(cmd.variable_id))`; then `self.variable_override_repo.create(SdtmProjectVariableOverrideNew { variable_id: cmd.variable_id, variable_type: cmd.variable_type, variable_core: cmd.variable_core, variable_role: cmd.variable_role, descriptions: cmd.descriptions }).await?`.
- `get_variable_override` / `update_variable_override` / `delete_variable_override` — mirror the domain-override ones; `get_variable_override` returns `Ok(None)` for a missing override, like its domain counterpart. `update_variable_override` passes `variable_role` straight through — it is already `Option<Option<SdtmRole>>` and the repository implements the three-state semantics.

- [ ] **Step 7: Wire up the module and re-exports**

`src/usecase.rs`:

```rust
mod commands;
mod error;
mod sdtm_project_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion, UpdateSdtmProjectDomain,
    UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable,
    UpdateSdtmProjectVariableOverride, UpdateSdtmProjectVersion,
};
pub use error::UsecaseError;
pub use sdtm_project_usecase::{SdtmProjectUsecase, SdtmProjectUsecaseConfig};
pub use views::{
    SdtmProjectDomainOverrideView, SdtmProjectDomainView, SdtmProjectVariableOverrideView,
    SdtmProjectVariableView, SdtmProjectVersionView,
};
```

In `src/lib.rs` replace the placeholder with `pub mod usecase;` and add:

```rust
pub use usecase::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion, SdtmProjectDomainOverrideView,
    SdtmProjectDomainView, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
    SdtmProjectVariableOverrideView, SdtmProjectVariableView, SdtmProjectVersionView,
    UpdateSdtmProjectDomain, UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable,
    UpdateSdtmProjectVariableOverride, UpdateSdtmProjectVersion, UsecaseError,
};
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p sdtm-project`
Expected: PASS — the 21 domain tests plus the 14 usecase tests.

- [ ] **Step 9: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 10: Commit**

```bash
git add lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
feat(sdtm-project): add the usecase layer

Twenty CRUD methods across the five aggregates, the command and view DTOs,
UsecaseError, and a generic SdtmProjectUsecase over the five repository
ports plus the project lookup. Every create_* checks its parent first and
reports a missing parent as Fk*NotFound, which the HTTP layer maps to 404.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p sdtm-project
         cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Migrations and the version repository

**Files:**
- Create: `lib/crates/sdtm-project/migrations/0001_create_sdtm_project_versions.sql`
- Create: `lib/crates/sdtm-project/migrations/0002_create_sdtm_project_domains.sql`
- Create: `lib/crates/sdtm-project/migrations/0003_create_sdtm_project_domain_overrides.sql`
- Create: `lib/crates/sdtm-project/migrations/0004_create_sdtm_project_variables.sql`
- Create: `lib/crates/sdtm-project/migrations/0005_create_sdtm_project_variable_overrides.sql`
- Create: `lib/crates/sdtm-project/src/adapter.rs`
- Create: `lib/crates/sdtm-project/src/adapter/persistence.rs`
- Create: `lib/crates/sdtm-project/src/adapter/persistence/postgres.rs`
- Create: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_version_repo.rs`

**Interfaces:**
- Consumes: `SdtmProjectVersion`, `SdtmProjectVersionNew`, `SdtmProjectVersionUpdate`, `SdtmProjectVersionRepository`, `DomainError` from Task 1.
- Produces:
  - `SdtmProjectVersionRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`, implementing `SdtmProjectVersionRepository`. Re-exported from the crate root.
  - The five migration files (used by Task 4, Task 5 and Task 8).

- [ ] **Step 1: Write the five migration files**

`0001_create_sdtm_project_versions.sql`:

```sql
-- sdtm_project_versions: one row per (project, version name).
-- `project_code` is a soft reference to `projects.code` from the
-- `project` crate; it is validated at the application level, not by a
-- database foreign key, so the two crates' schemas stay independent.

CREATE TABLE IF NOT EXISTS sdtm_project_versions (
    id            BIGSERIAL PRIMARY KEY,
    project_code  TEXT        NOT NULL,
    version_name  TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_versions_project_name_unique UNIQUE (project_code, version_name)
);

CREATE OR REPLACE FUNCTION sdtm_project_versions_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_versions_updated_at
BEFORE UPDATE ON sdtm_project_versions
FOR EACH ROW EXECUTE FUNCTION sdtm_project_versions_set_updated_at();
```

`0002_create_sdtm_project_domains.sql`:

```sql
-- sdtm_project_domains: one row per (version, domain code).
-- `key_variables` is a JSONB array of variable codes kept in the order
-- the caller supplied.

CREATE TABLE IF NOT EXISTS sdtm_project_domains (
    id             BIGSERIAL PRIMARY KEY,
    version_id     BIGINT      NOT NULL REFERENCES sdtm_project_versions(id) ON DELETE CASCADE,
    domain_code    TEXT        NOT NULL,
    key_variables  JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (version_id, domain_code),
    CONSTRAINT sdtm_project_domains_key_variables_is_array CHECK (jsonb_typeof(key_variables) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_domains_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_domains_updated_at
BEFORE UPDATE ON sdtm_project_domains
FOR EACH ROW EXECUTE FUNCTION sdtm_project_domains_set_updated_at();
```

`0003_create_sdtm_project_domain_overrides.sql`:

```sql
-- sdtm_project_domain_overrides: at most one override per domain.
-- `UNIQUE (domain_id)` is what enforces "each domain can only hold one
-- override".

CREATE TABLE IF NOT EXISTS sdtm_project_domain_overrides (
    id            BIGSERIAL PRIMARY KEY,
    domain_id     BIGINT      NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE,
    category      TEXT        NOT NULL,
    descriptions  JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_domain_overrides_domain_unique UNIQUE (domain_id),
    CONSTRAINT sdtm_project_domain_overrides_category_check CHECK (
        category IN (
            'Special Purpose',
            'Interventions',
            'Events',
            'Findings',
            'Trial Design',
            'Relationships',
            'Study Reference'
        )
    ),
    CONSTRAINT sdtm_project_domain_overrides_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_domain_overrides_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_domain_overrides_updated_at
BEFORE UPDATE ON sdtm_project_domain_overrides
FOR EACH ROW EXECUTE FUNCTION sdtm_project_domain_overrides_set_updated_at();
```

`0004_create_sdtm_project_variables.sql`:

```sql
-- sdtm_project_variables: one row per (domain, variable code).

CREATE TABLE IF NOT EXISTS sdtm_project_variables (
    id                BIGSERIAL PRIMARY KEY,
    domain_id         BIGINT      NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE,
    variable_code     TEXT        NOT NULL,
    variable_sequence BIGINT      NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (domain_id, variable_code)
);

CREATE OR REPLACE FUNCTION sdtm_project_variables_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_variables_updated_at
BEFORE UPDATE ON sdtm_project_variables
FOR EACH ROW EXECUTE FUNCTION sdtm_project_variables_set_updated_at();
```

`0005_create_sdtm_project_variable_overrides.sql`:

```sql
-- sdtm_project_variable_overrides: at most one override per variable.

CREATE TABLE IF NOT EXISTS sdtm_project_variable_overrides (
    id             BIGSERIAL PRIMARY KEY,
    variable_id    BIGINT      NOT NULL REFERENCES sdtm_project_variables(id) ON DELETE CASCADE,
    variable_type  TEXT        NOT NULL,
    variable_core  TEXT        NOT NULL,
    variable_role  TEXT,
    descriptions   JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_variable_overrides_variable_unique UNIQUE (variable_id),
    CONSTRAINT sdtm_project_variable_overrides_type_check CHECK (
        variable_type IN ('Numeric', 'Character')
    ),
    CONSTRAINT sdtm_project_variable_overrides_core_check CHECK (
        variable_core IN ('Req', 'Exp', 'Perm', 'Supp')
    ),
    CONSTRAINT sdtm_project_variable_overrides_role_check CHECK (
        variable_role IS NULL OR variable_role IN (
            'Identifier',
            'Topic',
            'Timing',
            'Record Qualifier',
            'Synonym Qualifier',
            'Variable Qualifier',
            'Grouping Qualifier',
            'Rule'
        )
    ),
    CONSTRAINT sdtm_project_variable_overrides_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_variable_overrides_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_variable_overrides_updated_at
BEFORE UPDATE ON sdtm_project_variable_overrides
FOR EACH ROW EXECUTE FUNCTION sdtm_project_variable_overrides_set_updated_at();
```

- [ ] **Step 2: Write the failing adapter unit test**

In `src/adapter/persistence/postgres/sdtm_project_version_repo.rs`, add a `#[cfg(test)] mod tests` with:

`migration_file_declares_version_table` — `let sql = include_str!("../../../../migrations/0001_create_sdtm_project_versions.sql");` then assert `sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_versions")`, `sql.contains("CONSTRAINT sdtm_project_versions_project_name_unique UNIQUE (project_code, version_name)")`, `sql.contains("sdtm_project_versions_updated_at")`.

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p sdtm-project migration_file_declares_version_table`
Expected: compile failure — the adapter module tree does not exist yet.

- [ ] **Step 4: Create the adapter module tree**

`src/adapter.rs`:

```rust
//! Adapter layer.
//!
//! Hosts three sub-modules:
//! - `persistence` — SQLx/PostgreSQL implementations of the repository
//!   ports.
//! - `service` — adapters that bridge another workspace crate's port
//!   into a domain port.
//! - `facade` — implementations of the API trait defined in
//!   `apis::sdtm_project::SdtmProjectService`.

pub mod facade;
pub mod persistence;
pub mod service;
```

Create `src/adapter/persistence.rs` with a module-level doc comment stating that the SQLx runtime API is used throughout because the workspace ships no `.sqlx/` cache, and `pub mod postgres;`.

Create `src/adapter/persistence/postgres.rs` declaring `pub mod sdtm_project_version_repo;` — Task 4 and Task 5 add the other four modules here.

Create `src/adapter/service.rs` and `src/adapter/facade.rs` as doc-comment-only placeholders for now.

- [ ] **Step 5: Implement the version repository**

In `src/adapter/persistence/postgres/sdtm_project_version_repo.rs`:

- A module-level doc comment explaining the runtime-API choice.
- `#[derive(FromRow)] struct SdtmProjectVersionRow { id: i64, project_code: String, version_name: String, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }` with `impl From<SdtmProjectVersionRow> for SdtmProjectVersion` calling `SdtmProjectVersion::for_repository(...)`.
- `#[derive(Clone)] pub struct SdtmProjectVersionRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`.
- `#[async_trait] impl SdtmProjectVersionRepository for SdtmProjectVersionRepoPg`:
  - `create` — `INSERT INTO sdtm_project_versions (project_code, version_name) VALUES ($1, $2) RETURNING id, project_code, version_name, created_at, updated_at`; on error map with `map_version_err`.
  - `find_by_id` — `SELECT id, project_code, version_name, created_at, updated_at FROM sdtm_project_versions WHERE id = $1`; `fetch_optional`; a missing row is `DomainError::SdtmProjectVersionNotFound(id)` (**not** `RowNotFound`).
  - `list_by_project` — `SELECT … WHERE project_code = $1 ORDER BY id ASC`; `fetch_all`.
  - `update` — `UPDATE sdtm_project_versions SET version_name = COALESCE($2, version_name) WHERE id = $1 RETURNING …`; `fetch_optional`; a missing row is `DomainError::SdtmProjectVersionNotFound(input.id)`.
  - `delete` — `DELETE FROM sdtm_project_versions WHERE id = $1`; zero rows affected is `DomainError::SdtmProjectVersionNotFound(id)`.
- `fn map_version_err(err: sqlx::Error) -> DomainError` — SQLSTATE `23505` → `DomainError::DuplicateSdtmProjectVersion { project_code: "(unknown)".into(), version_name: "(unknown)".into() }`; `23503` → `DomainError::FkSdtmProjectVersionNotFound(0)`; `sqlx::Error::RowNotFound` → `DomainError::NotFound`; anything else → `DomainError::Repository(err.to_string())`. Add a comment noting that the natural key is not recoverable from the driver error, so the placeholders are filled in by the caller's own pre-check.

- [ ] **Step 6: Re-export from the crate root**

In `src/lib.rs` add `pub use adapter::persistence::postgres::sdtm_project_version_repo::SdtmProjectVersionRepoPg;`.

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test -p sdtm-project`
Expected: PASS — 21 domain + 14 usecase + 1 adapter unit test.

- [ ] **Step 8: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 9: Commit**

```bash
git add lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
feat(sdtm-project): add the schema and the version repository

Five migrations covering the version, domain, domain-override, variable and
variable-override tables, each with an updated_at trigger and the CHECK
constraints the domain enums imply. The version repository is the first
PostgreSQL adapter.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p sdtm-project
         cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Domain repository and domain-override repository

**Files:**
- Create: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_domain_repo.rs`
- Create: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_domain_override_repo.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/persistence/postgres.rs`
- Modify: `lib/crates/sdtm-project/src/lib.rs`

**Interfaces:**
- Consumes: Task 1's domain aggregates and ports; Task 3's migrations.
- Produces:
  - `SdtmProjectDomainRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`, implementing `SdtmProjectDomainRepository`. Its `find_by_id`, `list_by_version` and `update` return domains whose `domain_override` is hydrated from `sdtm_project_domain_overrides`.
  - `SdtmProjectDomainOverrideRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`, implementing `SdtmProjectDomainOverrideRepository`. It is the **only writer** of `sdtm_project_domain_overrides`.
  - Both re-exported from the crate root.

- [ ] **Step 1: Write the failing adapter unit tests**

Each new repo file gets a `#[cfg(test)] mod tests` reading its migration with `include_str!`:

`sdtm_project_domain_repo.rs` → `migration_file_declares_domain_tables`:
`include_str!("../../../../migrations/0002_create_sdtm_project_domains.sql")`; assert it contains `"CREATE TABLE IF NOT EXISTS sdtm_project_domains"`, `"CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (version_id, domain_code)"`, `"key_variables  JSONB"`, `"sdtm_project_domains_updated_at"`.

`sdtm_project_domain_override_repo.rs` → `migration_file_declares_domain_override_table`:
`include_str!("../../../../migrations/0003_create_sdtm_project_domain_overrides.sql")`; assert it contains `"CREATE TABLE IF NOT EXISTS sdtm_project_domain_overrides"`, `"CONSTRAINT sdtm_project_domain_overrides_domain_unique UNIQUE (domain_id)"`, `"sdtm_project_domain_overrides_category_check"`, `"sdtm_project_domain_overrides_updated_at"`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sdtm-project migration_file_declares_domain`
Expected: compile failure — the modules do not exist yet.

- [ ] **Step 3: Implement the domain repository with override hydration**

`sdtm_project_domain_repo.rs`:

- Row struct (one column list serves every query):

```rust
#[derive(FromRow)]
struct SdtmProjectDomainRow {
    id: i64,
    version_id: i64,
    domain_code: String,
    key_variables: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // The override columns come from a LEFT JOIN and are all NULL when
    // the domain has no override.
    override_id: Option<i64>,
    override_category: Option<String>,
    override_descriptions: Option<JsonValue>,
}
```

- `impl SdtmProjectDomainRow { fn into_domain(self) -> Result<SdtmProjectDomain, DomainError> }` — deserialises `key_variables` into `Vec<String>`; when `override_id` is `Some`, parses `override_category` with `DomainCategory::try_from(..)?` and `override_descriptions` into `Vec<SdtmDomainDescription>`, building `Some(SdtmProjectDomainOverride { .. })`; otherwise `None`. A malformed category or descriptions blob is `DomainError::Repository(msg)`.

- `#[derive(Clone)] pub struct SdtmProjectDomainRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`.

- The read queries use a `LEFT JOIN` so the override is hydrated in one round trip:

```sql
SELECT d.id, d.version_id, d.domain_code, d.key_variables,
       d.created_at, d.updated_at,
       o.id AS override_id,
       o.category AS override_category,
       o.descriptions AS override_descriptions
FROM sdtm_project_domains d
LEFT JOIN sdtm_project_domain_overrides o ON o.domain_id = d.id
WHERE d.id = $1
```

and the same shape with `WHERE d.version_id = $1 ORDER BY d.id ASC` for `list_by_version`.

- `create` uses a data-modifying CTE so the returning row has the same column list (a fresh domain has no override, so the LEFT JOIN yields NULLs):

```sql
WITH inserted AS (
    INSERT INTO sdtm_project_domains (version_id, domain_code, key_variables)
    VALUES ($1, $2, $3)
    RETURNING id, version_id, domain_code, key_variables, created_at, updated_at
)
SELECT i.id, i.version_id, i.domain_code, i.key_variables,
       i.created_at, i.updated_at,
       o.id AS override_id,
       o.category AS override_category,
       o.descriptions AS override_descriptions
FROM inserted i
LEFT JOIN sdtm_project_domain_overrides o ON o.domain_id = i.id
```

- `update` uses the same CTE shape over an UPDATE, so the response carries the override:

```sql
WITH updated AS (
    UPDATE sdtm_project_domains
    SET domain_code   = COALESCE($2, domain_code),
        key_variables = COALESCE($3, key_variables)
    WHERE id = $1
    RETURNING id, version_id, domain_code, key_variables, created_at, updated_at
)
SELECT u.id, u.version_id, u.domain_code, u.key_variables,
       u.created_at, u.updated_at,
       o.id AS override_id,
       o.category AS override_category,
       o.descriptions AS override_descriptions
FROM updated u
LEFT JOIN sdtm_project_domain_overrides o ON o.domain_id = u.id
```

`$3` binds the JSONB array; a `Some(vec![])` binds `[]` (not NULL), so `COALESCE` keeps it and the column is cleared to an empty array rather than left untouched.

- `delete` — `DELETE FROM sdtm_project_domains WHERE id = $1`; zero rows affected is `DomainError::SdtmProjectDomainNotFound(id)`.

- `fn map_domain_err(err: sqlx::Error) -> DomainError` — `23505` → `DomainError::DuplicateSdtmProjectDomain { version_id: 0, domain_code: "(unknown)".into() }`; `23503` → `DomainError::FkSdtmProjectVersionNotFound(0)`; `RowNotFound` → `DomainError::NotFound`; else `Repository`.

- `key_variables` is bound as a `serde_json::Value` built from the `Vec<String>`: `serde_json::to_value(&input.key_variables).map_err(|e| DomainError::Repository(e.to_string()))?`. The order is whatever the vector already holds — never re-sorted.

- [ ] **Step 4: Implement the domain-override repository**

`sdtm_project_domain_override_repo.rs`:

- Row struct `SdtmProjectDomainOverrideRow { id: i64, domain_id: i64, category: String, descriptions: JsonValue, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }` with `into_override()`.
- `#[derive(Clone)] pub struct SdtmProjectDomainOverrideRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`.
- `create` — `INSERT INTO sdtm_project_domain_overrides (domain_id, category, descriptions) VALUES ($1, $2, $3) RETURNING id, domain_id, category, descriptions, created_at, updated_at`.
- `find_by_domain_id` — `SELECT … WHERE domain_id = $1`; `fetch_optional`; a missing row is `DomainError::SdtmProjectDomainOverrideNotFound(domain_id)`.
- `update` — `UPDATE sdtm_project_domain_overrides SET category = COALESCE($2, category), descriptions = COALESCE($3, descriptions) WHERE domain_id = $1 RETURNING …`; `fetch_optional`; a missing row is `DomainError::SdtmProjectDomainOverrideNotFound(input.domain_id)`.
- `delete` — `DELETE FROM sdtm_project_domain_overrides WHERE domain_id = $1`; zero rows affected is `DomainError::SdtmProjectDomainOverrideNotFound(domain_id)`.
- `fn map_domain_override_err(err: sqlx::Error) -> DomainError` — `23505` → `DomainError::DuplicateSdtmProjectDomainOverride { domain_id: 0 }`; `23503` → `DomainError::FkSdtmProjectDomainNotFound(0)`; `RowNotFound` → `NotFound`; else `Repository`.

- [ ] **Step 5: Register the modules and re-exports**

Add to `src/adapter/persistence/postgres.rs`:

```rust
pub mod sdtm_project_domain_override_repo;
pub mod sdtm_project_domain_repo;
pub mod sdtm_project_version_repo;
```

Add to `src/lib.rs`:

```rust
pub use adapter::persistence::postgres::{
    sdtm_project_domain_override_repo::SdtmProjectDomainOverrideRepoPg,
    sdtm_project_domain_repo::SdtmProjectDomainRepoPg,
    sdtm_project_version_repo::SdtmProjectVersionRepoPg,
};
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p sdtm-project`
Expected: PASS.

- [ ] **Step 7: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
feat(sdtm-project): add the domain and domain-override repositories

The domain repository hydrates the parent row's override with a LEFT JOIN
on every read, including the UPDATE path, so PUT and GET return the same
shape. The override repository is the only writer of the override table.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p sdtm-project
         cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Variable repository and variable-override repository

**Files:**
- Create: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_variable_repo.rs`
- Create: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_variable_override_repo.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/persistence/postgres.rs`
- Modify: `lib/crates/sdtm-project/src/lib.rs`

**Interfaces:**
- Consumes: Task 1's variable aggregates and ports; Task 3's migrations.
- Produces:
  - `SdtmProjectVariableRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`, implementing `SdtmProjectVariableRepository`. Its `find_by_id`, `list_by_domain` and `update` hydrate `variable_override` from `sdtm_project_variable_overrides`.
  - `SdtmProjectVariableOverrideRepoPg { pool: PgPool }` with `pub fn new(pool: PgPool) -> Self`, implementing `SdtmProjectVariableOverrideRepository`. It is the **only writer** of `sdtm_project_variable_overrides`.
  - Both re-exported from the crate root.

- [ ] **Step 1: Write the failing adapter unit tests**

`sdtm_project_variable_repo.rs` → `migration_file_declares_variable_table`:
`include_str!("../../../../migrations/0004_create_sdtm_project_variables.sql")`; assert `"CREATE TABLE IF NOT EXISTS sdtm_project_variables"`, `"CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (domain_id, variable_code)"`, `"sdtm_project_variables_updated_at"`.

`sdtm_project_variable_override_repo.rs` → `migration_file_declares_variable_override_table`:
`include_str!("../../../../migrations/0005_create_sdtm_project_variable_overrides.sql")`; assert `"CREATE TABLE IF NOT EXISTS sdtm_project_variable_overrides"`, `"CONSTRAINT sdtm_project_variable_overrides_variable_unique UNIQUE (variable_id)"`, `"sdtm_project_variable_overrides_type_check"`, `"sdtm_project_variable_overrides_core_check"`, `"sdtm_project_variable_overrides_role_check"`, `"sdtm_project_variable_overrides_updated_at"`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sdtm-project migration_file_declares_variable`
Expected: compile failure — the modules do not exist yet.

- [ ] **Step 3: Implement the variable repository with override hydration**

`sdtm_project_variable_repo.rs` mirrors Task 4's domain repository:

- Row struct `SdtmProjectVariableRow { id: i64, domain_id: i64, variable_code: String, variable_sequence: i64, created_at: DateTime<Utc>, updated_at: DateTime<Utc>, override_id: Option<i64>, override_variable_type: Option<String>, override_variable_core: Option<String>, override_variable_role: Option<String>, override_descriptions: Option<JsonValue> }` with `into_variable()`.
- `find_by_id` / `list_by_domain` use the `LEFT JOIN` shape:

```sql
SELECT v.id, v.domain_id, v.variable_code, v.variable_sequence,
       v.created_at, v.updated_at,
       o.id AS override_id,
       o.variable_type AS override_variable_type,
       o.variable_core AS override_variable_core,
       o.variable_role AS override_variable_role,
       o.descriptions AS override_descriptions
FROM sdtm_project_variables v
LEFT JOIN sdtm_project_variable_overrides o ON o.variable_id = v.id
WHERE v.id = $1
```

`list_by_domain` adds `WHERE v.domain_id = $1 ORDER BY v.variable_sequence ASC, v.id ASC`.

- `create` and `update` use the data-modifying CTE shape from Task 4 (`WITH inserted AS (… RETURNING …) SELECT … FROM inserted i LEFT JOIN …`, and the same for `WITH updated AS (… UPDATE … RETURNING …)`), so every returned variable carries the same column list.

- `update` sets `variable_code = COALESCE($2, variable_code)` and `variable_sequence = COALESCE($3, variable_sequence)`.

- `delete` — zero rows affected is `DomainError::SdtmProjectVariableNotFound(id)`.

- `fn map_variable_err(err: sqlx::Error) -> DomainError` — `23505` → `DomainError::DuplicateSdtmProjectVariable { domain_id: 0, variable_code: "(unknown)".into() }`; `23503` → `DomainError::FkSdtmProjectDomainNotFound(0)`; `RowNotFound` → `NotFound`; else `Repository`.

- [ ] **Step 4: Implement the variable-override repository**

`sdtm_project_variable_override_repo.rs` mirrors Task 4's override repository:

- Row struct `SdtmProjectVariableOverrideRow { id: i64, variable_id: i64, variable_type: String, variable_core: String, variable_role: Option<String>, descriptions: JsonValue, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }` with `into_override()`. The three enum columns parse with `SdtmVariableType::try_from`, `SdtmVariableCore::try_from`, `SdtmRole::try_from`.
- `create` — `INSERT INTO sdtm_project_variable_overrides (variable_id, variable_type, variable_core, variable_role, descriptions) VALUES ($1, $2, $3, $4, $5) RETURNING …`.
- `find_by_variable_id` — `SELECT … WHERE variable_id = $1`; missing → `DomainError::SdtmProjectVariableOverrideNotFound(variable_id)`.
- `update` — three-state `variable_role`:

```sql
UPDATE sdtm_project_variable_overrides SET
    variable_type = COALESCE($2, variable_type),
    variable_core = COALESCE($3, variable_core),
    descriptions  = COALESCE($5, descriptions),
    variable_role = $4
WHERE variable_id = $1
RETURNING id, variable_id, variable_type, variable_core, variable_role, descriptions, created_at, updated_at
```

  The three-state mapping on the `variable_role` column is: outer `None` → `COALESCE($4, variable_role)` (leave unchanged); outer `Some(None)` → `$4` bound as `NULL` (clear); outer `Some(Some(role))` → `$4` bound as the role string (replace). Pick the SQL expression per call, exactly as `domain-model`'s `sdtm_variable_repo.rs` does — it builds the statement with `format!` and `sqlx::query_as::<_, Row>(AssertSqlSafe(sql))`.

  Missing row → `DomainError::SdtmProjectVariableOverrideNotFound(input.variable_id)`.
- `delete` — `DELETE FROM sdtm_project_variable_overrides WHERE variable_id = $1`; zero rows affected is `DomainError::SdtmProjectVariableOverrideNotFound(variable_id)`.
- `fn map_variable_override_err(err: sqlx::Error) -> DomainError` — `23505` → `DomainError::DuplicateSdtmProjectVariableOverride { variable_id: 0 }`; `23503` → `DomainError::FkSdtmProjectVariableNotFound(0)`; `RowNotFound` → `NotFound`; else `Repository`.

- [ ] **Step 5: Register the modules and re-exports**

Add to `src/adapter/persistence/postgres.rs`:

```rust
pub mod sdtm_project_domain_override_repo;
pub mod sdtm_project_domain_repo;
pub mod sdtm_project_variable_override_repo;
pub mod sdtm_project_variable_repo;
pub mod sdtm_project_version_repo;
```

Add to `src/lib.rs`:

```rust
pub use adapter::persistence::postgres::{
    sdtm_project_domain_override_repo::SdtmProjectDomainOverrideRepoPg,
    sdtm_project_domain_repo::SdtmProjectDomainRepoPg,
    sdtm_project_variable_override_repo::SdtmProjectVariableOverrideRepoPg,
    sdtm_project_variable_repo::SdtmProjectVariableRepoPg,
    sdtm_project_version_repo::SdtmProjectVersionRepoPg,
};
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p sdtm-project`
Expected: PASS.

- [ ] **Step 7: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
feat(sdtm-project): add the variable and variable-override repositories

Same hydration shape as the domain pair: the variable repository LEFT JOINs
its override on every read including the UPDATE path, and the override
repository is the only writer of the override table.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p sdtm-project
         cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: The `apis::sdtm_project` port

**Files:**
- Create: `lib/crates/apis/src/sdtm_project.rs`
- Modify: `lib/crates/apis/src/lib.rs`
- Modify: `lib/crates/apis/tests/public_api.rs`

**Interfaces:**
- Consumes: nothing from the crate (the `apis` crate is dependency-free — it depends on no other workspace crate).
- Produces (all under `apis::sdtm_project`):
  - Copied value objects: `DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole` — same variants, serde renames and `as_str()` bodies as the crate's copies.
  - `SdtmDomainDescription { pub lang: String, pub details: SdtmDomainDescriptionDetail }`, `SdtmDomainDescriptionDetail { pub description: String, pub structure: String }`, `SdtmVariableDescription { pub lang: String, pub details: SdtmVariableDescriptionDetail }`, `SdtmVariableDescriptionDetail { pub label: String }` — `Debug + Clone + PartialEq + Eq + Serialize + Deserialize`.
  - `SdtmProjectApiError` — variants: `Validation(String)`, `NotFound`, `ProjectNotFound(String)`, `SdtmProjectVersionNotFound(i64)`, `SdtmProjectDomainNotFound(i64)`, `SdtmProjectDomainOverrideNotFound(i64)`, `SdtmProjectVariableNotFound(i64)`, `SdtmProjectVariableOverrideNotFound(i64)`, `DuplicateSdtmProjectVersion { project_code: String, version_name: String }`, `DuplicateSdtmProjectDomain { version_id: i64, domain_code: String }`, `DuplicateSdtmProjectDomainOverride { domain_id: i64 }`, `DuplicateSdtmProjectVariable { domain_id: i64, variable_code: String }`, `DuplicateSdtmProjectVariableOverride { variable_id: i64 }`, `FkSdtmProjectVersionNotFound(i64)`, `FkSdtmProjectDomainNotFound(i64)`, `FkSdtmProjectVariableNotFound(i64)`, `Repository(String)`. `#[derive(Debug, Clone, Error)]`.
  - Views: `SdtmProjectVersionView { id: i64, project_code: String, version_name: String, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectDomainView { id: i64, version_id: i64, domain_code: String, key_variables: Vec<String>, domain_override: Option<SdtmProjectDomainOverrideView>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectDomainOverrideView { id: i64, domain_id: i64, category: DomainCategory, descriptions: Vec<SdtmDomainDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectVariableView { id: i64, domain_id: i64, variable_code: String, variable_sequence: i64, variable_override: Option<SdtmProjectVariableOverrideView>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectVariableOverrideView { id: i64, variable_id: i64, variable_type: SdtmVariableType, variable_core: SdtmVariableCore, variable_role: Option<SdtmRole>, descriptions: Vec<SdtmVariableDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`. All `Debug + Clone + PartialEq + Eq`.
  - Lists: `SdtmProjectVersionList { pub versions: Vec<SdtmProjectVersionView> }`, `SdtmProjectDomainList { pub domains: Vec<SdtmProjectDomainView> }`, `SdtmProjectVariableList { pub variables: Vec<SdtmProjectVariableView> }`.
  - Requests (one per CRUD pair): `CreateSdtmProjectVersionRequest { project_code: String, version_name: String }`, `UpdateSdtmProjectVersionRequest { id: i64, version_name: Option<String> }`, `CreateSdtmProjectDomainRequest { version_id: i64, domain_code: String, key_variables: Vec<String> }`, `UpdateSdtmProjectDomainRequest { id: i64, domain_code: Option<String>, key_variables: Option<Vec<String>> }`, `CreateSdtmProjectDomainOverrideRequest { domain_id: i64, category: DomainCategory, descriptions: Vec<SdtmDomainDescription> }`, `UpdateSdtmProjectDomainOverrideRequest { domain_id: i64, category: Option<DomainCategory>, descriptions: Option<Vec<SdtmDomainDescription>> }`, `CreateSdtmProjectVariableRequest { domain_id: i64, variable_code: String, variable_sequence: i64 }`, `UpdateSdtmProjectVariableRequest { id: i64, variable_code: Option<String>, variable_sequence: Option<i64> }`, `CreateSdtmProjectVariableOverrideRequest { variable_id: i64, variable_type: SdtmVariableType, variable_core: SdtmVariableCore, variable_role: Option<SdtmRole>, descriptions: Vec<SdtmVariableDescription> }`, `UpdateSdtmProjectVariableOverrideRequest { variable_id: i64, variable_type: Option<SdtmVariableType>, variable_core: Option<SdtmVariableCore>, variable_role: Option<Option<SdtmRole>>, descriptions: Option<Vec<SdtmVariableDescription>> }`. All `Debug + Clone`; the `Update*` ones also derive `Default`.
  - The trait `SdtmProjectService: Send + Sync` with the twenty methods, mirroring the usecase's twenty:

    ```
    async fn create_version(&self, req: CreateSdtmProjectVersionRequest) -> Result<SdtmProjectVersionView, SdtmProjectApiError>
    async fn get_version_by_id(&self, id: i64) -> Result<SdtmProjectVersionView, SdtmProjectApiError>
    async fn list_versions_by_project(&self, project_code: &str) -> Result<SdtmProjectVersionList, SdtmProjectApiError>
    async fn update_version(&self, req: UpdateSdtmProjectVersionRequest) -> Result<SdtmProjectVersionView, SdtmProjectApiError>
    async fn delete_version(&self, id: i64) -> Result<(), SdtmProjectApiError>

    async fn create_domain(&self, req: CreateSdtmProjectDomainRequest) -> Result<SdtmProjectDomainView, SdtmProjectApiError>
    async fn get_domain_by_id(&self, id: i64) -> Result<SdtmProjectDomainView, SdtmProjectApiError>
    async fn list_domains_by_version(&self, version_id: i64) -> Result<SdtmProjectDomainList, SdtmProjectApiError>
    async fn update_domain(&self, req: UpdateSdtmProjectDomainRequest) -> Result<SdtmProjectDomainView, SdtmProjectApiError>
    async fn delete_domain(&self, id: i64) -> Result<(), SdtmProjectApiError>

    async fn create_domain_override(&self, req: CreateSdtmProjectDomainOverrideRequest) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError>
    async fn get_domain_override(&self, domain_id: i64) -> Result<Option<SdtmProjectDomainOverrideView>, SdtmProjectApiError>
    async fn update_domain_override(&self, req: UpdateSdtmProjectDomainOverrideRequest) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError>
    async fn delete_domain_override(&self, domain_id: i64) -> Result<(), SdtmProjectApiError>

    async fn create_variable(&self, req: CreateSdtmProjectVariableRequest) -> Result<SdtmProjectVariableView, SdtmProjectApiError>
    async fn get_variable_by_id(&self, id: i64) -> Result<SdtmProjectVariableView, SdtmProjectApiError>
    async fn list_variables_by_domain(&self, domain_id: i64) -> Result<SdtmProjectVariableList, SdtmProjectApiError>
    async fn update_variable(&self, req: UpdateSdtmProjectVariableRequest) -> Result<SdtmProjectVariableView, SdtmProjectApiError>
    async fn delete_variable(&self, id: i64) -> Result<(), SdtmProjectApiError>

    async fn create_variable_override(&self, req: CreateSdtmProjectVariableOverrideRequest) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError>
    async fn get_variable_override(&self, variable_id: i64) -> Result<Option<SdtmProjectVariableOverrideView>, SdtmProjectApiError>
    async fn update_variable_override(&self, req: UpdateSdtmProjectVariableOverrideRequest) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError>
    async fn delete_variable_override(&self, variable_id: i64) -> Result<(), SdtmProjectApiError>
    ```

- [ ] **Step 1: Write the failing apis public-API test**

Add to `lib/crates/apis/tests/public_api.rs`:

`sdtm_project_public_types_are_nameable` — construct one value of each public type: `DomainCategory::Events`, `SdtmVariableType::Character`, `SdtmVariableCore::Req`, `SdtmRole::Topic`, a `SdtmDomainDescription`, a `SdtmVariableDescription`, each of the five views (with `key_variables: vec!["AETERM".into()]` and `domain_override: None` / `variable_override: None`), each of the three lists, and one of each of the ten requests. Touch `SdtmProjectApiError::NotFound`.

`sdtm_project_service_trait_is_object_safe` — declare `let _: Option<Box<dyn SdtmProjectService>> = None;` and `fn assert_send_sync<T: Send + Sync + ?Sized>() {}` applied to `Box<dyn SdtmProjectService>`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apis`
Expected: compile failure — `apis::sdtm_project` does not exist.

- [ ] **Step 3: Create the port module**

Create `lib/crates/apis/src/sdtm_project.rs` with a module doc comment explaining that it is the outbound port for the per-project SDTM model tree and that it re-declares the value objects so the crate stays dependency-free.

Declare the copied enums, the description DTOs, the error enum, the views, the lists, the requests and the trait, exactly as listed in Interfaces. Copy the enum bodies from `lib/crates/apis/src/domain_model.rs` (they are already the apis-side copies of the same enums) — reuse that file's `DomainCategory`, `SdtmVariableType`, `SdtmVariableCore` and `SdtmRole` bodies verbatim.

- [ ] **Step 4: Register the module**

In `lib/crates/apis/src/lib.rs` add `pub mod sdtm_project;` to the module list, keeping the existing alphabetical ordering.

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p apis`
Expected: PASS — the existing user/auth assertions plus the two new ones.

- [ ] **Step 6: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p apis --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/apis
git commit -m "$(cat <<'EOF'
feat(apis): add the sdtm-project service port

The outbound port for the per-project SDTM model tree: twenty CRUD methods
over the five aggregates, the request and view DTOs, and the copied value
objects so the apis crate stays dependency-free.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p apis
         cargo clippy -p apis --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: The project lookup adapter and the facade

**Files:**
- Create: `lib/crates/sdtm-project/src/adapter/service/project.rs`
- Create: `lib/crates/sdtm-project/src/adapter/service/project/project_lookup_impl.rs`
- Create: `lib/crates/sdtm-project/src/adapter/facade.rs`
- Create: `lib/crates/sdtm-project/src/adapter/facade/in_memory.rs`
- Create: `lib/crates/sdtm-project/src/adapter/facade/in_memory/service.rs`
- Create: `lib/crates/sdtm-project/src/adapter/facade/in_memory/tests.rs`
- Modify: `lib/crates/sdtm-project/src/adapter.rs`
- Modify: `lib/crates/sdtm-project/src/lib.rs`

**Interfaces:**
- Consumes: Task 6's `apis::sdtm_project` types; Task 2's usecase.
- Produces:
  - `ProjectLookupImpl { projects: Arc<dyn apis::project::ProjectService> }` with `pub fn new(projects: Arc<dyn apis::project::ProjectService>) -> Self`, implementing the crate's `domain::ProjectLookup`. Re-exported from the crate root.
  - `SdtmProjectServiceImpl<V, D, DO, Va, VO, P>` with `pub fn new(usecase: SdtmProjectUsecase<V, D, DO, Va, VO, P>) -> Self` and `pub fn from_repos(version_repo: V, domain_repo: D, domain_override_repo: DO, variable_repo: Va, variable_override_repo: VO, projects: Arc<P>) -> Self`, implementing `apis::sdtm_project::SdtmProjectService`. Re-exported from the crate root.
  - `impl From<UsecaseError> for apis::sdtm_project::SdtmProjectApiError`.

- [ ] **Step 1: Write the failing facade tests**

Create `src/adapter/facade/in_memory/tests.rs`. It wires the facade on top of **its own** in-memory fakes (independent of the usecase tests' fakes — mirror `domain-model`, which keeps the two layers' fakes separate).

The fakes here are simpler than Task 2's: the facade tests exercise the api-surface round trip, so the fakes can be a single `Arc<Mutex<…>>`-backed store per aggregate with an `AtomicI64` id counter, and they must hydrate the override the same way (the domain fake looks up the override collection).

Tests (names are load-bearing):

`version_crud_round_trips_through_api` — create → list → update → delete through the `SdtmProjectService` trait methods.

`duplicate_version_maps_to_api_error` — creating the same `(project_code, version_name)` twice yields `SdtmProjectApiError::DuplicateSdtmProjectVersion { .. }`.

`unknown_version_maps_to_api_error` — `get_version_by_id(999)` yields `SdtmProjectApiError::SdtmProjectVersionNotFound(999)`.

`missing_project_maps_to_api_error` — creating a version for a code the fake lookup does not know yields `SdtmProjectApiError::ProjectNotFound(code)`.

`domain_crud_round_trips_through_api` — create → get → list → update → delete.

`domain_category_round_trips` — a domain override's `DomainCategory::TrialDesign` survives the api round trip unchanged.

`domain_override_round_trips_through_api` — create → get → update → delete of a domain override.

`variable_crud_round_trips_through_api` — create → get → list → update → delete.

`variable_role_enum_round_trips` — a variable override's `SdtmRole::RecordQualifier` survives unchanged.

`variable_role_three_state_clears_field` — updating with `variable_role: Some(None)` clears the role.

`key_variables_order_survives_the_api` — create a domain with `key_variables: vec!["AESEV".into(), "AETERM".into()]`; the returned view's `key_variables` is exactly `["AESEV", "AETERM"]` — same order, not sorted.

`validation_error_maps_to_validation_api_error` — creating a version with a blank `version_name` yields `SdtmProjectApiError::Validation(_)`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sdtm-project`
Expected: compile failure — the facade does not exist yet.

- [ ] **Step 3: Implement the project lookup adapter**

`src/adapter/service.rs`:

```rust
//! Outbound-port adapters. Adapters from the domain ports to the
//! `apis` crate live here. Today this only houses the `ProjectLookup`
//! adapter that bridges `apis::project::ProjectService` to the domain
//! `ProjectLookup`.

pub mod project;
```

`src/adapter/service/project.rs`:

```rust
mod project_lookup_impl;

pub use project_lookup_impl::ProjectLookupImpl;
```

`src/adapter/service/project/project_lookup_impl.rs`:

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::project::{ProjectApiError, ProjectService};

use crate::domain::{DomainError, ProjectLookup};

pub struct ProjectLookupImpl {
    projects: Arc<dyn ProjectService>,
}

impl ProjectLookupImpl {
    pub fn new(projects: Arc<dyn ProjectService>) -> Self {
        Self { projects }
    }
}

#[async_trait]
impl ProjectLookup for ProjectLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        match self.projects.get_project_by_code(code).await {
            Ok(_) => Ok(()),
            Err(ProjectApiError::NotFound) => Err(DomainError::ProjectNotFound(code.to_string())),
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }
}
```

- [ ] **Step 4: Implement the facade**

`src/adapter/facade.rs`:

```rust
//! Outbound-port adapters.
//!
//! Adapters in this sub-module implement API-facing traits defined in
//! other workspace crates (today: `apis::sdtm_project::SdtmProjectService`).

mod in_memory;

pub use in_memory::SdtmProjectServiceImpl;
```

`src/adapter/facade/in_memory.rs`:

```rust
mod service;

#[cfg(test)]
mod tests;

pub use service::SdtmProjectServiceImpl;
```

`src/adapter/facade/in_memory/service.rs`:

- `SdtmProjectServiceImpl<V, D, DO, Va, VO, P> { usecase: SdtmProjectUsecase<V, D, DO, Va, VO, P> }` with `new` and `from_repos` as listed in Interfaces. `from_repos` takes `projects: Arc<P>` and builds the config.
- Enum mappers between the `apis` copies and the crate's copies — `to_internal_category` / `from_internal_category`, `to_internal_variable_type` / `from_internal_variable_type`, `to_internal_variable_core` / `from_internal_variable_core`, `to_internal_role` / `from_internal_role`, plus `domain_descriptions_from_api` / `domain_descriptions_to_api` and `variable_descriptions_from_api` / `variable_descriptions_to_api`. Copy the bodies from `lib/crates/domain-model/src/adapter/facade/in_memory/service.rs`.
- View mappers: `version_view_from_internal`, `domain_view_from_internal`, `domain_override_view_from_internal`, `variable_view_from_internal`, `variable_override_view_from_internal`.
- `impl From<UsecaseError> for SdtmProjectApiError`:

  ```rust
  impl From<UsecaseError> for SdtmProjectApiError {
      fn from(err: UsecaseError) -> Self {
          match err {
              UsecaseError::Validation(domain) => SdtmProjectApiError::Validation(domain.to_string()),
              UsecaseError::Repository(domain) => match domain {
                  DomainError::NotFound => SdtmProjectApiError::NotFound,
                  DomainError::ProjectNotFound(code) => SdtmProjectApiError::ProjectNotFound(code),
                  DomainError::SdtmProjectVersionNotFound(id) => SdtmProjectApiError::SdtmProjectVersionNotFound(id),
                  DomainError::SdtmProjectDomainNotFound(id) => SdtmProjectApiError::SdtmProjectDomainNotFound(id),
                  DomainError::SdtmProjectDomainOverrideNotFound(id) => SdtmProjectApiError::SdtmProjectDomainOverrideNotFound(id),
                  DomainError::SdtmProjectVariableNotFound(id) => SdtmProjectApiError::SdtmProjectVariableNotFound(id),
                  DomainError::SdtmProjectVariableOverrideNotFound(id) => SdtmProjectApiError::SdtmProjectVariableOverrideNotFound(id),
                  DomainError::DuplicateSdtmProjectVersion { project_code, version_name } => SdtmProjectApiError::DuplicateSdtmProjectVersion { project_code, version_name },
                  DomainError::DuplicateSdtmProjectDomain { version_id, domain_code } => SdtmProjectApiError::DuplicateSdtmProjectDomain { version_id, domain_code },
                  DomainError::DuplicateSdtmProjectDomainOverride { domain_id } => SdtmProjectApiError::DuplicateSdtmProjectDomainOverride { domain_id },
                  DomainError::DuplicateSdtmProjectVariable { domain_id, variable_code } => SdtmProjectApiError::DuplicateSdtmProjectVariable { domain_id, variable_code },
                  DomainError::DuplicateSdtmProjectVariableOverride { variable_id } => SdtmProjectApiError::DuplicateSdtmProjectVariableOverride { variable_id },
                  DomainError::FkSdtmProjectVersionNotFound(id) => SdtmProjectApiError::FkSdtmProjectVersionNotFound(id),
                  DomainError::FkSdtmProjectDomainNotFound(id) => SdtmProjectApiError::FkSdtmProjectDomainNotFound(id),
                  DomainError::FkSdtmProjectVariableNotFound(id) => SdtmProjectApiError::FkSdtmProjectVariableNotFound(id),
                  // The validation-only variants never reach here through
                  // `UsecaseError::Repository` — but a stored row with a
                  // bad enum value could produce one. Map to `Repository`
                  // rather than panicking: a 500 beats a crashed handler.
                  DomainError::EmptyProjectCode
                  | DomainError::EmptyVersionName
                  | DomainError::EmptyDomainCode
                  | DomainError::EmptyVariableCode
                  | DomainError::EmptyKeyVariable
                  | DomainError::InvalidDomainCategory(_)
                  | DomainError::InvalidVariableType(_)
                  | DomainError::InvalidVariableCore(_)
                  | DomainError::InvalidVariableRole(_) => SdtmProjectApiError::Repository(domain.to_string()),
              },
          }
      }
  }
  ```

- `#[async_trait] impl SdtmProjectService for SdtmProjectServiceImpl<V, D, DO, Va, VO, P>` — twenty methods, each translating the `apis` request into a usecase command, calling the usecase, and mapping the internal view back to the api view. `list_versions_by_project` returns `SdtmProjectVersionList { versions: … }`; `list_domains_by_version` returns `SdtmProjectDomainList { domains: … }`; `list_variables_by_domain` returns `SdtmProjectVariableList { variables: … }`.

- [ ] **Step 5: Register the modules and re-exports**

In `src/adapter.rs` the `facade` and `service` modules are already declared (Task 3 created the placeholders) — replace the placeholder bodies with the real module files.

In `src/lib.rs` add:

```rust
pub use adapter::facade::SdtmProjectServiceImpl;
pub use adapter::service::project::ProjectLookupImpl;
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p sdtm-project`
Expected: PASS — 21 domain + 14 usecase + 5 adapter unit + 11 facade tests.

- [ ] **Step 7: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
feat(sdtm-project): add the project lookup adapter and the facade

ProjectLookupImpl bridges apis::project::ProjectService to the domain
ProjectLookup port. SdtmProjectServiceImpl adapts the usecase onto the
new apis::sdtm_project::SdtmProjectService port, with the enum and
description mappers and a non-panicking UsecaseError mapping.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p sdtm-project
         cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Crate-level tests and the README

**Files:**
- Create: `lib/crates/sdtm-project/tests/public_api.rs`
- Create: `lib/crates/sdtm-project/tests/integration_persistence.rs`
- Create: `lib/crates/sdtm-project/README.md`

**Interfaces:**
- Consumes: everything Tasks 1–7 produced.
- Produces: the crate's compile-only public-API contract and its live-database round trips.

- [ ] **Step 1: Write `tests/public_api.rs`**

Compile-only. Mirror `lib/crates/domain-model/tests/public_api.rs`:

- `public_types_are_nameable_from_crate_root` — a `use sdtm_project::{…}` importing every crate-root re-export the spec lists, then one local `fn assert_*(_: Type)` per type to prove each is nameable.
- `repo_constructors_take_a_pool` — five function pointers:

  ```rust
  let _: fn(PgPool) -> SdtmProjectVersionRepoPg = SdtmProjectVersionRepoPg::new;
  let _: fn(PgPool) -> SdtmProjectDomainRepoPg = SdtmProjectDomainRepoPg::new;
  let _: fn(PgPool) -> SdtmProjectDomainOverrideRepoPg = SdtmProjectDomainOverrideRepoPg::new;
  let _: fn(PgPool) -> SdtmProjectVariableRepoPg = SdtmProjectVariableRepoPg::new;
  let _: fn(PgPool) -> SdtmProjectVariableOverrideRepoPg = SdtmProjectVariableOverrideRepoPg::new;
  ```

- `usecase_constructor_takes_the_config` — a function pointer pinning `SdtmProjectUsecase::new`:

  ```rust
  fn assert_new<V, D, DO, Va, VO, P>(f: fn(SdtmProjectUsecaseConfig<V, D, DO, Va, VO, P>) -> SdtmProjectUsecase<V, D, DO, Va, VO, P>) {}
  ```

  called with `SdtmProjectUsecase::new`.
- `facade_constructor_takes_the_usecase` — pin `SdtmProjectServiceImpl::new` the same way.
- `service_is_object_safe` — `let _: Option<Box<dyn apis::sdtm_project::SdtmProjectService>> = None;` plus a `Send + Sync` assertion.

- [ ] **Step 2: Run the test to verify it compiles**

Run: `cargo test -p sdtm-project --test public_api`
Expected: PASS (it is a compile-time contract; there are no runtime assertions beyond the constructor pins).

- [ ] **Step 3: Write `tests/integration_persistence.rs`**

Mirror `lib/crates/domain-model/tests/integration_persistence.rs`:

- A `with_pool` helper that: loads `.env` via `dotenvy::dotenv()`; reads `AEGIS_SDTM_PROJECT_DATABASE_URL` and **panics with a clear message** when it is unset; connects a `PgPool`; drops the live tables in reverse FK order (`sdtm_project_variable_overrides`, `sdtm_project_variables`, `sdtm_project_domain_overrides`, `sdtm_project_domains`, `sdtm_project_versions`, each `CASCADE`); drops `_sqlx_migrations`; then applies `sqlx::migrate!("./migrations").run(&pool).await`; then runs the closure.
- A `unique(prefix: &str) -> String` helper using an `AtomicU64` counter plus wall-clock nanoseconds, for every column with a `UNIQUE` constraint.
- Every test is `#[tokio::test]` + `#[ignore = "requires AEGIS_SDTM_PROJECT_DATABASE_URL"]`.

Tests (names are load-bearing — the Review Focus items 1–5 live here):

`version_create_then_list_round_trips` — create a version with a unique `(project_code, version_name)`; `list_by_project` returns it.

`version_update_changes_name` — update the `version_name` and assert the new value comes back.

`version_update_of_missing_row_is_not_found` — `update` on id `999_999` is `DomainError::SdtmProjectVersionNotFound(999_999)`.

`version_delete_of_missing_row_is_not_found` — `delete` on id `999_999` is `DomainError::SdtmProjectVersionNotFound(999_999)`.

`version_unique_is_enforced_per_project` — the same `version_name` under two different `project_code`s both succeed; under the same `project_code` the second is `DomainError::DuplicateSdtmProjectVersion { .. }`.

`domain_create_then_list_round_trips` — create a domain on a version; `list_by_version` returns it.

`domain_key_variables_preserve_caller_order` — create a domain with `key_variables: vec!["AESEV".into(), "AETERM".into()]`; the row read back has exactly that order.

`domain_update_changes_code_and_key_variables` — update both `domain_code` and `key_variables` and assert both changed and the order survived.

`domain_update_of_missing_row_is_not_found` — `update` on id `999_999` is `DomainError::SdtmProjectDomainNotFound(999_999)`.

`domain_delete_of_missing_row_is_not_found` — `delete` on id `999_999` is `DomainError::SdtmProjectDomainNotFound(999_999)`.

`domain_override_create_then_get_round_trips` — create an override; `find_by_domain_id` returns the same category and descriptions.

`domain_override_is_unique_per_domain` — a second override for the same domain is `DomainError::DuplicateSdtmProjectDomainOverride { .. }`.

`domain_override_update_changes_category` — update the category; the new value comes back.

`domain_override_delete_of_missing_row_is_not_found` — `delete` on a domain with no override is `DomainError::SdtmProjectDomainOverrideNotFound(domain_id)`.

`domain_read_carries_the_override` — after creating an override, `find_by_id` on the domain returns `Some(override)` with the right category — this is the hydration contract.

`domain_update_carries_the_override` — `update` on that domain returns `Some(override)` too.

`version_delete_cascades_to_the_whole_subtree` — create a version → domain → domain override → variable → variable override; delete the version; then `find_by_id` on the domain, the variable and both overrides all report not-found.

`domain_delete_cascades_to_override_and_variables` — delete a domain; its override and its variables are gone.

`variable_create_then_list_orders_by_sequence` — create two variables with sequences `20` and `10`; `list_by_domain` returns them in `10, 20` order.

`variable_update_changes_sequence` — update the sequence; the new value comes back.

`variable_update_of_missing_row_is_not_found` / `variable_delete_of_missing_row_is_not_found` — same not-found shape as the domain.

`variable_override_create_then_get_round_trips` — create an override with `variable_role: Some(SdtmRole::Topic)`; `find_by_variable_id` returns it.

`variable_override_role_can_be_cleared` — update with the three-state clear; the role reads back as `None`.

`variable_override_delete_of_missing_row_is_not_found`.

`variable_delete_cascades_to_override`.

`domain_create_when_version_vanished_maps_to_fk_not_found` — insert a version, delete it directly with raw SQL (bypassing the repository), then `create` a domain on the dead version id; the error is `DomainError::FkSdtmProjectVersionNotFound(version_id)`. This is the Review Focus item 1 — the FK race must not surface as a 500.

- [ ] **Step 4: Run the live tests**

Run: `cargo test -p sdtm-project -- --ignored --test-threads=1`
Expected: the tests **panic with the clear "AEGIS_SDTM_PROJECT_DATABASE_URL must be set" message** if the variable is absent — that is the documented behaviour, not a failure. If a dedicated test database is available, set the variable and re-run; expect all of the above to PASS.

State in the commit message whether the live run actually executed or was skipped for lack of a database.

- [ ] **Step 5: Write the README**

`lib/crates/sdtm-project/README.md` must contain:

- One sentence: what the crate is ("Owns the per-project SDTM model tree — the domains and variables a project declares on top of the CDISC SDTM standard — behind five PostgreSQL tables and an `apis::sdtm_project::SdtmProjectService` port.").
- The `src/` tree (copy the tree from this plan's File Structure section).
- Database setup:

  ```bash
  sqlx migrate run --source lib/crates/sdtm-project/migrations
  ```

  plus a note that the server does not auto-migrate, so this must be run against the server's `AEGIS_DATABASE_URL`.
- The live-test section: `AEGIS_SDTM_PROJECT_DATABASE_URL` and `cargo test -p sdtm-project -- --ignored --test-threads=1`, with a prominent warning:

  > `AEGIS_SDTM_PROJECT_DATABASE_URL` **must point at a dedicated database.** The live test drops the `_sqlx_migrations` bookkeeping table, and SQLx keeps exactly one such table *per database*, shared by every crate that migrates it. Pointing this at the server's database would erase the applied-migration record for auth, user, project, terminology, mission and crf alike.
- A constructor snippet:

  ```rust
  use sdtm_project::{
      SdtmProjectServiceImpl, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
      SdtmProjectVersionRepoPg, SdtmProjectDomainRepoPg, SdtmProjectDomainOverrideRepoPg,
      SdtmProjectVariableRepoPg, SdtmProjectVariableOverrideRepoPg,
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

- A back-link to `docs/guidelines/lib-crate-development.md`.

- [ ] **Step 6: Run the full gate**

Run: `cargo fmt --all -- --check && cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings && cargo test -p sdtm-project && cargo doc -p sdtm-project --no-deps`
Expected: all clean.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
test(sdtm-project): add the crate-level tests and the README

public_api.rs pins the crate-root surface, the five repo constructors and
the usecase/facade constructor chains. integration_persistence.rs covers the
live round trips, including the cascade deletes, the override hydration and
the foreign-key race that must surface as a 404 rather than a 500.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p sdtm-project
         cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings
         cargo doc -p sdtm-project --no-deps

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Server wiring

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml`
- Modify: `apps/server/aegis-server/src/state.rs`
- Modify: `apps/server/aegis-server/src/run.rs`
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`

**Interfaces:**
- Consumes: Task 6's `apis::sdtm_project` module; Task 7's `SdtmProjectServiceImpl` and `ProjectLookupImpl`.
- Produces:
  - `AppState.sdtm_project: Arc<dyn apis::sdtm_project::SdtmProjectService>`.
  - `ApiError::SdtmProject(#[from] apis::sdtm_project::SdtmProjectApiError)` with `status()` and `code()` arms.
  - `aegis_server::run::build_sdtm_project_service(pool, project) -> Arc<dyn apis::sdtm_project::SdtmProjectService>`.

- [ ] **Step 1: Add the dependency**

In `apps/server/aegis-server/Cargo.toml` add `sdtm-project = { path = "../../../lib/crates/sdtm-project" }` with a one-line comment saying it provides the SDTM project tree service.

- [ ] **Step 2: Add the state field and the null double**

In `apps/server/aegis-server/src/state.rs`:

- Add to `AppState`:

  ```rust
  pub sdtm_project: Arc<dyn apis::sdtm_project::SdtmProjectService>,
  ```

- Add to `state::test_support` a `NullSdtmProjectService` implementing all twenty `SdtmProjectService` methods, each returning `todo!()` — the same shape as `NullDomainModelService` and `NullCrfService`. Every other module's test doubles construct an `AppState`, so this keeps them compiling.

- [ ] **Step 3: Add the error mapping**

In `apps/server/aegis-server/src/transport/http/error.rs`:

- Add the variant:

  ```rust
  #[error("{0}")]
  SdtmProject(#[from] apis::sdtm_project::SdtmProjectApiError),
  ```

- Add the `status()` arm:

  ```rust
  ApiError::SdtmProject(e) => match e {
      apis::sdtm_project::SdtmProjectApiError::Validation(_) => StatusCode::BAD_REQUEST,
      apis::sdtm_project::SdtmProjectApiError::NotFound
      | apis::sdtm_project::SdtmProjectApiError::ProjectNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::SdtmProjectVersionNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::SdtmProjectDomainNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::SdtmProjectDomainOverrideNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::SdtmProjectVariableNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::SdtmProjectVariableOverrideNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::FkSdtmProjectVersionNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::FkSdtmProjectDomainNotFound(_)
      | apis::sdtm_project::SdtmProjectApiError::FkSdtmProjectVariableNotFound(_) => StatusCode::NOT_FOUND,
      apis::sdtm_project::SdtmProjectApiError::DuplicateSdtmProjectVersion { .. }
      | apis::sdtm_project::SdtmProjectApiError::DuplicateSdtmProjectDomain { .. }
      | apis::sdtm_project::SdtmProjectApiError::DuplicateSdtmProjectDomainOverride { .. }
      | apis::sdtm_project::SdtmProjectApiError::DuplicateSdtmProjectVariable { .. }
      | apis::sdtm_project::SdtmProjectApiError::DuplicateSdtmProjectVariableOverride { .. } => StatusCode::CONFLICT,
      apis::sdtm_project::SdtmProjectApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
  },
  ```

- Add the `code()` arm with the stable codes from the spec: `validation_failed`, `not_found`, `project_not_found`, `sdtm_project_version_not_found`, `sdtm_project_domain_not_found`, `sdtm_project_domain_override_not_found`, `sdtm_project_variable_not_found`, `sdtm_project_variable_override_not_found`, `duplicate_sdtm_project_version`, `duplicate_sdtm_project_domain`, `duplicate_sdtm_project_domain_override`, `duplicate_sdtm_project_variable`, `duplicate_sdtm_project_variable_override`, `fk_sdtm_project_version_not_found`, `fk_sdtm_project_domain_not_found`, `fk_sdtm_project_variable_not_found`, `repository_error`.

- Add a `#[cfg(test)] mod tests` case pinning the mapping: `SdtmProjectApiError::FkSdtmProjectVersionNotFound(1)` → 404 with code `fk_sdtm_project_version_not_found`; `DuplicateSdtmProjectDomain { .. }` → 409 with code `duplicate_sdtm_project_domain`.

- [ ] **Step 4: Wire the service in `run.rs`**

In `apps/server/aegis-server/src/run.rs`:

- Import `apis::sdtm_project::SdtmProjectService` and the crate's `SdtmProjectServiceImpl`, `SdtmProjectUsecase`, `SdtmProjectUsecaseConfig`, the five repos, and `ProjectLookupImpl`.
- Add `let sdtm_project = build_sdtm_project_service(pool.clone(), project.clone());` next to the other `build_*_service` calls — **before** `build_crf_service(pool, project.clone())` so `pool` is still available, or move the `crf` call after it.
- Add the field to the `AppState` literal: `sdtm_project,`.
- Add the builder function:

  ```rust
  fn build_sdtm_project_service(
      pool: PgPool,
      project: Arc<dyn apis::project::ProjectService>,
  ) -> Arc<dyn apis::sdtm_project::SdtmProjectService> {
      let version_repo = SdtmProjectVersionRepoPg::new(pool.clone());
      let domain_repo = SdtmProjectDomainRepoPg::new(pool.clone());
      let domain_override_repo = SdtmProjectDomainOverrideRepoPg::new(pool.clone());
      let variable_repo = SdtmProjectVariableRepoPg::new(pool.clone());
      let variable_override_repo = SdtmProjectVariableOverrideRepoPg::new(pool);
      let projects = Arc::new(ProjectLookupImpl::new(project));
      let usecase = SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
          version_repo,
          domain_repo,
          domain_override_repo,
          variable_repo,
          variable_override_repo,
          projects,
      });
      Arc::new(SdtmProjectServiceImpl::new(usecase))
  }
  ```

- [ ] **Step 5: Verify the server compiles and its tests pass**

Run: `cargo test -p aegis-server`
Expected: PASS. The other modules' test doubles compile against the widened `AppState`.

- [ ] **Step 6: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p aegis-server --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add apps/server/aegis-server
git commit -m "$(cat <<'EOF'
feat(server): wire the sdtm-project service into the app state

Adds the AppState field, the null test double, the ApiError variant and its
status/code mapping, and the builder that assembles the five Postgres
repositories over the shared pool with the project lookup bridged off the
canonical ProjectService.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p aegis-server
         cargo clippy -p aegis-server --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: The server HTTP surface

**Files:**
- Create: `apps/server/aegis-server/src/transport/http/sdtm_project.rs`
- Create: `apps/server/aegis-server/src/transport/http/sdtm_project/router.rs`
- Create: `apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`

**Interfaces:**
- Consumes: Task 6's `apis::sdtm_project` DTOs; Task 9's `AppState.sdtm_project`.
- Produces: 22 routes under `/api/sdtm-project`, the wire DTOs, and the OpenAPI schema entries.

- [ ] **Step 1: Write the wire DTOs**

Append a `-- sdtm-project --` section to `apps/server/aegis-server/src/transport/http/dto.rs`:

- **New enums** (do **not** reuse the `domain-model` section's copies — the two feature areas are allowed to diverge): `SdtmProjectDomainCategory`, `SdtmProjectVariableType`, `SdtmProjectVariableCore`, `SdtmProjectRole`. Same variants and serde renames as the domain's copies. `SdtmProjectDomainCategory` renames: `"Special Purpose"`, `"Interventions"`, `"Events"`, `"Findings"`, `"Trial Design"`, `"Relationships"`, `"Study Reference"`. `SdtmProjectRole` renames: `"Identifier"`, `"Topic"`, `"Timing"`, `"Record Qualifier"`, `"Synonym Qualifier"`, `"Variable Qualifier"`, `"Grouping Qualifier"`, `"Rule"`. The other two use `#[serde(rename_all = "PascalCase")]`.
- **Description DTOs**: `SdtmProjectDomainDescription { lang: String, details: SdtmProjectDomainDescriptionDetail }`, `SdtmProjectDomainDescriptionDetail { description: String, structure: String }`, `SdtmProjectVariableDescription { lang: String, details: SdtmProjectVariableDescriptionDetail }`, `SdtmProjectVariableDescriptionDetail { label: String }`.
- **Request DTOs** (one per CRUD pair), all `#[serde(rename_all = "camelCase")]` + `ToSchema`:

  `CreateSdtmProjectVersionRequest { project_code: String, version_name: String }`
  `UpdateSdtmProjectVersionRequest { version_name: Option<String> }`
  `CreateSdtmProjectDomainRequest { version_id: i64, domain_code: String, key_variables: Vec<String> }`
  `UpdateSdtmProjectDomainRequest { domain_code: Option<String>, key_variables: Option<Vec<String>> }`
  `CreateSdtmProjectDomainOverrideRequest { category: SdtmProjectDomainCategory, descriptions: Vec<SdtmProjectDomainDescription> }`
  `UpdateSdtmProjectDomainOverrideRequest { category: Option<SdtmProjectDomainCategory>, descriptions: Option<Vec<SdtmProjectDomainDescription>> }`
  `CreateSdtmProjectVariableRequest { domain_id: i64, variable_code: String, variable_sequence: i64 }`
  `UpdateSdtmProjectVariableRequest { variable_code: Option<String>, variable_sequence: Option<i64> }`
  `CreateSdtmProjectVariableOverrideRequest { variable_type: SdtmProjectVariableType, variable_core: SdtmProjectVariableCore, variable_role: Option<SdtmProjectRole>, descriptions: Vec<SdtmProjectVariableDescription> }`
  `UpdateSdtmProjectVariableOverrideRequest { variable_type: Option<SdtmProjectVariableType>, variable_core: Option<SdtmProjectVariableCore>, variable_role: Option<Option<SdtmProjectRole>>, descriptions: Option<Vec<SdtmProjectVariableDescription>> }`
  `SdtmProjectVersionListQuery { project_code: String }`

  The `Update*` request DTOs carry **no** `id` — the id comes from the path parameter, so the handler supplies it when it builds the `apis` request.

- **Response DTOs**: `SdtmProjectVersionViewResponse { id: i64, project_code: String, version_name: String, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectDomainViewResponse { id: i64, version_id: i64, domain_code: String, key_variables: Vec<String>, #[serde(rename = "override")] override_: Option<SdtmProjectDomainOverrideViewResponse>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectDomainOverrideViewResponse { id: i64, domain_id: i64, category: SdtmProjectDomainCategory, descriptions: Vec<SdtmProjectDomainDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectVariableViewResponse { id: i64, domain_id: i64, variable_code: String, variable_sequence: i64, #[serde(rename = "override")] override_: Option<SdtmProjectVariableOverrideViewResponse>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`, `SdtmProjectVariableOverrideViewResponse { id: i64, variable_id: i64, variable_type: SdtmProjectVariableType, variable_core: SdtmProjectVariableCore, variable_role: Option<SdtmProjectRole>, descriptions: Vec<SdtmProjectVariableDescription>, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`.

  The `override_` field name with `#[serde(rename = "override")]` is deliberate: `override` is a Rust keyword, so the wire DTO cannot name it directly.

- **List responses**: `SdtmProjectVersionListResponse { versions: Vec<SdtmProjectVersionViewResponse> }`, `SdtmProjectDomainListResponse { domains: Vec<SdtmProjectDomainViewResponse> }`, `SdtmProjectVariableListResponse { variables: Vec<SdtmProjectVariableViewResponse> }`.

- **Conversion impls**: `impl From<apis::sdtm_project::SdtmProjectVersionView> for SdtmProjectVersionViewResponse` and one per view type, plus the enum mappers `SdtmProjectDomainCategory` ↔ `apis::sdtm_project::DomainCategory`, `SdtmProjectVariableType` ↔ `…::SdtmVariableType`, `SdtmProjectVariableCore` ↔ `…::SdtmVariableCore`, `SdtmProjectRole` ↔ `…::SdtmRole`, and the description mappers both ways.

- Add a `#[cfg(test)] mod tests` case in `dto.rs` asserting the wire JSON of a domain view: serialise a `SdtmProjectDomainViewResponse` with `override_: Some(..)` and assert the JSON contains `"override"` (not `"override_"`) and that `keyVariables` is the array in the given order.

- [ ] **Step 2: Write the failing handler tests**

Create `apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs` with a `#[cfg(test)] mod tests` containing a fake `SdtmProjectService` (every method returns `todo!()`) and two tests, following the fake-service pattern in `apps/server/aegis-server/src/transport/http/project/handlers.rs`:

`list_versions_without_project_code_is_rejected` — a request to `GET /api/sdtm-project/versions` with no query string is rejected by axum's `Query` extractor with **400**.

`create_domain_returns_created` — a request to `POST /api/sdtm-project/domains` with a valid body returns **201** and the created domain's `domainCode`.

- [ ] **Step 3: Implement the handlers**

`apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs` — twenty handlers, each a thin adapter:

| Handler | Method + path | Success |
| --- | --- | --- |
| `create_version` | `POST /versions` | 201 |
| `list_versions_by_project` | `GET /versions?projectCode=…` | 200 |
| `get_version_by_id` | `GET /versions/{id}` | 200 |
| `update_version` | `PUT /versions/{id}` | 200 |
| `delete_version` | `DELETE /versions/{id}` | 204 |
| `create_domain` | `POST /domains` | 201 |
| `get_domain_by_id` | `GET /domains/{id}` | 200 |
| `list_domains_by_version` | `GET /versions/{version_id}/domains` | 200 |
| `update_domain` | `PUT /domains/{id}` | 200 |
| `delete_domain` | `DELETE /domains/{id}` | 204 |
| `create_domain_override` | `POST /domains/{domain_id}/override` | 201 |
| `get_domain_override` | `GET /domains/{domain_id}/override` | 200 (`{"override": null}` when there is no override) |
| `update_domain_override` | `PUT /domains/{domain_id}/override` | 200 |
| `delete_domain_override` | `DELETE /domains/{domain_id}/override` | 204 |
| `create_variable` | `POST /variables` | 201 |
| `get_variable_by_id` | `GET /variables/{id}` | 200 |
| `list_variables_by_domain` | `GET /domains/{domain_id}/variables` | 200 |
| `update_variable` | `PUT /variables/{id}` | 200 |
| `delete_variable` | `DELETE /variables/{id}` | 204 |
| `create_variable_override` | `POST /variables/{variable_id}/override` | 201 |
| `get_variable_override` | `GET /variables/{variable_id}/override` | 200 (`{"override": null}` when there is no override) |
| `update_variable_override` | `PUT /variables/{variable_id}/override` | 200 |
| `delete_variable_override` | `DELETE /variables/{variable_id}/override` | 204 |

Handler shape (every handler follows it):

```rust
pub async fn create_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<dto::CreateSdtmProjectVersionRequest>,
) -> Result<(StatusCode, Json<dto::SdtmProjectVersionViewResponse>), ApiError> {
    let view = state
        .sdtm_project
        .create_version(apis::sdtm_project::CreateSdtmProjectVersionRequest {
            project_code: req.project_code,
            version_name: req.version_name,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}
```

Notes:
- **No role gate.** Every handler takes `_claims: AuthClaims` and none calls `require_admin_or_root`.
- The `#[utoipa::path]` block on each handler lists the success status, the error statuses it can actually return, and `security(("BearerAuth" = []))`. **Do not list a `403`** — no role gate is enforced.
- `list_versions_by_project` takes `Query(dto::SdtmProjectVersionListQuery)`; `project_code` is required.
- `update_*` handlers take the id from `Path(dto::PathId { id })` and merge it into the `apis` request.
- The override handlers take the parent id from the path (`Path(dto::PathId { id })`) and use it as `domain_id` / `variable_id`.

- [ ] **Step 4: Implement the router**

`apps/server/aegis-server/src/transport/http/sdtm_project/router.rs`:

```rust
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::sdtm_project::handlers;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // ---- SdtmProjectVersion ----
        .routes(routes!(handlers::list_versions_by_project))
        .routes(routes!(handlers::create_version))
        .routes(routes!(handlers::get_version_by_id))
        .routes(routes!(handlers::update_version))
        .routes(routes!(handlers::delete_version))
        // ---- SdtmProjectDomain ----
        .routes(routes!(handlers::create_domain))
        .routes(routes!(handlers::get_domain_by_id))
        .routes(routes!(handlers::list_domains_by_version))
        .routes(routes!(handlers::update_domain))
        .routes(routes!(handlers::delete_domain))
        // ---- SdtmProjectDomainOverride ----
        .routes(routes!(handlers::create_domain_override))
        .routes(routes!(handlers::get_domain_override))
        .routes(routes!(handlers::update_domain_override))
        .routes(routes!(handlers::delete_domain_override))
        // ---- SdtmProjectVariable ----
        .routes(routes!(handlers::create_variable))
        .routes(routes!(handlers::get_variable_by_id))
        .routes(routes!(handlers::list_variables_by_domain))
        .routes(routes!(handlers::update_variable))
        .routes(routes!(handlers::delete_variable))
        // ---- SdtmProjectVariableOverride ----
        .routes(routes!(handlers::create_variable_override))
        .routes(routes!(handlers::get_variable_override))
        .routes(routes!(handlers::update_variable_override))
        .routes(routes!(handlers::delete_variable_override))
}
```

One `routes!` call per handler — `utoipa-axum` 0.2 panics when a single `routes!` receives two handlers sharing an HTTP method.

`apps/server/aegis-server/src/transport/http/sdtm_project.rs`:

```rust
//! HTTP handlers for the sdtm-project namespace.

pub mod handlers;
pub mod router;
```

- [ ] **Step 5: Mount the router and register the OpenAPI schemas**

In `apps/server/aegis-server/src/transport/http/router.rs`:

- Add `pub mod sdtm_project;` to the module list.
- Nest the router: `.nest("/api/sdtm-project", sdtm_project::router::router())` alongside the other namespaces.
- Add to the `#[cfg(test)] mod tests` block an OpenAPI assertion:

  ```rust
  // /api/sdtm-project namespace must advertise every route with the
  // BearerAuth requirement. No route advertises a 403: the namespace
  // has no role gate, so a 403 can never be produced.
  for (method, path) in [
      ("post", "/api/sdtm-project/versions"),
      ("get", "/api/sdtm-project/versions"),
      ("get", "/api/sdtm-project/versions/{id}"),
      ("put", "/api/sdtm-project/versions/{id}"),
      ("delete", "/api/sdtm-project/versions/{id}"),
      ("post", "/api/sdtm-project/domains"),
      ("get", "/api/sdtm-project/domains/{id}"),
      ("get", "/api/sdtm-project/versions/{version_id}/domains"),
      ("put", "/api/sdtm-project/domains/{id}"),
      ("delete", "/api/sdtm-project/domains/{id}"),
      ("post", "/api/sdtm-project/domains/{domain_id}/override"),
      ("get", "/api/sdtm-project/domains/{domain_id}/override"),
      ("put", "/api/sdtm-project/domains/{domain_id}/override"),
      ("delete", "/api/sdtm-project/domains/{domain_id}/override"),
      ("post", "/api/sdtm-project/variables"),
      ("get", "/api/sdtm-project/variables/{id}"),
      ("get", "/api/sdtm-project/domains/{domain_id}/variables"),
      ("put", "/api/sdtm-project/variables/{id}"),
      ("delete", "/api/sdtm-project/variables/{id}"),
      ("post", "/api/sdtm-project/variables/{variable_id}/override"),
      ("get", "/api/sdtm-project/variables/{variable_id}/override"),
      ("put", "/api/sdtm-project/variables/{variable_id}/override"),
      ("delete", "/api/sdtm-project/variables/{variable_id}/override"),
  ] {
      let op = &doc["paths"][path][method];
      assert!(op.is_object(), "missing {method} {path} in openapi");
      assert_eq!(
          op["security"][0]["BearerAuth"],
          serde_json::json!([]),
          "{method} {path} must require BearerAuth",
      );
      let response_keys: Vec<&str> = op["responses"]
          .as_object()
          .expect("responses object")
          .keys()
          .map(|s| s.as_str())
          .collect();
      assert!(
          !response_keys.contains(&"403"),
          "{method} {path} must not advertise a 403 (got {response_keys:?})",
      );
  }
  ```

- In `apps/server/aegis-server/src/transport/http/openapi.rs`, register the new response and request schemas in the `components(schemas(...))` list so the OpenAPI document carries them.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p aegis-server`
Expected: PASS — the two handler tests, the dto test, the error-mapping test, and the OpenAPI assertions.

- [ ] **Step 7: Lint and format**

Run: `cargo fmt --all -- --check && cargo clippy -p aegis-server --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 8: Run the full workspace gate**

Run:

```bash
cargo check --workspace
cargo test --workspace
cargo doc -p sdtm-project --no-deps
```

Expected: all clean. If an unrelated workspace member fails because of a missing system library, document that rather than working around it.

- [ ] **Step 9: Commit**

```bash
git add apps/server/aegis-server
git commit -m "$(cat <<'EOF'
feat(server): add the sdtm-project HTTP surface

Twenty-two routes under /api/sdtm-project covering CRUD for the version,
domain, domain-override, variable and variable-override models, plus the
wire DTOs and the OpenAPI schema entries. Any authenticated user may
read and write: the namespace has no role gate, so no route advertises a
403.

Spec: docs/superpowers/specs/2026-10-23-sdtm-project-crate-design.md
Verified: cargo test -p aegis-server
         cargo test --workspace
         cargo clippy -p aegis-server --all-targets --all-features -- -D warnings

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review

**Spec coverage.** Every section of the spec maps to a task: data model → Task 1; schema → Task 3; list strategy → Tasks 3–5; override hydration → Tasks 2, 4, 5; cross-crate project validation → Task 7; validation rules → Tasks 1–2; public API surface → Task 1 + Task 8; `DomainError` variants → Task 1; usecase → Task 2; `apis` port → Task 6; persistence → Tasks 3–5; facade → Task 7; server HTTP surface → Tasks 9–10; tests → Tasks 1–2, 6–8, 10; dependencies → Task 1; workspace wiring → Task 1 + Task 6 + Task 9; README → Task 8; verification gate → every task's final step.

**Type consistency.** `key_variables` is `Vec<String>` in the domain aggregate, the domain view, the `apis` view and the wire DTO — never `HashSet`. `domain_override` / `variable_override` in the domain aggregates and views, the same names plus `#[serde(rename = "override")]` in the wire DTOs. `projects: Arc<P>` in the usecase config, matching `crf`. The override commands are keyed on `domain_id` / `variable_id` in the domain, the usecase and the `apis` requests alike. `SdtmProjectApiError::Fk*` maps to 404 in both the error table and the server's `status()` arm.

**Step scan.** No step says "TBD", "handle edge cases", or "write appropriate tests". Every test step names the test and its assertions; every code step gives the exact signature and the values the spec pins.

**Review Focus.** All five items have a named test in the owning task: item 1 → Task 5's `domain_create_when_version_vanished_maps_to_fk_not_found`; item 2 → Tasks 3, 4, 5's `*_delete_of_missing_row_is_not_found`; item 3 → Tasks 3, 4, 5's `*_update_of_missing_row_is_not_found`; item 4 → Task 1's `normalize_key_variables_*`, Task 4's `domain_key_variables_preserve_caller_order`, Task 7's `key_variables_order_survives_the_api`; item 5 → Task 4's `version_delete_cascades_to_the_whole_subtree` and `domain_delete_cascades_to_override_and_variables`.

**Proportion.** The plan states signatures, SQL shapes, test names and assertions; it does not contain the implementation bodies. The one place it carries SQL verbatim is the migration files, which the schema section of the spec fixes exactly and where a paraphrase would risk a wrong constraint.