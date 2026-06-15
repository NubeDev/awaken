//! WebSocket live-query bridge.
//!
//! The realtime surface (`rubix/docs/SCOPE.md`, "Realtime"): a client subscribes
//! over a WebSocket and receives the WS-07 data-change feed, filtered by its
//! principal's scoped session (contract #1). [`subscribe`] opens the subscription
//! on the scoped session; [`bridge`] forwards each change as a JSON frame.

mod bridge;
mod subscribe;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

use subscribe::subscribe_records_route;

/// The WebSocket route mounted at `/ws/records`.
pub fn router() -> Router<AppState> {
    Router::new().route("/ws/records", get(subscribe_records_route))
}
