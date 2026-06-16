//! Integration: the WS job channel (`BULK-AND-JOBS.md`, "WS job channel").
//!
//! Covers: a ticket presented via the `Sec-WebSocket-Protocol` subprotocol is
//! accepted and the backlog ring replays to a late subscriber; a bad ticket is
//! rejected before upgrade; a streamed query job carries chunked result frames +
//! the terminal `done` while its poll is status-only; and a dropped WS does not
//! cancel the job.

#[path = "../fixture/mod.rs"]
mod fixture;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures::StreamExt;
use http_body_util::BodyExt;
use rubix_gate::Capability;
use serde_json::{Value, json};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tower::ServiceExt;

use fixture::app::{SECRET, SUBJECT, TestJobApp, boot_jobs};
use rubix_server::jobs::JobLimits;

/// Boot the job app and serve it on an ephemeral port, returning the address and
/// the server task handle. The router is cloned for the in-process oneshot calls.
async fn serve(app: axum::Router) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    (addr, handle)
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("route responds");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, json)
}

fn authed(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-rubix-subject", SUBJECT)
        .header("x-rubix-secret", SECRET)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

/// Open a job WebSocket presenting `ticket` via the subprotocol header.
async fn connect_job(
    addr: std::net::SocketAddr,
    job_id: &str,
    ticket: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Error,
> {
    let mut request = format!("ws://{addr}/ws/jobs/{job_id}")
        .into_client_request()
        .expect("ws request");
    request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_str(&format!("rubix-job-ticket, {ticket}")).expect("subprotocol header"),
    );
    connect_async(request).await.map(|(socket, _resp)| socket)
}

/// Drain text frames from the socket until it closes, returning the parsed frames.
async fn drain_frames(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Vec<Value> {
    let mut frames = Vec::new();
    while let Ok(Some(message)) = tokio::time::timeout(Duration::from_secs(5), socket.next()).await
    {
        match message.expect("ws message") {
            Message::Text(text) => frames.push(serde_json::from_str(&text).expect("json frame")),
            Message::Close(_) => break,
            _ => continue,
        }
    }
    frames
}

#[tokio::test]
async fn a_late_subscriber_replays_the_backlog_then_sees_the_terminal_frame() {
    let TestJobApp { app, .. } = boot_jobs(
        "ws_jobs_replay",
        &[Capability::BulkSubmit],
        JobLimits::default(),
        Duration::from_secs(10),
    )
    .await;
    let (addr, server) = serve(app.clone()).await;

    let (_status, accepted) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 4 }))).await;
    let job_id = accepted["job_id"].as_str().unwrap().to_owned();
    let ticket = accepted["ticket"].as_str().unwrap().to_owned();

    // Connect after the job has had a moment to run, so the backlog ring already
    // holds frames the late subscriber must replay.
    tokio::time::sleep(Duration::from_millis(60)).await;
    let mut socket = connect_job(addr, &job_id, &ticket)
        .await
        .expect("ws connect");

    let frames = drain_frames(&mut socket).await;
    // Replay + tail delivers every step item and the terminal done, no duplicates.
    let items: Vec<&Value> = frames
        .iter()
        .filter(|f| f["type"] == json!("item"))
        .collect();
    assert_eq!(items.len(), 4, "all four step items: {frames:?}");
    assert_eq!(frames.last().unwrap()["type"], json!("done"));

    server.abort();
}

#[tokio::test]
async fn a_bad_ticket_is_rejected_before_upgrade() {
    let TestJobApp { app, .. } = boot_jobs(
        "ws_jobs_bad",
        &[Capability::BulkSubmit],
        JobLimits::default(),
        Duration::from_secs(10),
    )
    .await;
    let (addr, server) = serve(app.clone()).await;

    let (_status, accepted) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 1 }))).await;
    let job_id = accepted["job_id"].as_str().unwrap().to_owned();

    let result = connect_job(addr, &job_id, "not-the-real-ticket").await;
    assert!(result.is_err(), "a bad ticket must not upgrade");

    server.abort();
}

#[tokio::test]
async fn the_job_runs_to_completion_after_the_ws_drops() {
    let TestJobApp { app, state } = boot_jobs(
        "ws_jobs_drop",
        &[Capability::BulkSubmit],
        JobLimits::default(),
        Duration::from_secs(10),
    )
    .await;
    let (addr, server) = serve(app.clone()).await;

    let (_status, accepted) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 6 }))).await;
    let job_id = accepted["job_id"].as_str().unwrap().to_owned();
    let ticket = accepted["ticket"].as_str().unwrap().to_owned();

    // Open then immediately drop the socket — this must NOT cancel the job.
    let socket = connect_job(addr, &job_id, &ticket)
        .await
        .expect("ws connect");
    drop(socket);

    // The job still completes; poll the registry until terminal.
    let mut completed = false;
    for _ in 0..200 {
        if let Some(job) = state.jobs.get(&job_id).await
            && !job.status().is_running()
        {
            completed = true;
            assert!(matches!(
                job.status(),
                rubix_server::jobs::JobStatus::Completed
            ));
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        completed,
        "job ran to completion despite the dropped socket"
    );

    server.abort();
}

#[tokio::test]
async fn a_streamed_query_carries_chunks_over_ws_while_the_poll_is_status_only() {
    let TestJobApp { app, state } = boot_jobs(
        "ws_jobs_query",
        &[Capability::BulkSubmit, Capability::ExternalQuery],
        JobLimits::default(),
        Duration::from_secs(10),
    )
    .await;
    let (addr, server) = serve(app.clone()).await;

    // Seed a couple of records so the streamed scan returns rows.
    for v in 0..2 {
        state
            .store
            .raw()
            .query("CREATE record CONTENT { namespace: 'rubix', content: { kind: 'sample', v: $v }, created: time::now(), updated: time::now() }")
            .bind(("v", v))
            .await
            .expect("seed");
    }

    // A streamed query promotes to a Tier-2 job (202 handle).
    let (status, accepted) = send(
        &app,
        authed(
            "POST",
            "/query",
            json!({ "sql": "SELECT content FROM record", "stream": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let job_id = accepted["job_id"].as_str().unwrap().to_owned();
    let ticket = accepted["ticket"].as_str().unwrap().to_owned();

    // The poll is status-only for a streamed job: result_transport is "stream" and
    // it carries no buffered rows.
    let (s, poll) = send(
        &app,
        Request::builder()
            .method("GET")
            .uri(format!("/bulk/jobs/{job_id}"))
            .header("authorization", format!("Bearer {ticket}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(poll["result_transport"], json!("stream"));
    assert!(poll.get("result").is_none() || poll["result"].is_null());

    // The rows arrive over the WS as chunk frames, closed by a terminal done.
    let mut socket = connect_job(addr, &job_id, &ticket)
        .await
        .expect("ws connect");
    let frames = drain_frames(&mut socket).await;
    let chunks: Vec<&Value> = frames
        .iter()
        .filter(|f| f["type"] == json!("chunk"))
        .collect();
    assert!(!chunks.is_empty(), "streamed chunks arrived: {frames:?}");
    assert_eq!(frames.last().unwrap()["type"], json!("done"));

    server.abort();
}
