# domain-model

CRUD over the CDISC SDTM domain model aggregates
(`SdtmVersion`, `SdtmDomain`, `SdtmVariable`), backed by
PostgreSQL.

## Layered architecture

```
domain-model crate
└── adapter
    ├── facade             (internal, generic over V/D/Va)
    └── persistence        (postgres, sqlx runtime API)
usecase
└── DomainModelUsecase<V, D, Va>
    └── commands / views / UsecaseError
domain
└── DomainCategory, SdtmVariableType, SdtmVariableCore, SdtmRole,
    SdtmVariableControlled
    └── SdtmVersion, SdtmDomain, SdtmVariable
    └── SdtmVersionRepository, SdtmDomainRepository,
        SdtmVariableRepository
    └── DomainError
```

`adapter::persistence::postgres::*RepoPg` implements the three
ports. `adapter::facade::internal::service::DomainModelServiceImpl`
adapts `DomainModelUsecase` to the
`apis::domain_model::DomainModelService` outbound port.

## Data model

| Aggregate      | Fields                                                                |
| -------------- | --------------------------------------------------------------------- |
| `SdtmVersion`  | `id`, `name` (unique), `created_at`, `updated_at`                     |
| `SdtmDomain`   | `id`, `version_id` (FK CASCADE), `name`, `category`, `descriptions` (JSONB), `created_at`, `updated_at` |
| `SdtmVariable` | `id`, `domain_id` (FK CASCADE), `name`, `variable_controlled` (JSONB), `variable_type`, `variable_core`, `variable_role`, `variable_sequence`, `descriptions` (JSONB), `created_at`, `updated_at` |

`descriptions` carries `Vec<SdtmDomainDescription>` /
`Vec<SdtmVariableDescription>` as a single JSONB column
(`NOT NULL DEFAULT '[]'::jsonb`).

`variable_controlled` carries an `SdtmVariableControlled` as a single
nullable JSONB column: `{"code": ["C66731"], "format": ["Codelist"]}`.
Both lists are optional, and a list that is not set is omitted from
the JSON rather than written as `null` — so the stored shape is
`{"code": ["C66731"]}` when only `code` is set.

## HTTP surface

Mounted under `/api/domain-model/*` in `aegis-server`. Every
write route (`POST`, `PUT`, `DELETE`) calls
`require_admin_or_root(&claims)?;` first. Reads require only
authenticated claims.

```
GET    /api/domain-model/versions
POST   /api/domain-model/versions           (admin/root)
PUT    /api/domain-model/versions/:id       (admin/root)
DELETE /api/domain-model/versions/:id       (admin/root)

POST   /api/domain-model/domains            (admin/root)
GET    /api/domain-model/domains/:id
GET    /api/domain-model/versions/:version_id/domains
PUT    /api/domain-model/domains/:id        (admin/root)
DELETE /api/domain-model/domains/:id        (admin/root)

POST   /api/domain-model/variables          (admin/root)
GET    /api/domain-model/variables/:id
GET    /api/domain-model/domains/:domain_id/variables
PUT    /api/domain-model/variables/:id      (admin/root)
DELETE /api/domain-model/variables/:id      (admin/root)
```

## Database setup

```bash
sqlx migrate run --source lib/crates/domain-model/migrations
```

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/domain-model/migrations --timestamp <description>
```

## Verification

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

Live-DB integration tests (gated with `#[ignore]`) require
`AEGIS_TEST_DATABASE_URL`:

```bash
cargo test -p domain-model -- --ignored --test-threads=1
```

Spec: `docs/superpowers/specs/2026-08-24-domain-model-crate-design.md`.
Conventions: `docs/guidelines/lib-crate-development.md`.