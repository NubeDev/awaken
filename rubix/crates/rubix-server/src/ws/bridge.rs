//! Forward live-query data-change events onto a WebSocket as JSON frames.
//!
//! The bridge half of the realtime surface (`rubix/docs/sessions/WS-16.md`):
//! pump the WS-07 [`DataChangeStream`] and write each change to the client as a
//! text frame. The stream is already scoped by the principal's session
//! (contract #1); an optional `kind` filter narrows it further to one collection
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "List/realtime filtering by
//! collection") so a single-collection grid wakes only on its own records. The
//! filter only drops events the client did not ask for — it never surfaces a
//! record the session could not already read. The loop ends when the live query
//! is killed (the session drops) or the socket closes — whichever comes first.

use axum::extract::ws::{Message, WebSocket};
use rubix_bus::{DataChange, DataChangeKind, DataChangeStream};
use rubix_gate::ScopedSession;
use serde_json::json;

/// Pump `stream` into `socket` until either ends, forwarding only changes that
/// match `kind_filter` (when set).
///
/// `session` is held for the lifetime of the loop so its connection (the one the
/// live query runs on) is not dropped early. Each forwarded [`DataChange`] is
/// rendered to a JSON text frame; a decode or send failure ends the loop, closing
/// the socket. When `kind_filter` is `Some`, a change whose record's
/// `content.kind` differs is skipped rather than sent.
pub async fn forward_changes(
    mut socket: WebSocket,
    mut stream: DataChangeStream,
    session: ScopedSession,
    kind_filter: Option<String>,
) {
    while let Some(next) = stream.next().await {
        let Ok(change) = next else {
            break;
        };
        if !matches_kind(&change, kind_filter.as_deref()) {
            continue;
        }
        let frame = match serde_json::to_string(&change_frame(&change)) {
            Ok(frame) => frame,
            Err(_) => break,
        };
        if socket.send(Message::Text(frame)).await.is_err() {
            break;
        }
    }
    drop(session);
    let _ = socket.send(Message::Close(None)).await;
}

/// Whether `change` passes the optional collection `kind` filter.
///
/// No filter passes everything; a set filter passes only a record whose
/// `content.kind` string equals `kind`.
fn matches_kind(change: &DataChange, kind: Option<&str>) -> bool {
    match kind {
        None => true,
        Some(kind) => change
            .record()
            .content
            .get("kind")
            .and_then(serde_json::Value::as_str)
            == Some(kind),
    }
}

/// Render a [`DataChange`] into the wire frame: the change kind plus the record's
/// id, namespace, and content.
fn change_frame(change: &DataChange) -> serde_json::Value {
    let record = change.record();
    json!({
        "kind": kind_str(change.kind()),
        "record": {
            "id": record.id.to_string(),
            "namespace": record.namespace,
            "content": record.content,
        },
    })
}

/// The stable wire string for a change kind.
fn kind_str(kind: DataChangeKind) -> &'static str {
    match kind {
        DataChangeKind::Created => "created",
        DataChangeKind::Updated => "updated",
        DataChangeKind::Deleted => "deleted",
    }
}
