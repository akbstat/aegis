//! Adapter layer.
//!
//! Houses the persistence adapters that implement the domain ports,
//! plus outbound-port adapters that adapt the usecase layer to
//! API-facing traits defined in other workspace crates.

pub(crate) mod cache;
pub(crate) mod facade;
pub(crate) mod persistence;
pub mod service;

pub use cache::in_memory::token_version::InMemoryTokenVersionCache;
pub use facade::internal::AuthServiceImpl;
pub use persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};
