//! Record resource routes — CRUD over the generic record model.
//!
//! Mutations (create/update/delete) cross the WS-05 gate; reads (get/list) run on
//! the WS-03 scoped session (`rubix/docs/sessions/WS-16.md`, contract #1). One
//! file per route; this barrel only merges them into a router.

pub(crate) mod capability;
pub(crate) mod create;
mod delete;
mod get;
mod list;
mod update;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

use create::create_record_route;
use delete::delete_record_route;
use get::get_record_route;
use list::list_records_route;
use update::update_record_route;

/// The record routes mounted under `/records`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/records",
            post(create_record_route).get(list_records_route),
        )
        .route(
            "/records/:id",
            get(get_record_route)
                .patch(update_record_route)
                .delete(delete_record_route),
        )
}
