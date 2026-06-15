//! Integration: a WebSocket client receives a live-query event on record insert.
//!
//! The WS-16 Done definition (`rubix/docs/sessions/WS-16.md`): open the WS bridge
//! on a scoped session, insert a record, and assert the client receives the
//! live-query event. The bridge runs on the principal's scoped session, so the
//! event is filtered by SurrealDB row-level permissions (contract #1).

#[path = "../fixture/mod.rs"]
mod fixture;

use std::time::Duration;

use futures::StreamExt;
use rubix_core::Id;
use rubix_gate::{Capability, Change, Command, apply};
use serde_json::Value;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use fixture::app::{NS, SECRET, SUBJECT, boot};

#[tokio::test]
async fn client_receives_a_live_query_event_on_insert() {
    let app = boot("server_ws", &[Capability::IngestPublish]).await;
    let store = app.store.clone();

    // Bind an ephemeral port and serve the transport in the background.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app.app).await.expect("serve");
    });

    // Connect a WebSocket client carrying the principal credentials.
    let mut request = format!("ws://{addr}/ws/records")
        .into_client_request()
        .expect("ws request");
    request
        .headers_mut()
        .insert("x-rubix-subject", SUBJECT.parse().expect("subject header"));
    request
        .headers_mut()
        .insert("x-rubix-secret", SECRET.parse().expect("secret header"));
    let (mut socket, _response) = connect_async(request).await.expect("ws connect");

    // Give the live query a moment to register before the insert.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Insert a record through the gate (the same path the HTTP create route uses).
    let id = Id::new();
    let principal = rubix_gate::authenticate(
        store.raw(),
        &rubix_gate::PrincipalToken::new(SUBJECT, SECRET),
    )
    .await
    .expect("authenticate");
    let command = Command::new(
        principal,
        Capability::IngestPublish,
        id.clone(),
        Change::Create(serde_json::json!({ "temp": 42.0 })),
    );
    apply(store.raw(), &command, None)
        .await
        .expect("gate create");

    // The client should receive the created event for the inserted record.
    let frame = tokio::time::timeout(Duration::from_secs(5), next_text(&mut socket))
        .await
        .expect("event arrives in time");
    let event: Value = serde_json::from_str(&frame).expect("json frame");
    assert_eq!(event["kind"], "created");
    assert_eq!(event["record"]["id"], serde_json::json!(id.to_string()));
    assert_eq!(event["record"]["namespace"], serde_json::json!(NS));
    assert_eq!(event["record"]["content"]["temp"], serde_json::json!(42.0));

    socket.close(None).await.ok();
    server.abort();
}

/// Pull text frames until the first one arrives, skipping pings/pongs.
async fn next_text(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> String {
    while let Some(message) = socket.next().await {
        match message.expect("ws message") {
            Message::Text(text) => return text,
            Message::Close(_) => panic!("socket closed before an event arrived"),
            _ => continue,
        }
    }
    panic!("stream ended before an event arrived");
}
