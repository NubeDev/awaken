//! Boards route — wiring only. Evaluate reflow control/rule boards over the
//! store.

pub(crate) mod components;
pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod dto;
pub(crate) mod get;
pub(crate) mod list;
pub(crate) mod outputs;
pub(crate) mod patch;
pub(crate) mod run;
pub(crate) mod run_stored;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/boards/run", post(run::run_board))
        .route(
            "/api/v1/boards/components",
            axum::routing::get(components::list_components),
        )
        .route(
            "/api/v1/boards",
            post(create::create_board).get(list::list_boards),
        )
        .route(
            "/api/v1/boards/{slug}",
            axum::routing::get(get::get_board)
                .patch(patch::patch_board)
                .delete(delete::delete_board),
        )
        .route(
            "/api/v1/boards/{slug}/run",
            post(run_stored::run_stored_board),
        )
        .route(
            "/api/v1/boards/{slug}/outputs",
            axum::routing::get(outputs::board_outputs),
        )
}
