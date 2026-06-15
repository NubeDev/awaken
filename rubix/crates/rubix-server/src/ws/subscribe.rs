//! `GET /ws/records` — open a live-query subscription over a WebSocket.
//!
//! The realtime surface (`rubix/docs/SCOPE.md`, "Realtime"; contract #1): a
//! client upgrades to a WebSocket and receives the WS-07 data-change feed for the
//! `record` table, filtered by its principal's scoped session. The scope is set
//! once here at subscribe — SurrealDB row-level permissions decide which records
//! produce events; there is no per-message app proxy. The principal is resolved
//! from the same credential headers as the HTTP routes before the upgrade, so an
//! unauthenticated client never opens a stream.

use axum::extract::State;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::Response;
use rubix_bus::subscribe_table;

use crate::auth::Authenticated;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::ws::bridge::forward_changes;

/// The table the live-query bridge subscribes to.
const RECORD_TABLE: &str = "record";

/// Upgrade the connection and stream record changes visible to the principal.
///
/// The subscription opens on the principal's scoped session connection, so the
/// engine filters the live query at the source. If the engine rejects the live
/// query, the upgrade fails before any frame is sent.
pub async fn subscribe_records_route(
    State(_state): State<AppState>,
    auth: Authenticated,
    upgrade: WebSocketUpgrade,
) -> ApiResult<Response> {
    let stream = subscribe_table(auth.session.connection(), RECORD_TABLE)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    // The scoped session is moved into the upgrade callback so its connection
    // outlives the live query — dropping it would kill the stream.
    Ok(upgrade.on_upgrade(move |socket| forward_changes(socket, stream, auth.session)))
}
