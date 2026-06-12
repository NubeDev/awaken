//! Boards route — wiring only. Evaluate reflow control/rule boards over the
//! store.

pub(crate) mod run;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/v1/boards/run", post(run::run_board))
}
