//! `GET /health` — report that the process and its store are live.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::state::AppState;

/// Probe the store and report liveness.
///
/// Returns `200` with `{"status":"ok"}` when the store answers, `503` with
/// `{"status":"unavailable"}` when it does not — failing closed rather than
/// reporting healthy on a dead store.
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.store.health().await {
        Ok(()) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable" })),
        ),
    }
}
