//! Reading resource routes — the time-series data plane over the wire.
//!
//! Unlike `records`, readings do **not** cross the command gate: the bulk append
//! is a data-plane write authorized once per request by the `readings-append`
//! capability and applied on the owner handle (`rubix/docs/design/READINGS-TIMESERIES.md`).
//! The windowed read runs on the WS-03 scoped session like every other read. One
//! file per route; this barrel only merges them into a router.

mod append;
mod window;

use axum::Router;
use axum::routing::post;

use crate::state::AppState;

use append::append_readings_route;
use window::read_readings_route;

/// The reading routes mounted under `/readings`.
pub fn router() -> Router<AppState> {
    Router::new().route(
        "/readings",
        post(append_readings_route).get(read_readings_route),
    )
}
