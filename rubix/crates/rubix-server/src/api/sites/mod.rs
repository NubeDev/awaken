//! Site routes — wiring only.

pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod get;
pub(crate) mod list;
pub(crate) mod patch;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/sites",
            post(create::create_site).get(list::list_sites),
        )
        .route(
            "/api/v1/sites/{id}",
            get(get::get_site)
                .patch(patch::patch_site)
                .delete(delete::delete_site),
        )
}
