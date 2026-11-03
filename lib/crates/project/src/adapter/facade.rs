//! Facade adapters — adapt the in-crate usecase to the apis ports.
//!
//! The only facade today is `ProjectServiceImpl`, which implements
//! `apis::project::ProjectService` on top of `ProjectUsecase`.

pub mod internal;
