//! `GET /ws/jobs/{id}` — observe a job's progress + results over a WebSocket
//! (`BULK-AND-JOBS.md`, "WS job channel").
//!
//! The job-observation half of the WS plane: upgrade → resolve the ticket → replay
//! the job's backlog ring (so a late or reconnecting subscriber resumes without
//! re-running the work) → tail its broadcast until a terminal frame, then close.
//! This reuses the `bridge.rs` forward pattern but **not** the live-query data
//! plane — frames come from the in-memory registry, not a SurrealDB live query.
//!
//! The ticket is presented via the **`Sec-WebSocket-Protocol` subprotocol**
//! (`rubix-job-ticket, <ticket>`), never a `?ticket=` query string: the browser
//! `WebSocket` API cannot set custom headers, so the subprotocol is the only
//! header-grade, browser-compatible channel, and (unlike a query string) it does
//! not land in access logs or history. Frame auth is the cheap ticket resolve at
//! upgrade — no capability re-check per frame.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use tokio::sync::broadcast::error::RecvError;

use crate::error::{ApiError, ApiResult};
use crate::jobs::{Job, JobFrame, JobSubscription, resolve_observer};
use crate::state::AppState;

/// The subprotocol marker that flags the second token as the job ticket.
const TICKET_SUBPROTOCOL: &str = "rubix-job-ticket";

/// Upgrade the connection and stream the job's frames to the ticket holder.
///
/// A missing/invalid/expired ticket is rejected **before** the upgrade (`401`); a
/// job the registry no longer holds is `404`. On success the server echoes the
/// `rubix-job-ticket` subprotocol (required for a browser to accept the socket) and
/// forwards frames until the job is terminal.
pub async fn subscribe_job_route(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> ApiResult<Response> {
    let ticket = ticket_from_subprotocol(&headers).ok_or_else(|| {
        ApiError::Unauthenticated("missing job ticket in Sec-WebSocket-Protocol".to_owned())
    })?;
    let job = resolve_observer(&state, &job_id, &ticket).await?;

    // Echo the marker subprotocol back; the browser rejects a socket whose server
    // did not select one of the offered subprotocols.
    Ok(upgrade
        .protocols([TICKET_SUBPROTOCOL])
        .on_upgrade(move |socket| forward_job(socket, job)))
}

/// Extract the ticket from the `Sec-WebSocket-Protocol` header.
///
/// The client offers two subprotocol tokens — the marker `rubix-job-ticket` and
/// the ticket value. Returns the ticket when the marker is present, else `None`
/// (so a socket without the marker is rejected before upgrade).
fn ticket_from_subprotocol(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("sec-websocket-protocol")?.to_str().ok()?;
    let tokens: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    if !tokens.contains(&TICKET_SUBPROTOCOL) {
        return None;
    }
    tokens
        .into_iter()
        .find(|token| *token != TICKET_SUBPROTOCOL)
        .map(str::to_owned)
}

/// Replay the job's backlog ring then tail its broadcast until terminal.
///
/// The subscription snapshot (backlog + status + live receiver) is taken
/// atomically, so the replay-then-tail consumer sees each frame exactly once. If
/// the job was already terminal at snapshot, the terminal frame is in the replayed
/// backlog and the socket closes immediately; otherwise it tails the broadcast
/// until a `Done`/`Failed` frame arrives. A `Lagged` receiver (the consumer fell
/// behind the broadcast buffer) keeps tailing — a reconnect replays the ring.
///
/// A dropped socket ends this loop but never touches the job's cancel token, so the
/// job runs to completion regardless of who is watching.
async fn forward_job(mut socket: WebSocket, job: Arc<Job>) {
    let JobSubscription {
        backlog,
        status,
        mut receiver,
    } = job.subscribe();

    for frame in &backlog {
        if !send_frame(&mut socket, frame).await {
            return;
        }
    }
    if !status.is_running() {
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    loop {
        match receiver.recv().await {
            Ok(frame) => {
                let terminal = frame.is_terminal();
                if !send_frame(&mut socket, &frame).await {
                    return;
                }
                if terminal {
                    break;
                }
            }
            Err(RecvError::Closed) => break,
            Err(RecvError::Lagged(_)) => continue,
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

/// Serialise a frame to a JSON text frame and send it, returning whether the send
/// succeeded (a serialise or socket error ends the stream).
async fn send_frame(socket: &mut WebSocket, frame: &JobFrame) -> bool {
    match serde_json::to_string(frame) {
        Ok(text) => socket.send(Message::Text(text)).await.is_ok(),
        Err(_) => false,
    }
}
