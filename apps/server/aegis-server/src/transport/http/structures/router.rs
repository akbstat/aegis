//! Routes for the structures namespace.
//!
//! Each `routes!` call lists a single handler per HTTP verb. utoipa-axum
//! 0.2 panics if a single `routes!` call receives two handlers that
//! share the same HTTP method, so we split the surface into one call
//! per verb. The resulting `OpenApiRouter<AppState>` is then merged
//! into the API sub-tree in `transport::http::router`.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::structures::handlers;

/// Compose every handler under `/`; the caller (`transport::http::router`)
/// is responsible for nesting this under `/api/structures`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // ---- reads ----
        .routes(routes!(handlers::list_structures))
        .routes(routes!(handlers::get_structure))
        .routes(routes!(handlers::get_structure_meta))
        .routes(routes!(handlers::build_structure))
        .routes(routes!(handlers::list_own_nodes))
        // ---- writes ----
        .routes(routes!(handlers::create_structure))
        .routes(routes!(handlers::rename_structure))
        .routes(routes!(handlers::delete_structure))
        .routes(routes!(handlers::add_node))
        .routes(routes!(handlers::update_node))
        .routes(routes!(handlers::delete_node))
}
