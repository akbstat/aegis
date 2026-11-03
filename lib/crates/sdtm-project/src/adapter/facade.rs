//! Outbound-port adapters.
//!
//! Adapters in this sub-module implement API-facing traits defined in
//! other workspace crates (today: `apis::sdtm_project::SdtmProjectService`).

mod internal;

pub use internal::SdtmProjectServiceImpl;
