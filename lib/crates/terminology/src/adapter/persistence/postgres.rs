//! PostgreSQL-backed implementations of the three terminology
//! repository ports. Each repo uses SQLx's *runtime* query API
//! (`sqlx::query_as`, `QueryBuilder`) rather than the compile-time
//! macros, mirroring the user / project crates.

pub mod code_item_repo;
pub mod code_list_repo;
pub mod terminology_version_repo;
mod tsquery;

pub use code_item_repo::CodeItemRepo;
pub use code_list_repo::CodeListRepo;
pub use terminology_version_repo::TerminologyVersionRepo;
