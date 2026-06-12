//! Zenoh data-plane integration: a separate peer session subscribes to `cur`
//! and queries the `write`/`his` queryables served by the server's bus.

use std::time::Duration;

use axum::http::StatusCode;
use serde_json::{json, Value};

use super::harness::TestApp;

/// Open a second peer session standing in for a dashboard / external agent.
async fn client_session() -> zenoh::Session {
    zenoh::open(zenoh::Config::default())
        .await
        .expect("client session")
}

/// Drain query replies until one carries a successful payload, decoding it as
/// `T`. Other in-process bus instances answer the same wildcard queryable with
/// "not found" errors for points they don't own; skip those.
async fn first_ok<T: serde::de::DeserializeOwned>(
    replies: zenoh::handlers::FifoChannelHandler<zenoh::query::Reply>,
) -> T {
    loop {
        let reply = tokio::time::timeout(Duration::from_secs(3), replies.recv_async())
            .await
            .expect("reply within timeout")
            .expect("reply");
        if let Ok(sample) = reply.result() {
            return serde_json::from_slice(&sample.payload().to_bytes()).expect("decode reply");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cur_ingest_publishes_on_zenoh() {
    let (app, _bus) = TestApp::with_bus().await;
    let site = app.create_site_with("bus1", "s1").await;
    let equip = app.create_equip(&site).await;
    let point = app.create_point(&equip, "sensor", "temp").await;

    let client = client_session().await;
    let sub = client
        .declare_subscriber("bus1/s1/ahu-3/temp/cur")
        .await
        .expect("subscribe");
    // Let scouting connect the two peers before publishing.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{point}/cur"),
            Some(json!({ "value": 21.5 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let sample = tokio::time::timeout(Duration::from_secs(3), sub.recv_async())
        .await
        .expect("cur sample within timeout")
        .expect("sample");
    let value: Value =
        serde_json::from_slice(&sample.payload().to_bytes()).expect("decode cur payload");
    assert_eq!(value, json!(21.5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_queryable_commands_priority_array() {
    let (app, _bus) = TestApp::with_bus().await;
    let site = app.create_site_with("bus2", "s2").await;
    let equip = app.create_equip(&site).await;
    let _point = app.create_point(&equip, "cmd", "fan").await;

    let client = client_session().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let replies = client
        .get("bus2/s2/ahu-3/fan/write")
        .payload(serde_json::to_vec(&json!({ "value": true, "priority": 8 })).unwrap())
        .await
        .expect("get write");

    let point: Value = first_ok(replies).await;
    assert_eq!(point["cur_value"], json!(true));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn his_queryable_serves_history() {
    let (app, _bus) = TestApp::with_bus().await;
    let site = app.create_site_with("bus3", "s3").await;
    let equip = app.create_equip(&site).await;
    let point = app.create_point(&equip, "sensor", "temp").await;

    for v in [20.0, 21.0, 22.0] {
        let (status, _) = app
            .request(
                "POST",
                &format!("/api/v1/points/{point}/cur"),
                Some(json!({ "value": v })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }

    let client = client_session().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let replies = client
        .get("bus3/s3/ahu-3/temp/his/**")
        .await
        .expect("get his");
    let samples: Vec<Value> = first_ok(replies).await;
    assert_eq!(samples.len(), 3);
}
