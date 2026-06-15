//! `GET /health` — report that the process and its store are live.
//!
//! The liveness probe every later route hangs off (`rubix/docs/sessions/
//! WS-16.md`). Fails closed: a dead store reports `503`, never a healthy `200`.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::state::AppState;

/// Probe the store and report liveness.
///
/// Returns `200` with `{"status":"ok"}` when the store answers, `503` with
/// `{"status":"unavailable"}` when it does not.
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.store.health().await {
        Ok(()) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable" })),
        ),
    }
}
