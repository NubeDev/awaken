//! Agent routes — wiring only. One chat turn of the embedded BMS agent, plus a
//! read-only view of the agent's (process-global, env-configured) status.

pub(crate) mod chat;
pub(crate) mod status;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/agent/chat", post(chat::chat))
        .route("/api/v1/agent/status", get(status::status))
}
