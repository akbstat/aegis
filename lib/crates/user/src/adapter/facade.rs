//! Outbound-port adapters.
//!
//! Adapters in this sub-module implement API-facing traits defined
//! in other workspace crates (today: `apis::user::UserService`).
//! A second port (e.g. a future gRPC facade) can be added as a
//! sibling module.

mod internal;

pub use internal::UserServiceImpl;
