//! Point routes — wiring only.

pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod get;
pub(crate) mod list;
pub(crate) mod patch;
pub(crate) mod response;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/points",
            post(create::create_point).get(list::list_points),
        )
        .route(
            "/api/v1/points/{id}",
            get(get::get_point)
                .patch(patch::patch_point)
                .delete(delete::delete_point),
        )
}
