//! Agent route — wiring only. One chat turn of the embedded BMS agent.

pub(crate) mod chat;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/v1/agent/chat", post(chat::chat))
}
