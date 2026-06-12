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
    let sub = client
        .declare_subscriber("bus2/s2/ahu-3/fan/cur")
        .await
        .expect("subscribe");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let replies = client
        .get("bus2/s2/ahu-3/fan/write")
        .payload(serde_json::to_vec(&json!({ "value": true, "priority": 8 })).unwrap())
        .await
        .expect("get write");

    let point: Value = first_ok(replies).await;
    assert_eq!(point["cur_value"], json!(true));

    // A bus-driven write republishes the effective value on `cur`.
    let sample = tokio::time::timeout(Duration::from_secs(3), sub.recv_async())
        .await
        .expect("cur sample within timeout")
        .expect("sample");
    let cur: Value =
        serde_json::from_slice(&sample.payload().to_bytes()).expect("decode cur payload");
    assert_eq!(cur, json!(true));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_query_for_unowned_site_gets_no_reply() {
    // The node owns no `ghost/x` site, so its wildcard `**/write` queryable must
    // stay silent rather than answer "not found" — that's what lets the owning
    // node be the sole responder in a multi-node mesh.
    let (_app, _bus) = TestApp::with_bus().await;
    let client = client_session().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let replies = client
        .get("ghost/x/ahu-1/fan/write")
        .payload(serde_json::to_vec(&json!({ "value": true })).unwrap())
        .await
        .expect("get write");

    // No owned site covers the key → no successful reply. Either the query
    // window closes the channel (Ok(Err)) or it times out (outer Err); a
    // delivered reply (Ok(Ok)) would be the failure.
    let got = tokio::time::timeout(Duration::from_millis(800), replies.recv_async()).await;
    let delivered = matches!(got, Ok(Ok(_)));
    assert!(
        !delivered,
        "expected no reply for unowned site, got {got:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spark_create_publishes_on_zenoh() {
    let (app, _bus) = TestApp::with_bus().await;
    let site = app.create_site_with("bus5", "s5").await;

    let client = client_session().await;
    let sub = client
        .declare_subscriber("bus5/s5/spark/**")
        .await
        .expect("subscribe");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/sparks",
            Some(json!({
                "site_id": site,
                "rule": "simultaneous-heat-cool",
                "severity": "warning",
                "message": "AHU-3 heating and cooling at once",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let sample = tokio::time::timeout(Duration::from_secs(3), sub.recv_async())
        .await
        .expect("spark within timeout")
        .expect("sample");
    // The finding lands on `{org}/{site}/spark/{rule}/{id}`.
    let key = sample.key_expr().as_str();
    assert!(
        key.starts_with("bus5/s5/spark/simultaneous-heat-cool/"),
        "unexpected key {key}"
    );
    let published: Value =
        serde_json::from_slice(&sample.payload().to_bytes()).expect("decode spark payload");
    assert_eq!(published["rule"], json!("simultaneous-heat-cool"));
    assert_eq!(published["id"], body["id"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn board_emit_spark_publishes_on_zenoh() {
    let (app, _bus) = TestApp::with_bus().await;
    let _site = app.create_site_with("bus6", "s6").await;

    let client = client_session().await;
    let sub = client
        .declare_subscriber("bus6/s6/spark/**")
        .await
        .expect("subscribe");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Store a board whose single node emits a finding, then run it by slug.
    let board = json!({
        "slug": "conflict-rule", "display_name": "Conflict rule",
        "trigger": {"kind": "manual"},
        "board": {
            "nodes": [{
                "id": "s1", "component": "emit_spark",
                "config": {
                    "site": "bus6/s6", "rule": "heat-cool-conflict",
                    "severity": "fault", "message": "AHU-3 heat/cool conflict"
                }
            }],
            "connections": []
        }
    });
    let (status, _) = app.request("POST", "/api/v1/boards", Some(board)).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = app
        .request("POST", "/api/v1/boards/conflict-rule/run", None)
        .await;
    assert_eq!(status, StatusCode::OK);

    // The board-emitted finding lands on the same `spark` keyexpr scheme as
    // an HTTP-created one.
    let sample = tokio::time::timeout(Duration::from_secs(3), sub.recv_async())
        .await
        .expect("spark within timeout")
        .expect("sample");
    let key = sample.key_expr().as_str();
    assert!(
        key.starts_with("bus6/s6/spark/heat-cool-conflict/"),
        "unexpected key {key}"
    );
    let published: Value =
        serde_json::from_slice(&sample.payload().to_bytes()).expect("decode spark payload");
    assert_eq!(published["rule"], json!("heat-cool-conflict"));
    assert_eq!(published["severity"], json!("fault"));
    assert_eq!(published["message"], json!("AHU-3 heat/cool conflict"));
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
