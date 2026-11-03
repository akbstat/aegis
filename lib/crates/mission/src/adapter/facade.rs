//! Facade adapters — adapt the in-crate usecase to the apis ports.
//!
//! The only facade today is `MissionServiceImpl`, which implements
//! `apis::mission::MissionService` on top of `MissionUsecase`.

pub mod internal;
