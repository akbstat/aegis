//! Outbound-port adapters that sit on top of the usecase.
//!
//! Hosts [`DomainModelServiceImpl`] — the implementation of
//! `apis::domain_model::DomainModelService` that adapts
//! `domain_model::DomainModelUsecase` to the API contract.
//! Behaviour is exercised by `tests`, which wire the adapter on
//! top of in-memory `SdtmVersionRepository` /
//! `SdtmDomainRepository` / `SdtmVariableRepository`
//! implementations so no live PostgreSQL connection is required.

mod service;

#[cfg(test)]
mod tests;

pub use service::DomainModelServiceImpl;
