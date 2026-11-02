# SDTM Project Crate Design

## Goal

Add `lib/crates/sdtm-project` as a reusable Rust library that owns the
**per-project SDTM model tree**: the set of domains and variables a project
has customised on top of the CDISC SDTM standard.

The crate owns:

- The schema for five `sdtm_project_*` tables (five migrations).
- Validation rules for each aggregate (non-empty codes, enum CHECK constraints).
- A PostgreSQL-backed `*RepoPg` per aggregate (five repositories).
- A narrow `domain::ProjectLookup` port (`get_by_code`) that the usecase uses
  to validate `SdtmProjectVersion.project_code` against the `project` crate.
  The concrete adapter delegates to `apis::project::ProjectService`.
- A `SdtmProjectUsecase<V, D, DO, Va, VO, P>` generic over the five repository
  ports plus the project lookup, orchestrating CRUD for all five aggregates
  and projecting them into view DTOs.
- A `SdtmProjectServiceImpl` facade that adapts `SdtmProjectUsecase` to the
  new `apis::sdtm_project::SdtmProjectService` outbound port.

The crate does **not** own the CDISC SDTM standard itself. That lives in
`lib/crates/domain-model` (`SdtmVersion` / `SdtmDomain` / `SdtmVariable`).
This crate owns what a **project** has declared *on top of* it.

The crate does **not** own project lifecycle. Projects live in the `project`
crate behind `apis::project::ProjectService`; this crate depends on the `apis`
crate, not on `project` directly.

### Relationship to `domain-model`

`domain-model` describes the CDISC SDTM **standard** (one row per published
IG version, one row per standard domain, one row per standard variable).
`sdtm-project` describes a **project's own model tree** layered on top: the
project picks a version name, declares which domains it uses, and for each
domain and variable records *overrides* — the project's own category,
descriptions, variable type / core / role.

The two crates deliberately **share no types**. Per the task requirement,
`DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole`,
`SdtmDomainDescription` and `SdtmVariableDescription` are **copied** into
`sdtm-project`'s own `domain/` layer rather than imported from
`domain-model`. The copy is deliberate: the standard and the project overlay
are allowed to drift independently, and a shared type would silently couple
them.

## Architecture

Ports-and-adapters DDD structure, mirroring
[`lib/crates/domain-model/`](../../lib/crates/domain-model/) and
[`lib/crates/crf/`](../../lib/crates/crf/):

- `domain` — value objects (the copied enums and description DTOs),
  aggregates (one per table + `*New` / `*Update` DTOs), ports (five
  `*Repository` traits + `ProjectLookup`), and `DomainError`. No I/O, no
  `sqlx`, no `tokio`.
- `usecase` — `SdtmProjectUsecase` orchestrates CRUD across the six ports
  and projects domain aggregates into view DTOs. Command DTOs live in
  `usecase/commands.rs`; views in `usecase/views.rs`; `UsecaseError` in
  `usecase/error.rs`.
- `adapter` — concrete implementations:
  - `adapter::persistence::postgres` — `*RepoPg` per port (SQLx runtime API).
  - `adapter::service::project::ProjectLookupImpl` — bridges
    `apis::project::ProjectService` to the domain `ProjectLookup`.
  - `adapter::facade::in_memory::SdtmProjectServiceImpl` — adapts
    `SdtmProjectUsecase` to `apis::sdtm_project::SdtmProjectService`.

### Module tree

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
│   ├── lib.rs
│   ├── domain.rs
│   ├── domain/
│   │   ├── domain_category.rs          # copied from domain-model
│   │   ├── variable_type.rs            # copied from domain-model
│   │   ├── sdtm_domain_description.rs  # copied from domain-model
│   │   ├── sdtm_variable_description.rs # copied from domain-model
│   │   ├── sdtm_project_version.rs
│   │   ├── sdtm_project_domain.rs
│   │   ├── sdtm_project_domain_override.rs
│   │   ├── sdtm_project_variable.rs
│   │   ├── sdtm_project_variable_override.rs
│   │   ├── project_lookup.rs
│   │   ├── repository.rs
│   │   ├── error.rs
│   │   └── tests.rs
│   ├── usecase.rs
│   ├── usecase/
│   │   ├── commands.rs
│   │   ├── error.rs
│   │   ├── views.rs
│   │   ├── sdtm_project_usecase.rs
│   │   └── tests.rs
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
└── tests/
    ├── public_api.rs
    └── integration_persistence.rs
