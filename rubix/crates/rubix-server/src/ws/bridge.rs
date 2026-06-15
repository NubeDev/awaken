//! Forward live-query data-change events onto a WebSocket as JSON frames.
//!
//! The bridge half of the realtime surface (`rubix/docs/sessions/WS-16.md`):
//! pump the WS-07 [`DataChangeStream`] and write each change to the client as a
//! text frame. The stream is already scoped by the principal's session, so this
//! loop adds no filter (contract #1). It ends when the live query is killed (the
//! session drops) or the socket closes — whichever comes first.

use axum::extract::ws::{Message, WebSocket};
use rubix_bus::{DataChange, DataChangeKind, DataChangeStream};
use rubix_gate::ScopedSession;
use serde_json::json;

/// Pump `stream` into `socket` until either ends.
///
/// `session` is held for the lifetime of the loop so its connection (the one the
/// live query runs on) is not dropped early. Each [`DataChange`] is rendered to a
/// JSON text frame; a decode or send failure ends the loop, closing the socket.
pub async fn forward_changes(
    mut socket: WebSocket,
    mut stream: DataChangeStream,
    session: ScopedSession,
) {
    while let Some(next) = stream.next().await {
        let Ok(change) = next else {
            break;
        };
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
