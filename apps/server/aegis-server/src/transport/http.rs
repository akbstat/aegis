//! HTTP transport.
//!
//! Hosts the axum `Router` composition, the auth handlers, the wire
//! DTOs, the `ErrorBody` + `ApiError` mapping, the healthz handler,
//! and the utoipa OpenAPI builder. Sub-modules re-export their
//! public surface here so consumers can write
//! `use aegis_server::transport::http::router` or
//! `use aegis_server::transport::router` (the outer re-export).

pub mod auth;
pub mod crf;
pub mod domain_model;
pub mod dto;
pub mod error;
pub mod healthz;
pub mod log_ingest;
pub mod mission;
pub mod openapi;
pub mod project;
pub mod router;
pub mod sdtm_project;
pub mod structures;
pub mod terminology;
pub mod trace_id;
pub mod user;

pub use router::router;
