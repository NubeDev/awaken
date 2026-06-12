//! Pinned dashboard widget routes — wiring only.

pub(crate) mod create;
pub(crate) mod list;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/widgets",
        post(create::create_widget).get(list::list_widgets),
    )
}
