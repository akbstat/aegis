//! Adapter layer.
//!
//! Hosts three sub-modules:
//! - `persistence` — SQLx/PostgreSQL implementations of the repository
//!   ports.
//! - `service` — adapters that bridge another workspace crate's port
//!   into a domain port.
//! - `facade` — the implementation of the API trait defined in
//!   `apis::structures::StructureService`.

pub mod facade;
pub mod persistence;
pub mod service;
