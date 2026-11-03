//! Usecase layer.
//!
//! `CrfUsecase<V, F, I, O, U, Da, A, P>` orchestrates the seven
//! aggregates and the cross-crate project lookup, projects
//! domain aggregates into view DTOs, and surfaces
//! `UsecaseError`. The facade in
//! `adapter::facade::internal::CrfServiceImpl` adapts this
//! type to `apis::crf::CrfService`.

mod commands;
mod crf_usecase;
mod error;
mod views;

#[cfg(test)]
pub(crate) mod tests;

pub use commands::{
    CreateAnnotation, CreateCrfBulkForm, CreateCrfBulkItem, CreateCrfForm, CreateCrfItem,
    CreateCrfOption, CreateCrfUnit, CreateCrfVersion, CreateDomainAnnotation,
    SearchAnnotationsByVersion, SearchCrfFormsByVersion, SearchCrfItemsByVersion,
    SearchCrfOptionsByVersion, SearchCrfUnitsByVersion, SearchDomainAnnotationsByVersion,
    SetCrfApproved, UpdateAnnotation, UpdateCrfForm, UpdateCrfItem, UpdateCrfOption, UpdateCrfUnit,
    UpdateCrfVersion, UpdateDomainAnnotation,
};
pub use crf_usecase::{CrfUsecase, CrfUsecaseConfig};
pub use error::UsecaseError;
pub use views::{
    AnnotationView, CrfBulkFormResult, CrfFormDetailView, CrfFormView, CrfItemDetailView,
    CrfItemView, CrfOptionDetailView, CrfOptionView, CrfUnitDetailView, CrfUnitView,
    CrfVersionView, DomainAnnotationView,
};
