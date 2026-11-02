//! `StructureService` adapter backed by the usecase.
//!
//! Hosts `StructureServiceImpl` — the implementation of
//! `apis::structures::StructureService` that adapts
//! `structures::usecase::StructureUsecase` to the API contract.

pub mod service;
