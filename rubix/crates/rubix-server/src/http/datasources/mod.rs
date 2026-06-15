//! Datasource registry routes.
//!
//! `GET /datasources` lists the declared datasources (`rubix/docs/sessions/
//! WS-16.md`). Registration (`POST /datasources`) is deferred with the WS-13
//! extension control plane (see TODOs.md). This barrel merges the read route.

mod list;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

use list::list_datasources_route;

/// The datasource routes mounted at `/datasources`.
pub fn router() -> Router<AppState> {
    Router::new().route("/datasources", get(list_datasources_route))
}
