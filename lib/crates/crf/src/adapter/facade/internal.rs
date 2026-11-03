//! The crate's own `CrfService` facade.
//!
//! Hosts [`CrfServiceImpl`] — the implementation of
//! `apis::crf::CrfService` that adapts `CrfUsecase` to the API
//! contract.

mod service;

#[cfg(test)]
mod tests;

pub use service::CrfServiceImpl;
