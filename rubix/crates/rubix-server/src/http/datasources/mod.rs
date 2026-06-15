//! Datasource registry control-plane routes.
//!
//! The Grafana "add a datasource" surface (`rubix/docs/SCOPE.md`, "Datasources";
//! `rubix/docs/sessions/WS-16.md`): full CRUD over the declared datasources. Reads
//! (`GET` list + one) are open over the shared registry; writes (`POST`/`PATCH`/
//! `DELETE`) cross the WS-04 `datasource-register` capability, checked fail-closed
//! in `rubix-datasource`. This barrel merges the routes into one router.

pub(crate) mod create;
mod delete;
mod get;
mod list;
mod update;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

use create::create_datasource_route;
use delete::delete_datasource_route;
use get::get_datasource_route;
use list::list_datasources_route;
use update::update_datasource_route;

/// The datasource routes mounted under `/datasources`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/datasources",
            post(create_datasource_route).get(list_datasources_route),
        )
        .route(
            "/datasources/:id",
            get(get_datasource_route)
                .patch(update_datasource_route)
                .delete(delete_datasource_route),
        )
}
