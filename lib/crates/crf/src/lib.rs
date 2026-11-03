//! # crf crate
//!
//! Workspace library providing CRUD over the Case Report Form
//! aggregates (`CrfVersion`, `CrfForm`, `CrfItem`, `CrfOption`,
//! `CrfUnit`, `DomainAnnotation`, `Annotation`) and version-scoped
//! ILIKE search, backed by PostgreSQL.
//!
//! Layered architecture:
//!
//! - `domain` — value objects, aggregates, ports (`*Repository`,
//!   `ProjectLookup`), and `DomainError`. No I/O.
//! - `usecase` — `CrfUsecase<V, F, I, O, U, Da, A, P>` orchestrates
//!   CRUD + search and projects aggregates into view DTOs.
//! - `adapter` — concrete implementations:
//!   - `adapter::persistence::postgres` — `*RepoPg` per port
//!     (SQLx runtime API).
//!   - `adapter::service::project::ProjectLookupImpl` — bridges
//!     `apis::project::ProjectService` to the domain
//!     `ProjectLookup`.
//!   - `adapter::facade::internal::CrfServiceImpl` — adapts
//!     `CrfUsecase` to `apis::crf::CrfService`.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::internal::CrfServiceImpl;
pub use adapter::persistence::postgres::annotation_repo::AnnotationRepoPg;
pub use adapter::persistence::postgres::crf_bulk_form_repo::CrfBulkFormRepoPg;
pub use adapter::persistence::postgres::crf_form_repo::CrfFormRepoPg;
pub use adapter::persistence::postgres::crf_item_repo::CrfItemRepoPg;
pub use adapter::persistence::postgres::crf_option_repo::CrfOptionRepoPg;
pub use adapter::persistence::postgres::crf_unit_repo::CrfUnitRepoPg;
pub use adapter::persistence::postgres::crf_version_repo::CrfVersionRepoPg;
pub use adapter::persistence::postgres::domain_annotation_repo::DomainAnnotationRepoPg;
pub use adapter::service::project::project_lookup_impl::ProjectLookupImpl;

pub use domain::{
    Annotation, AnnotationOwner, AnnotationRepository, CrfBulkCreateForm, CrfBulkCreateFormResult,
    CrfBulkCreateItem, CrfBulkFormRepository, CrfForm, CrfFormNew, CrfFormRepository,
    CrfFormUpdate, CrfItem, CrfItemKind, CrfItemNew, CrfItemRepository, CrfItemUpdate, CrfOption,
    CrfOptionNew, CrfOptionRepository, CrfOptionUpdate, CrfUnit, CrfUnitNew, CrfUnitRepository,
    CrfUnitUpdate, CrfVersion, CrfVersionNew, CrfVersionRepository, CrfVersionUpdate,
    DomainAnnotation, DomainAnnotationNew, DomainAnnotationRepository, DomainAnnotationUpdate,
    DomainError, ProjectLookup,
};

pub use usecase::{
    AnnotationView, CreateAnnotation, CreateCrfBulkForm, CreateCrfBulkItem, CreateCrfForm,
    CreateCrfItem, CreateCrfOption, CreateCrfUnit, CreateCrfVersion, CreateDomainAnnotation,
    CrfBulkFormResult, CrfFormDetailView, CrfFormView, CrfItemDetailView, CrfItemView,
    CrfOptionDetailView, CrfOptionView, CrfUnitDetailView, CrfUnitView, CrfUsecase,
    CrfUsecaseConfig, CrfVersionView, DomainAnnotationView, SearchAnnotationsByVersion,
    SearchCrfFormsByVersion, SearchCrfItemsByVersion, SearchCrfOptionsByVersion,
    SearchCrfUnitsByVersion, SearchDomainAnnotationsByVersion, UpdateAnnotation, UpdateCrfForm,
    UpdateCrfItem, UpdateCrfOption, UpdateCrfUnit, UpdateCrfVersion, UpdateDomainAnnotation,
    UsecaseError,
};