```

## Data Model

```rust
// domain/sdtm_project_version.rs
pub struct SdtmProjectVersion {
    pub id: i64,
    pub project_code: String,     // maps to Project.code in the `project` crate
    pub version_name: String,     // unique per (project_code, version_name)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/sdtm_project_domain.rs
pub struct SdtmProjectDomain {
    pub id: i64,
    pub version_id: i64,                 // -> SdtmProjectVersion.id
    pub domain_code: String,             // unique per (version_id, domain_code)
    pub key_variables: Vec<String>,      // variable codes, in user-supplied order
    pub domain_override: Option<SdtmProjectDomainOverride>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/sdtm_project_domain_override.rs
pub struct SdtmProjectDomainOverride {
    pub id: i64,
    pub domain_id: i64,             // -> SdtmProjectDomain.id
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/sdtm_project_variable.rs
pub struct SdtmProjectVariable {
    pub id: i64,
    pub domain_id: i64,                 // -> SdtmProjectDomain.id
    pub variable_code: String,          // unique per (domain_id, variable_code)
    pub variable_sequence: i64,
    pub variable_override: Option<SdtmProjectVariableOverride>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/sdtm_project_variable_override.rs
pub struct SdtmProjectVariableOverride {
    pub id: i64,
    pub variable_id: i64,           // -> SdtmProjectVariable.id
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### `domain_override` / `variable_override` — a keyword that cannot be a field name

`override` is a **reserved** word in Rust (not merely a contextual one), so
`pub override: …` is a compile error. The field therefore carries a
descriptive name instead: `domain_override` on the domain aggregate and
its views, `variable_override` on the variable aggregate and its views.

The wire key is unaffected. The `apis` and `sdtm-project` view structs
carry no `Serialize` derive, so they never reach the wire; the server's
`dto.rs` does, and each field there keeps an explicit
`#[serde(rename = "override")]`. That explicit rename beats the struct's
`rename_all = "camelCase"`, so the key stays `"override"` rather than
`"domainOverride"`. The server's DTO tests assert that the camelCase
spelling does not leak.

The server's DTO structs name their fields the same way
(`domain_override` / `variable_override`) and rely on the same
`#[serde(rename = "override")]`.

### Copied value objects

`domain/domain_category.rs`, `domain/variable_type.rs`,
`domain/sdtm_domain_description.rs` and `domain/sdtm_variable_description.rs`
are **copies** of the `domain-model` originals. They keep the same variants,
the same `as_str()` methods and the same `TryFrom<&str>` conversions; the
`TryFrom` error type is this crate's own `DomainError`, with its own
`InvalidDomainCategory` / `InvalidVariableType` / `InvalidVariableCore` /
`InvalidVariableRole` variants.

```rust
pub enum DomainCategory {
    SpecialPurpose, Interventions, Events, Findings,
    TrialDesign, Relationships, StudyReference,
}
pub enum SdtmVariableType { Numeric, Character }
pub enum SdtmVariableCore { Req, Exp, Perm, Supp }
pub enum SdtmRole {
    Identifier, Topic, Timing, RecordQualifier,
    SynonymQualifier, VariableQualifier, GroupingQualifier, Rule,
}

pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}
```

### Aggregate → table mapping

| Aggregate | Table | Owning FK |
| --- | --- | --- |
| `SdtmProjectVersion` | `sdtm_project_versions` | — |
| `SdtmProjectDomain` | `sdtm_project_domains` | `version_id` → versions |
| `SdtmProjectDomainOverride` | `sdtm_project_domain_overrides` | `domain_id` → domains |
| `SdtmProjectVariable` | `sdtm_project_variables` | `domain_id` → domains |
| `SdtmProjectVariableOverride` | `sdtm_project_variable_overrides` | `variable_id` → variables |

## Schema

```
sdtm_project_versions
  id            BIGSERIAL PRIMARY KEY
  project_code  TEXT NOT NULL -- app-level validated against projects.code
  version_name  TEXT NOT NULL
  created_at, updated_at                       -- auto + BEFORE UPDATE trigger
  UNIQUE (project_code, version_name)

sdtm_project_domains
  id             BIGSERIAL PRIMARY KEY
  version_id     BIGINT NOT NULL REFERENCES sdtm_project_versions(id) ON DELETE CASCADE
  domain_code    TEXT NOT NULL
  key_variables  JSONB NOT NULL DEFAULT '[]'::jsonb
  created_at, updated_at
  UNIQUE (version_id, domain_code)
  CHECK (jsonb_typeof(key_variables) = 'array')

sdtm_project_domain_overrides
  id            BIGSERIAL PRIMARY KEY
  domain_id     BIGINT NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE
  category      TEXT NOT NULL
  descriptions  JSONB NOT NULL DEFAULT '[]'::jsonb
  created_at, updated_at
  UNIQUE (domain_id)
  CHECK (category IN ('Special Purpose','Interventions','Events','Findings',
                      'Trial Design','Relationships','Study Reference'))
  CHECK (jsonb_typeof(descriptions) = 'array')

sdtm_project_variables
  id                BIGSERIAL PRIMARY KEY
  domain_id         BIGINT NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE
  variable_code     TEXT NOT NULL
  variable_sequence BIGINT NOT NULL
  created_at, updated_at
  UNIQUE (domain_id, variable_code)

sdtm_project_variable_overrides
  id             BIGSERIAL PRIMARY KEY
  variable_id    BIGINT NOT NULL REFERENCES sdtm_project_variables(id) ON DELETE CASCADE
  variable_type  TEXT NOT NULL
  variable_core  TEXT NOT NULL
  variable_role  TEXT
  descriptions   JSONB NOT NULL DEFAULT '[]'::jsonb
  created_at, updated_at
  UNIQUE (variable_id)
  CHECK (variable_type IN ('Numeric','Character'))
  CHECK (variable_core IN ('Req','Exp','Perm','Supp'))
  CHECK (variable_role IS NULL OR variable_role IN ('Identifier','Topic','Timing',
      'Record Qualifier','Synonym Qualifier','Variable Qualifier',
      'Grouping Qualifier','Rule'))
  CHECK (jsonb_typeof(descriptions) = 'array')
```

One `BEFORE UPDATE` trigger per table refreshes `updated_at` to `NOW()` so
every code path (direct SQL, ad-hoc psql) is covered.

### `key_variables` — JSONB array, user order preserved

`key_variables` is an **ordered** collection, not a `HashSet`. The order the
caller supplies is meaningful and is preserved end to end: the domain stores
a `Vec<String>`, the JSONB column stores the array in that exact order, and
the wire returns the same order. PostgreSQL's `jsonb` type preserves array
element order — it only loses the order of *object* keys — so the ordering
survives the round trip untouched.

The collection keeps one piece of *set* semantics: duplicates are removed on
write, keeping the first occurrence. `AETERM, AESEV, AETERM` normalises to
`AETERM, AESEV`. A blank entry is rejected as a validation error rather
than silently dropped.

The `jsonb_typeof(...) = 'array'` CHECK guards against out-of-band inserts
of a non-array value.

### The two override tables

An override is stored in its **own table** rather than as a JSONB column on
the parent row, because the spec gives each override its own `id` and parent
FK and asks for CRUD over each model independently. `UNIQUE (domain_id)` and
`UNIQUE (variable_id)` enforce "each domain / variable can hold at most one
override" at the database level.

`ON DELETE CASCADE` on every FK means deleting a version removes its
domains, their overrides, their variables and their variable overrides.

## List strategy

| Method | Scope | Ordering |
| --- | --- | --- |
| `list_versions_by_project` | all versions of a project | `id ASC` |
| `list_domains_by_version` | all domains of a version | `id ASC` |
| `list_variables_by_domain` | all variables of a domain | `variable_sequence ASC, id ASC` |

Overrides are 1:1 with their parent and are never listed on their own; they
are read through `get_*_override` or embedded in the parent view.

## Override hydration — who reads and who writes

The parent aggregates carry `domain_override` / `variable_override:
Option<…>`, and the parent repositories **hydrate** that field with a
`LEFT JOIN` against the override table on every read (`find_by_id`,
`list_by_*`). Reads therefore return the parent and its override in a single
round trip, with no N+1.

The **update** path hydrates too. `UPDATE sdtm_project_domains d FROM
sdtm_project_domain_overrides o WHERE d.id = $1 RETURNING d.*, o.*` returns
the updated parent *and* its override in one statement, so
`PUT /domains/{id}` and `GET /domains/{id}` return the same shape. A
response that reported `"override": null` for a domain that demonstrably has
one would be a silent wrong-answer bug.

Writes go the other way: the **override repository owns the override table**.
`SdtmProjectDomainRepository::create` / `update` deliberately do not write to
`sdtm_project_domain_overrides`, and `SdtmProjectDomainOverrideRepository`
owns every insert / update / delete against it. There is exactly one writer
per table, so there is no ambiguity about which path last wrote the row.

Consequence: `create_domain` returns a domain whose `domain_override` is
`None` (a fresh domain cannot have an override yet), while
`get_domain_by_id` on the same row returns `Some(…)` once an override
exists. This is expected and is covered by tests.

## Cross-crate: project validation

`SdtmProjectVersion.project_code` references `Project.code` in the
`project` crate, whose table is owned by that crate's migrations. As with
`crf` and `mission`, this crate stores `project_code` as plain `TEXT` with
**no database-level foreign key** — the two crates' schemas stay independent
and either can be migrated first.

Existence is checked at the application level through a narrow port:

```rust
// domain/project_lookup.rs
#[async_trait]
pub trait ProjectLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
}
```

The adapter `adapter::service::project::ProjectLookupImpl` wraps
`Arc<dyn apis::project::ProjectService>` and maps
`ProjectApiError::NotFound` → `DomainError::ProjectNotFound(code)`; every
other error collapses to `DomainError::Repository`.

`create_version` calls the lookup **before** inserting. `update_version`
does not re-check: `project_code` is immutable after creation (see below), so
the parent project cannot change underneath an existing tree.

## Validation

| Aggregate | Rule | Error |
| --- | --- | --- |
| `SdtmProjectVersion::new` | `project_code` non-blank | `EmptyProjectCode` |
| | `version_name` non-blank | `EmptyVersionName` |
| `SdtmProjectDomain::new` | `domain_code` non-blank | `EmptyDomainCode` |
| `SdtmProjectVariable::new` | `variable_code` non-blank | `EmptyVariableCode` |
| `SdtmProjectDomain::new` | every `key_variables` entry non-blank | `EmptyKeyVariable` |
| overrides | enum-shaped; no string fields to validate | — |

The validating constructor `Foo::new(...)` is the domain-layer entry point;
`Foo::for_repository(...)` is `pub(crate)` and reserved for the adapter layer
(the in-memory fakes in `usecase/tests.rs` and `facade/in_memory/tests.rs` use
it, exactly as `domain-model` does). The usecase re-validates every command
before reaching the repository.

### Immutability decisions

- `SdtmProjectVersion.project_code` is **immutable** after creation. Moving a
  version to another project would orphan its entire domain tree and break
  the `(project_code, version_name)` uniqueness contract. `update_version`
  can change `version_name` only.
- `SdtmProjectDomain.version_id` is **immutable**. A domain always belongs to
  the version it was created under.
- `SdtmProjectVariable.domain_id` is **immutable**, for the same reason.

## Public API surface at the crate root

```rust
// lib/crates/sdtm-project/src/lib.rs
pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::SdtmProjectServiceImpl;
pub use adapter::persistence::postgres::{
    sdtm_project_domain_override_repo::SdtmProjectDomainOverrideRepoPg,
    sdtm_project_domain_repo::SdtmProjectDomainRepoPg,
    sdtm_project_variable_override_repo::SdtmProjectVariableOverrideRepoPg,
    sdtm_project_variable_repo::SdtmProjectVariableRepoPg,
    sdtm_project_version_repo::SdtmProjectVersionRepoPg,
};
pub use adapter::service::project::ProjectLookupImpl;

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
pub use usecase::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion, SdtmProjectDomainOverrideView,
    SdtmProjectDomainView, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
    SdtmProjectVariableOverrideView, SdtmProjectVariableView, SdtmProjectVersionView,
    UpdateSdtmProjectDomain, UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable,
    UpdateSdtmProjectVariableOverride, UpdateSdtmProjectVersion, UsecaseError,
};
```

## `DomainError` variants

```rust
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    // ---- validation (constructor-time) ----
    #[error("empty project code")]
    EmptyProjectCode,
    #[error("empty version name")]
    EmptyVersionName,
    #[error("empty domain code")]
    EmptyDomainCode,
    #[error("empty variable code")]
    EmptyVariableCode,
    #[error("empty key variable code")]
    EmptyKeyVariable,
    #[error("invalid domain category: {0}")]
    InvalidDomainCategory(String),
    #[error("invalid variable type: {0}")]
    InvalidVariableType(String),
    #[error("invalid variable core: {0}")]
    InvalidVariableCore(String),
    #[error("invalid variable role: {0}")]
    InvalidVariableRole(String),

    // ---- existence / FK / duplicate (runtime) ----
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("sdtm project version not found: {0}")]
    SdtmProjectVersionNotFound(i64),
    #[error("sdtm project domain not found: {0}")]
    SdtmProjectDomainNotFound(i64),
    #[error("sdtm project domain override not found: {0}")]
    SdtmProjectDomainOverrideNotFound(i64),
    #[error("sdtm project variable not found: {0}")]
    SdtmProjectVariableNotFound(i64),
    #[error("sdtm project variable override not found: {0}")]
    SdtmProjectVariableOverrideNotFound(i64),

    #[error("sdtm project version already exists: {project_code} / {version_name}")]
    DuplicateSdtmProjectVersion { project_code: String, version_name: String },
    #[error("sdtm project domain already exists: version {version_id} / {domain_code}")]
    DuplicateSdtmProjectDomain { version_id: i64, domain_code: String },
    #[error("sdtm project domain override already exists: domain {domain_id}")]
    DuplicateSdtmProjectDomainOverride { domain_id: i64 },
    #[error("sdtm project variable already exists: domain {domain_id} / {variable_code}")]
    DuplicateSdtmProjectVariable { domain_id: i64, variable_code: String },
    #[error("sdtm project variable override already exists: variable {variable_id}")]
    DuplicateSdtmProjectVariableOverride { variable_id: i64 },

    #[error("referenced sdtm project version not found: {0}")]
    FkSdtmProjectVersionNotFound(i64),
    #[error("referenced sdtm project domain not found: {0}")]
    FkSdtmProjectDomainNotFound(i64),
    #[error("referenced sdtm project variable not found: {0}")]
    FkSdtmProjectVariableNotFound(i64),

    #[error("not found")]
    NotFound,
    #[error("repository error: {0}")]
    Repository(String),
}
```

## Usecase

```rust
pub struct SdtmProjectUsecaseConfig<V, D, DO, Va, VO, P> {
    pub version_repo: V,
    pub domain_repo: D,
    pub domain_override_repo: DO,
    pub variable_repo: Va,
    pub variable_override_repo: VO,
    /// `Arc<P>` rather than `P`, so the call site can hand over an
    /// `Arc<ProjectLookupImpl>` without an extra generic parameter or a
    /// blanket `impl<T: ProjectLookup> ProjectLookup for Arc<T>`. This
    /// mirrors `crf::CrfUsecaseConfig`.
    pub projects: Arc<P>,
}

pub struct SdtmProjectUsecase<V, D, DO, Va, VO, P> { /* same six fields */ }
```

`P: ProjectLookup` is the sixth generic parameter, after the five
repositories.

### Methods

```rust
// ---- SdtmProjectVersion ----
async fn create_version(&self, cmd: CreateSdtmProjectVersion) -> Result<SdtmProjectVersionView, UsecaseError>;
async fn get_version_by_id(&self, id: i64) -> Result<SdtmProjectVersionView, UsecaseError>;
async fn list_versions_by_project(&self, project_code: &str) -> Result<Vec<SdtmProjectVersionView>, UsecaseError>;
async fn update_version(&self, cmd: UpdateSdtmProjectVersion) -> Result<SdtmProjectVersionView, UsecaseError>;
async fn delete_version(&self, id: i64) -> Result<(), UsecaseError>;

// ---- SdtmProjectDomain ----
async fn create_domain(&self, cmd: CreateSdtmProjectDomain) -> Result<SdtmProjectDomainView, UsecaseError>;
async fn get_domain_by_id(&self, id: i64) -> Result<SdtmProjectDomainView, UsecaseError>;
async fn list_domains_by_version(&self, version_id: i64) -> Result<Vec<SdtmProjectDomainView>, UsecaseError>;
async fn update_domain(&self, cmd: UpdateSdtmProjectDomain) -> Result<SdtmProjectDomainView, UsecaseError>;
async fn delete_domain(&self, id: i64) -> Result<(), UsecaseError>;

// ---- SdtmProjectDomainOverride ----
async fn create_domain_override(&self, cmd: CreateSdtmProjectDomainOverride) -> Result<SdtmProjectDomainOverrideView, UsecaseError>;
async fn get_domain_override(&self, domain_id: i64) -> Result<Option<SdtmProjectDomainOverrideView>, UsecaseError>;
async fn update_domain_override(&self, cmd: UpdateSdtmProjectDomainOverride) -> Result<SdtmProjectDomainOverrideView, UsecaseError>;
async fn delete_domain_override(&self, domain_id: i64) -> Result<(), UsecaseError>;

// ---- SdtmProjectVariable ----
async fn create_variable(&self, cmd: CreateSdtmProjectVariable) -> Result<SdtmProjectVariableView, UsecaseError>;
async fn get_variable_by_id(&self, id: i64) -> Result<SdtmProjectVariableView, UsecaseError>;
async fn list_variables_by_domain(&self, domain_id: i64) -> Result<Vec<SdtmProjectVariableView>, UsecaseError>;
async fn update_variable(&self, cmd: UpdateSdtmProjectVariable) -> Result<SdtmProjectVariableView, UsecaseError>;
async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError>;

// ---- SdtmProjectVariableOverride ----
async fn create_variable_override(&self, cmd: CreateSdtmProjectVariableOverride) -> Result<SdtmProjectVariableOverrideView, UsecaseError>;
async fn get_variable_override(&self, variable_id: i64) -> Result<Option<SdtmProjectVariableOverrideView>, UsecaseError>;
async fn update_variable_override(&self, cmd: UpdateSdtmProjectVariableOverride) -> Result<SdtmProjectVariableOverrideView, UsecaseError>;
async fn delete_variable_override(&self, variable_id: i64) -> Result<(), UsecaseError>;
```

Twenty methods, four per aggregate plus one extra list method each for
version / domain / variable.

### Parent existence is checked before the insert

Every `create_*` method verifies its parent exists **before** writing, and
a missing parent is a **404** — uniformly, whether the parent is the
project, a version, a domain or a variable:

| Create | Parent checked | Error when missing |
| --- | --- | --- |
| `create_version` | the project, via `ProjectLookup` | `ProjectNotFound(code)` |
| `create_domain` | `version_repo.find_by_id(version_id)` | `FkSdtmProjectVersionNotFound(id)` |
| `create_variable` | `domain_repo.find_by_id(domain_id)` | `FkSdtmProjectDomainNotFound(id)` |
| `create_domain_override` | `domain_repo.find_by_id(domain_id)` | `FkSdtmProjectDomainNotFound(id)` |
| `create_variable_override` | `variable_repo.find_by_id(variable_id)` | `FkSdtmProjectVariableNotFound(id)` |

The usecase already holds every parent repository, so this costs one extra
`find_by_id` round trip per create and needs no new ports. The database FK
remains the backstop for the race where the parent is deleted between the
check and the insert; that race surfaces as the same `Fk*NotFound` variant
and therefore the same 404.

### Override commands are keyed on the parent id

`UpdateSdtmProjectDomainOverride` carries `domain_id` (not the override's
own `id`), and `UpdateSdtmProjectVariableOverride` carries `variable_id`.
The parent's id is the natural key of a 1:1 sub-resource, and it lets the
HTTP layer address the override without exposing a surrogate id it never
needs.

`variable_role` on the variable-override update uses the three-state
`Option<Option<SdtmRole>>` convention already used by `domain-model`: outer
`None` = leave unchanged, `Some(None)` = clear to `NULL`, `Some(Some(role))`
= replace.

### Reading an override is not an error when there is none

An override is **optional**: a domain or variable that has never been
overridden is a normal state, not a broken one. So `get_domain_override` and
`get_variable_override` return `Result<Option<…>, _>` — the repository
still reports a missing row as `SdtmProject{Domain,Variable}OverrideNotFound`,
but the usecase turns exactly that variant into `Ok(None)`. Every other
repository error still propagates.

On the wire, `GET …/override` on a domain or variable with no override is
**200 with `{"override": null}`**, not 404. The 404 arms are gone from
those two routes' OpenAPI responses.

The response is always an *envelope* with an `override` key, never a bare
nullable override:

```jsonc
// no override
{ "override": null }
// with an override
{ "override": { "id": 3, "domainId": 2, "category": "Events", … } }
```

The key is present either way, so a client reads `body.override` without
first checking whether the body itself is null. The two envelope DTOs are
`SdtmProjectDomainOverrideResponse` and `SdtmProjectVariableOverrideResponse`
in the server's `dto.rs`; both name their field `domain_override` /
`variable_override` in Rust (the bare word `override` is a reserved
keyword) and rename it to `override` on the wire.

`update_*_override` and `delete_*_override` keep their 404: asking to
change or remove something that does not exist is still a client error.

### Views

```rust
pub struct SdtmProjectVersionView { pub id: i64, pub project_code: String, pub version_name: String, pub created_at, pub updated_at }
pub struct SdtmProjectDomainView { pub id: i64, pub version_id: i64, pub domain_code: String, pub key_variables: Vec<String>, pub domain_override: Option<SdtmProjectDomainOverrideView>, pub created_at, pub updated_at }
pub struct SdtmProjectDomainOverrideView { pub id: i64, pub domain_id: i64, pub category: DomainCategory, pub descriptions: Vec<SdtmDomainDescription>, pub created_at, pub updated_at }
pub struct SdtmProjectVariableView { pub id: i64, pub domain_id: i64, pub variable_code: String, pub variable_sequence: i64, pub variable_override: Option<SdtmProjectVariableOverrideView>, pub created_at, pub updated_at }
pub struct SdtmProjectVariableOverrideView { pub id: i64, pub variable_id: i64, pub variable_type: SdtmVariableType, pub variable_core: SdtmVariableCore, pub variable_role: Option<SdtmRole>, pub descriptions: Vec<SdtmVariableDescription>, pub created_at, pub updated_at }
```

Each view has a `From<Aggregate>` impl in `usecase/views.rs`.

## `apis::sdtm_project::SdtmProjectService` (the public-facing port)

New module `lib/crates/apis/src/sdtm_project.rs`, registered in
`apis/src/lib.rs`. The `apis` crate stays dependency-free and re-declares the
value objects it needs (`DomainCategory`, `SdtmVariableType`,
`SdtmVariableCore`, `SdtmRole`, `SdtmDomainDescription`(+Detail),
`SdtmVariableDescription`(+Detail)), exactly as `apis/src/domain_model.rs`
does. Adapters convert losslessly via matching variant constructors.

The trait mirrors the usecase's twenty methods:

```rust
#[async_trait]
pub trait SdtmProjectService: Send + Sync {
    async fn create_version(&self, req: CreateSdtmProjectVersionRequest) -> Result<SdtmProjectVersionView, SdtmProjectApiError>;
    async fn get_version_by_id(&self, id: i64) -> Result<SdtmProjectVersionView, SdtmProjectApiError>;
    async fn list_versions_by_project(&self, project_code: &str) -> Result<SdtmProjectVersionList, SdtmProjectApiError>;
    async fn update_version(&self, req: UpdateSdtmProjectVersionRequest) -> Result<SdtmProjectVersionView, SdtmProjectApiError>;
    async fn delete_version(&self, id: i64) -> Result<(), SdtmProjectApiError>;

    async fn create_domain(&self, req: CreateSdtmProjectDomainRequest) -> Result<SdtmProjectDomainView, SdtmProjectApiError>;
    async fn get_domain_by_id(&self, id: i64) -> Result<SdtmProjectDomainView, SdtmProjectApiError>;
    async fn list_domains_by_version(&self, version_id: i64) -> Result<SdtmProjectDomainList, SdtmProjectApiError>;
    async fn update_domain(&self, req: UpdateSdtmProjectDomainRequest) -> Result<SdtmProjectDomainView, SdtmProjectApiError>;
    async fn delete_domain(&self, id: i64) -> Result<(), SdtmProjectApiError>;

    async fn create_domain_override(&self, req: CreateSdtmProjectDomainOverrideRequest) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError>;
    async fn get_domain_override(&self, domain_id: i64) -> Result<Option<SdtmProjectDomainOverrideView>, SdtmProjectApiError>;
    async fn update_domain_override(&self, req: UpdateSdtmProjectDomainOverrideRequest) -> Result<SdtmProjectDomainOverrideView, SdtmProjectApiError>;
    async fn delete_domain_override(&self, domain_id: i64) -> Result<(), SdtmProjectApiError>;

    async fn create_variable(&self, req: CreateSdtmProjectVariableRequest) -> Result<SdtmProjectVariableView, SdtmProjectApiError>;
    async fn get_variable_by_id(&self, id: i64) -> Result<SdtmProjectVariableView, SdtmProjectApiError>;
    async fn list_variables_by_domain(&self, domain_id: i64) -> Result<SdtmProjectVariableList, SdtmProjectApiError>;
    async fn update_variable(&self, req: UpdateSdtmProjectVariableRequest) -> Result<SdtmProjectVariableView, SdtmProjectApiError>;
    async fn delete_variable(&self, id: i64) -> Result<(), SdtmProjectApiError>;

    async fn create_variable_override(&self, req: CreateSdtmProjectVariableOverrideRequest) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError>;
    async fn get_variable_override(&self, variable_id: i64) -> Result<Option<SdtmProjectVariableOverrideView>, SdtmProjectApiError>;
    async fn update_variable_override(&self, req: UpdateSdtmProjectVariableOverrideRequest) -> Result<SdtmProjectVariableOverrideView, SdtmProjectApiError>;
    async fn delete_variable_override(&self, variable_id: i64) -> Result<(), SdtmProjectApiError>;
}
```

`SdtmProjectApiError` mirrors `DomainModelApiError`'s shape: `Validation`,
`NotFound`, per-aggregate `*NotFound`, per-aggregate `Duplicate*`,
`Fk*NotFound`, `ProjectNotFound`, `Repository`.

## Persistence (sqlx, runtime API)

SQLx runtime API (`sqlx::query_as`, `sqlx::query`, `FromRow`) is used
throughout. The workspace ships no `.sqlx/` offline cache and no
`DATABASE_URL` at build time, so the compile-time-checked macros are
intentionally avoided. Each repo file carries the module-level comment
explaining this, as `domain-model` does.

`map_*_error` in each repo is the only place that knows about
`sqlx::Error`:

| Driver error | Domain error |
| --- | --- |
| `RowNotFound` | `DomainError::NotFound` (fallback only — see below) |
| `Database` with SQLSTATE `23505` | the matching `Duplicate*` variant |
| `Database` with SQLSTATE `23503` | the matching `Fk*NotFound` variant |
| anything else | `DomainError::Repository(msg)` |

The `Duplicate*` variants carry the natural key where the driver does not
surface it (the `domain-model` repos pass `0` / `"(unknown)"`; this crate
does the same and documents it).

**Missing rows produce the per-aggregate variant, not the generic
`NotFound`.** Every repository method that can miss a row uses
`fetch_optional(...).ok_or(DomainError::SdtmProjectVersionNotFound(id))` (or
the matching variant) rather than letting `sqlx::Error::RowNotFound` escape.
That is what `domain-model` does, and it is what makes the wire contract
useful: `GET /versions/{id}` on an unknown id returns
`404 sdtm_project_version_not_found`, not the generic `404 not_found`.
`DomainError::NotFound` survives only as the `sqlx::Error::RowNotFound`
fallback inside `map_*_error` and is never the response a client sees.

### Override upsert semantics in SQL

The override repos expose plain `create` / `update` / `delete` keyed on the
parent id:

- `create` → `INSERT INTO sdtm_project_domain_overrides (domain_id, …) VALUES ($1, …) RETURNING …`.
  A second override for the same domain violates `UNIQUE (domain_id)` and
  surfaces as `DuplicateSdtmProjectDomainOverride`.
- `update` → `UPDATE … SET category = COALESCE($2, category), descriptions = COALESCE($3, descriptions) WHERE domain_id = $1 RETURNING …`.
  A missing row surfaces as `SdtmProjectDomainOverrideNotFound(domain_id)`.
- `delete` → `DELETE FROM … WHERE domain_id = $1`; zero rows affected is the
  same not-found error.

## Facade — `adapter/facade/in_memory/service.rs`

`SdtmProjectServiceImpl<V, D, DO, Va, VO, P>` implements
`apis::sdtm_project::SdtmProjectService` on top of a
`SdtmProjectUsecase`. It is generic over the six ports so it can be exercised
against in-memory fakes in tests and against the PostgreSQL repos in
production — the same pattern as `domain-model`'s `DomainModelServiceImpl`.

Constructors:

```rust
impl<V, D, DO, Va, VO, P> SdtmProjectServiceImpl<V, D, DO, Va, VO, P> {
    pub fn new(usecase: SdtmProjectUsecase<V, D, DO, Va, VO, P>) -> Self;
    pub fn from_repos(version_repo: V, domain_repo: D, domain_override_repo: DO,
                      variable_repo: Va, variable_override_repo: VO, projects: P) -> Self;
}
```

The facade carries the enum / description mappers between the `apis` copies
and the crate's own copies — the same exhaustive `match` pairs that
`domain-model`'s facade uses.

`domain-model`'s `From<UsecaseError> for DomainModelApiError` ends with an
`unreachable!()` arm for the validation-only `DomainError` variants. This
crate's equivalent arm maps them to `SdtmProjectApiError::Repository`
instead: a panic in a request handler is strictly worse than a 500, and the
`Invalid*` variants are in principle reachable if a stored row carries a
value the CHECK constraint did not catch.

## Server HTTP surface

New module `apps/server/aegis-server/src/transport/http/sdtm_project/`
with `router.rs` and `handlers.rs`, following the `domain_model` module's
layout (one `routes!` call per HTTP verb, because `utoipa-axum` 0.2 panics
when a single `routes!` receives two handlers sharing a method).

The router is nested under `/api/sdtm-project` in `transport/http/router.rs`.

### Routes

| Method | Path | Handler | Success |
| --- | --- | --- | --- |
| POST | `/api/sdtm-project/versions` | `create_version` | 201 |
| GET | `/api/sdtm-project/versions?projectCode=…` | `list_versions_by_project` | 200 |
| GET | `/api/sdtm-project/versions/{id}` | `get_version_by_id` | 200 |
| PUT | `/api/sdtm-project/versions/{id}` | `update_version` | 200 |
| DELETE | `/api/sdtm-project/versions/{id}` | `delete_version` | 204 |
| POST | `/api/sdtm-project/domains` | `create_domain` | 201 |
| GET | `/api/sdtm-project/domains/{id}` | `get_domain_by_id` | 200 |
| GET | `/api/sdtm-project/versions/{version_id}/domains` | `list_domains_by_version` | 200 |
| PUT | `/api/sdtm-project/domains/{id}` | `update_domain` | 200 |
| DELETE | `/api/sdtm-project/domains/{id}` | `delete_domain` | 204 |
| POST | `/api/sdtm-project/domains/{domain_id}/override` | `create_domain_override` | 201 |
| GET | `/api/sdtm-project/domains/{domain_id}/override` | `get_domain_override` | 200 (`{"override": …}`) |
| PUT | `/api/sdtm-project/domains/{domain_id}/override` | `update_domain_override` | 200 |
| DELETE | `/api/sdtm-project/domains/{domain_id}/override` | `delete_domain_override` | 204 |
| POST | `/api/sdtm-project/variables` | `create_variable` | 201 |
| GET | `/api/sdtm-project/variables/{id}` | `get_variable_by_id` | 200 |
| GET | `/api/sdtm-project/domains/{domain_id}/variables` | `list_variables_by_domain` | 200 |
| PUT | `/api/sdtm-project/variables/{id}` | `update_variable` | 200 |
| DELETE | `/api/sdtm-project/variables/{id}` | `delete_variable` | 204 |
| POST | `/api/sdtm-project/variables/{variable_id}/override` | `create_variable_override` | 201 |
| GET | `/api/sdtm-project/variables/{variable_id}/override` | `get_variable_override` | 200 (`{"override": …}`) |
| PUT | `/api/sdtm-project/variables/{variable_id}/override` | `update_variable_override` | 200 |
| DELETE | `/api/sdtm-project/variables/{variable_id}/override` | `delete_variable_override` | 204 |

`POST` on an override path creates the override and returns **409** if one
already exists. `PUT` updates an existing override and returns **404** if
there is none. There is no `PUT` upsert — create and update are distinct,
matching the "CRUD methods for models" requirement.

`GET` on an override path returns **200** with the override, or **200 with a
`null` body** when the domain or variable has no override — the override is
optional, so a missing one is not a failure.

`GET /versions` takes a **required** `projectCode` query parameter; a
request without it is rejected by axum's query extractor with 400. An
unknown project code is **not** an error — it returns `200` with an empty
`versions` array, matching how the other scoped list endpoints behave.

### Wire DTOs

`apps/server/aegis-server/src/transport/http/dto.rs` gains the
`sdtm-project` wire types, all `#[serde(rename_all = "camelCase")]` and
`#[derive(ToSchema)]`:

- `SdtmProjectVersionViewResponse`, `SdtmProjectDomainViewResponse`,
  `SdtmProjectDomainOverrideViewResponse`, `SdtmProjectVariableViewResponse`,
  `SdtmProjectVariableOverrideViewResponse`
- `SdtmProjectVersionListResponse`, `SdtmProjectDomainListResponse`,
  `SdtmProjectVariableListResponse`
- `SdtmProjectDomainOverrideResponse`, `SdtmProjectVariableOverrideResponse` —
  the `GET …/override` envelopes; each carries a single `override` key
- `CreateSdtmProjectVersionRequest`, `UpdateSdtmProjectVersionRequest`, … for
  all five aggregates
- `SdtmProjectVersionListQuery { project_code: String }` — the only list
  query in the namespace; the domain and variable lists are scoped by a
  path parameter and need no query DTO.

The wire DTOs re-declare `DomainCategory`, `SdtmVariableType`,
`SdtmVariableCore`, `SdtmRole`, `SdtmDomainDescription`(+Detail) and
`SdtmVariableDescription`(+Detail) as **new** types in the `sdtm-project`
section of `dto.rs` — they are *not* reused from the `domain-model` section,
because the two feature areas are allowed to diverge. The server's
`dto.rs` therefore carries two parallel sets of these enums.

The domain and variable view responses carry the nested override with
`#[serde(rename = "override")]`, so the JSON is:

```json
{
  "id": 7,
  "versionId": 3,
  "domainCode": "AE",
  "keyVariables": ["AETERM", "AESEV"],
  "override": {
    "id": 1,
    "domainId": 7,
    "category": "Events",
    "descriptions": [ { "lang": "en", "details": { "description": "…", "structure": "…" } } ]
  },
  "createdAt": "2026-10-23T00:00:00Z",
  "updatedAt": "2026-10-23T00:00:00Z"
}
```

### Error mapping

`transport/http/error.rs` gains:

```rust
#[error("{0}")]
SdtmProject(#[from] apis::sdtm_project::SdtmProjectApiError),
```

plus the `status()` / `code()` dispatch arms:

| `SdtmProjectApiError` | HTTP | `code` |
| --- | --- | --- |
| `Validation(_)` | 400 | `validation_failed` |
| `NotFound` | 404 | `not_found` |
| `ProjectNotFound(_)` | 404 | `project_not_found` |
| `SdtmProjectVersionNotFound(_)` | 404 | `sdtm_project_version_not_found` |
| `SdtmProjectDomainNotFound(_)` | 404 | `sdtm_project_domain_not_found` |
| `SdtmProjectDomainOverrideNotFound(_)` | 404 | `sdtm_project_domain_override_not_found` |
| `SdtmProjectVariableNotFound(_)` | 404 | `sdtm_project_variable_not_found` |
| `SdtmProjectVariableOverrideNotFound(_)` | 404 | `sdtm_project_variable_override_not_found` |
| `Duplicate*` | 409 | `duplicate_sdtm_project_version` / `…_domain` / `…_domain_override` / `…_variable` / `…_variable_override` |
| `Fk*NotFound` | 404 | `fk_sdtm_project_version_not_found` / … |
| `Repository(_)` | 500 | `repository_error` |

### Auth policy

**Any authenticated user** may read and write. Every handler takes
`AuthClaims` (as `crf`'s handlers do) and none of them calls
`require_admin_or_root`. The `crf` module is the precedent: project-scoped
clinical content is not role-gated, and the sdtm-project tree follows the
same rule.

Because no role gate exists, the sdtm-project write routes do **not**
advertise a `403` response in their `#[utoipa::path]` block. (`crf` does
advertise one even though it enforces no role gate — that is a documentation
inconsistency there, not a convention worth copying.) The router's OpenAPI
test asserts BearerAuth on every sdtm-project route but asserts no 403 for
any of them.

### Server wiring

- `AppState` gains `pub sdtm_project: Arc<dyn apis::sdtm_project::SdtmProjectService>`.
  `AppState`'s test-support module gains a null double for it, so the other
  modules' test doubles stay compiling.
- `run.rs` gains `build_sdtm_project_service(pool, project)`:

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
        version_repo, domain_repo, domain_override_repo,
        variable_repo, variable_override_repo, projects,
    });
    Arc::new(SdtmProjectServiceImpl::new(usecase))
}
```

  The `project` service is already built in `run.rs` and is passed in, so the
  project-existence check reuses the canonical `projects` table.
- `transport/http/openapi.rs` registers the new schemas.
- `transport/http/router.rs` nests the new router under `/api/sdtm-project`.

## Tests

Per the workspace guideline (§9), in this order:

1. **Domain unit tests** — `src/domain/tests.rs`: the four copied enums'
   `TryFrom<&str>` round trips and rejection of unknown strings; the
   validating constructors rejecting blank codes; ordered `key_variables`
   handling.
2. **Adapter unit tests** — `src/adapter/persistence/postgres/*_repo.rs`,
   each with a `#[cfg(test)] mod tests` that reads its migration file with
   `include_str!` and asserts the table, the unique constraint, the CHECK
   constraints and the `updated_at` trigger, so the schema cannot regress
   silently.
3. **Usecase tests** — `src/usecase/tests.rs`: in-memory fakes for the five
   repos plus a fake `ProjectLookup`, exercising each aggregate's CRUD
   round trip and each validation failure.
4. **Facade tests** — `src/adapter/facade/in_memory/tests.rs`: the facade
   wired on top of in-memory ports, exercising the api-level round trip,
   the enum conversions in both directions, the three-state
   `variable_role` update, and the `UsecaseError → SdtmProjectApiError`
   mapping.
5. **`tests/public_api.rs`** — compile-only. Names every documented consumer
   import, pins the constructor chain (`fn(PgPool) -> _`, `fn(R) -> _` as
   function pointers) and asserts the trait bounds the usecase relies on.
6. **`tests/integration_persistence.rs`** — live-database round trips,
   `#[ignore]`-gated.

### Live-DB test harness

- Env var: `AEGIS_SDTM_PROJECT_DATABASE_URL` (loaded via `dotenvy`).
- The test applies the crate's migrations, then **drops the live tables and
  the `_sqlx_migrations` bookkeeping table** so each run starts clean.
  Destructive against a real database on purpose.
- **The URL must point at a dedicated database, not the server's.** SQLx
  keeps a single `_sqlx_migrations` table *per database*, shared by every
  `--source` migration run. Dropping it in the server's database would erase
  the applied-migration record for auth, user, project, terminology, mission
  and crf alike, and the next `sqlx migrate run` would try to re-apply every
  one of them. This is the same convention the other crates follow with
  their `AEGIS_<CRATE>_DATABASE_URL` variables; none of them are defined in
  the workspace `.env` yet.
- Run with `cargo test -p sdtm-project -- --ignored --test-threads=1`.
- Columns with a `UNIQUE` constraint get a per-run unique value (atomic
  counter + wall-clock nanoseconds) so concurrent runs do not collide.

### Migrating the server's database

The server does not auto-migrate. To create the five `sdtm_project_*` tables
in the server's database:

```bash
sqlx migrate run --source lib/crates/sdtm-project/migrations
```

run against `AEGIS_DATABASE_URL`. This is the same per-crate `--source`
invocation the other crates use; the empty `migrations/<crate>/` directories
at the workspace root are placeholders and are not used.

## Dependencies (`Cargo.toml`)

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
# `created_at` / `updated_at`.
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

## Workspace wiring

`Cargo.toml` `[workspace].members` gains `"lib/crates/sdtm-project"`.

The server's `Cargo.toml` gains `sdtm-project = { path = "../../../lib/crates/sdtm-project" }`.
The `apis` crate gains no new dependency (the port is self-contained).

## `README.md`

`lib/crates/sdtm-project/README.md` covers: one-sentence purpose; the `src/`
tree; database setup (`sqlx migrate run --source lib/crates/sdtm-project/migrations`,
the `AEGIS_SDTM_PROJECT_DATABASE_URL` env var, a constructor snippet); how to
run the ignored tests; and a back-link to
`docs/guidelines/lib-crate-development.md`.

The README must state prominently that `AEGIS_SDTM_PROJECT_DATABASE_URL` has
to point at a **dedicated** database, because the live test drops
`_sqlx_migrations` — a table SQLx shares across every crate that migrates
that database.

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p sdtm-project --all-targets --all-features -- -D warnings
cargo test -p sdtm-project
cargo doc -p sdtm-project --no-deps
cargo clippy -p apis --all-targets --all-features -- -D warnings
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test -p sdtm-project -- --ignored --test-threads=1   # when AEGIS_SDTM_PROJECT_DATABASE_URL is set
```

## Out of scope

- **The desktop app.** `apps/desktop/aegis-desktop` is not touched by this
  change — no Tauri commands, no TS feature module, no wire-DTO mirror in
  `src/shared/api/types.ts`.
- **Any UI.** The crate and the HTTP API are the deliverable.
- **Copying or importing the `domain-model` standard.** The sdtm-project
  tree is standalone; nothing links a project version to a `SdtmVersion` row.
- **Search.** No ILIKE / full-text search, unlike the `crf` crate.
- **Pagination.** Every list endpoint returns the full set, wrapped in a
  `*List` DTO so the response shape can grow later.
- **Moving a version between projects** (`project_code` is immutable).

## Open questions resolved with the requester

1. **Override storage** — separate tables with `UNIQUE` on the parent FK
   (chosen) rather than embedded JSONB columns on the parent row.
2. **Auth policy** — any authenticated user may write (chosen), matching the
   `crf` module rather than the `domain-model` module's admin/root gate.
3. **`project_code` integrity** — application-level check through a
   `ProjectLookup` port (chosen), matching the `crf` / `mission` precedent,
   rather than a database foreign key into the `project` crate's table.
4. **Missing parent on create** — a **404**, uniformly. The usecase
   pre-checks the parent before the insert rather than letting the database
   FK violation decide the status code.
5. **`key_variables` ordering** — the caller's order is authoritative and is
   preserved through the domain, the JSONB column and the wire.
6. **Reading a missing override** — not an error. The override is optional,
   so `get_*_override` returns `Option` and the HTTP route answers **200
   with `{"override": null}`** rather than 404.

## Decisions made by the implementer

1. `created_at` / `updated_at` on all five tables, `DEFAULT NOW()` plus a
   `BEFORE UPDATE` trigger — the workspace convention.
2. `key_variables` is an **ordered** `Vec<String>`, not a `HashSet` — the
   requester's clarification: the order is always the caller's. Duplicates
   are removed on write (first occurrence wins) and blank entries are
   rejected, but nothing is ever re-sorted.
3. The variable aggregate carries `variable_override` too, even though the
   spec lists the override field only on `SdtmProjectDomain` — the two
   aggregates are structurally parallel and the asymmetry reads as an
   oversight.
4. `project_code`, `version_id` and `domain_id` are immutable after
   creation; only `version_name`, `domain_code`, `variable_code`,
   `variable_sequence` and the override payloads are mutable.
5. The override repos are keyed on the **parent id**, not the override's own
   surrogate id.
6. `create_version` validates the project through `ProjectLookup`;
   `update_version` does not re-validate (the project cannot change).
7. `SdtmProjectUsecaseConfig.projects` is `Arc<P>` (the `crf` crate's
   shape) so the call site can pass an `Arc<ProjectLookupImpl>` directly.
8. The `update` paths hydrate the override, so `PUT` and `GET` on a parent
   return the same shape.
9. The write routes do not advertise a `403` response, because no role gate
   is enforced.
10. The facade's `From<UsecaseError>` maps the validation-only domain
    errors to `Repository` rather than `unreachable!()`, so a bad stored
    row yields a 500 instead of a panic.
