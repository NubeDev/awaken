//! Equip routes — wiring only.

pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod get;
pub(crate) mod list;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/equips",
            post(create::create_equip).get(list::list_equips),
        )
        .route(
            "/api/v1/equips/{id}",
            get(get::get_equip).delete(delete::delete_equip),
        )
}
