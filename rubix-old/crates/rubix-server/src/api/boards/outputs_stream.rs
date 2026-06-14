//! GET /api/v1/boards/{slug}/outputs/stream?org=&site_id= — a Server-Sent
//! Events stream of a board's live per-node output snapshots. Emits the current
//! snapshot on connect, then a fresh snapshot every time the board runs (a
//! scheduled scan, a subscription sample, or an on-demand run), so a client sees
//! values in real time without the 5s polling of `GET …/outputs`.
//!
//! Same scope authorization as the REST snapshot endpoint: board link values can
//! carry a tenant's point values, so the stream is tenant-scoped data, not
//! public. Each SSE `data:` frame is the JSON array of [`PortOutput`].

use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{self, Stream, StreamExt};
use tokio::sync::broadcast::error::RecvError;

use super::dto::BoardScope;
use crate::api::scope_auth::may_read_board;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::scheduler::PortOutput;
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/boards/{slug}/outputs/stream", tag = "boards",
    params(("slug" = String, Path, description = "Board slug"), BoardScope),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "SSE stream; each event is a JSON array of PortOutput"),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody)))]
pub(crate) async fn board_outputs_stream(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(slug): Path<String>,
    Query(scope): Query<BoardScope>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    if !may_read_board(&principal, &state.store, &scope.org, scope.site_id, &slug) {
        return Err(ApiError::Forbidden(format!(
            "subject may not read flows in org `{}`",
            scope.org
        )));
    }

    // Seed with the current snapshot, then follow the board's broadcast. With no
    // scheduler there is nothing to follow — emit the (empty) snapshot and end.
    let (snapshot, rx) = match &state.scheduler {
        Some(scheduler) => {
            let (snap, rx) = scheduler.outputs().subscribe(&slug);
            (snap, Some(rx))
        }
        None => (Vec::new(), None),
    };

    let initial = stream::once(async move { Ok(snapshot_event(&snapshot)) });
    let updates = stream::unfold(rx, |rx| async move {
        let mut rx = rx?;
        loop {
            match rx.recv().await {
                Ok(snapshot) => return Some((Ok(snapshot_event(&snapshot)), Some(rx))),
                // A slow client fell behind; skip to the next available frame.
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return None,
            }
        }
    });

    Ok(Sse::new(initial.chain(updates)).keep_alive(KeepAlive::default()))
}

/// One SSE frame: the snapshot as a JSON array of [`PortOutput`].
fn snapshot_event(snapshot: &[PortOutput]) -> Event {
    let data = serde_json::to_string(snapshot).unwrap_or_else(|_| "[]".to_string());
    Event::default().data(data)
}
