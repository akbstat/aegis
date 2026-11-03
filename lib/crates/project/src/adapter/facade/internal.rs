//! The crate's own `ProjectService` facade: the only facade
//! implementation today.
//!
//! Holds a `ProjectUsecase<P, R, U>` and projects its results into
//! the apis `ProjectView` / `ProductView` types. The adapter is
//! generic over the repository ports, so the same impl serves
//! every storage backend.

mod service;
#[cfg(test)]
mod tests;

pub use service::ProjectServiceImpl;
