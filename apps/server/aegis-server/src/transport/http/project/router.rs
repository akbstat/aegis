//! Project HTTP routes.
//!
//! Mounted at `/api/project` by the top-level router.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::project::handlers;

/// Build the resource router that backs `/api/project`. Each handler
/// is registered in its own `routes!(...)` call because utoipa-axum
/// 0.2 panics on multiple same-method handlers in a single call.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create_project))
        .routes(routes!(handlers::list_projects))
        .routes(routes!(handlers::get_project_by_code))
        .routes(routes!(handlers::update_project))
        .routes(routes!(handlers::create_project_branch))
        .routes(routes!(handlers::list_project_branches))
        .routes(routes!(handlers::get_project_branch_by_id))
        .routes(routes!(handlers::update_project_branch))
}
