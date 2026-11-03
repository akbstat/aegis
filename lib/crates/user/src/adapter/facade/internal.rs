//! Outbound-port adapters that sit on top of the usecase.
//!
//! Today this module hosts a single implementation — the
//! [`UserServiceImpl`](service::UserServiceImpl) that adapts
//! `user::UserUsecase` to `apis::user::UserService`. The adapter
//! is generic over the persistence port, so the same impl serves
//! every storage backend.

mod service;

pub use service::UserServiceImpl;

#[cfg(test)]
mod tests;
