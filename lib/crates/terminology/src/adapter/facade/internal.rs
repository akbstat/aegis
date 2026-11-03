//! Outbound-port adapters that sit on top of the usecase.
//!
//! Hosts [`TerminologyServiceImpl`] — the implementation of
//! `apis::terminology::TerminologyService` that adapts
//! `terminology::TerminologyUsecase` to the API contract.
//! Behaviour is exercised by `tests`, which wire the adapter on
//! top of in-memory `TerminologyVersionRepository` /
//! `CodeListRepository` / `CodeItemRepository` implementations so
//! no live PostgreSQL connection is required.

mod service;

#[cfg(test)]
mod tests;

pub use service::TerminologyServiceImpl;
