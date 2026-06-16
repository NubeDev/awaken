//! Integration: the `/extensions*` admin surface reads the runtime and drives the
//! lifecycle through the gate (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase 4;
//! `rubix/docs/design/ADMIN-API.md`).
//!
//! Uses a **builtin**-flavour control record so the lifecycle mutation exercises
//! the full gate path without spawning a real child (the process-spawn path is
//! covered by `rubix-ext`'s own supervisor tests). Asserts: the reads project the
//! control record + degraded gauges per-namespace; an out-of-grant `start` is a
//! `403` before any effect; a granted `start` crosses the gate, is counted, and
//! writes the lifecycle field; unknown ids and bad actions are `404`/`400`.

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_core::Record;
use rubix_gate::Capability;
use rubix_store::StoreHandle;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{NS, SECRET, SUBJECT, TestApp, boot};

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("route responds");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
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

/// Seed a builtin-flavour extension control record visible to the test session.
async fn seed_control_record(store: &StoreHandle, subject: &str, lifecycle: &str) {
    let record = Record::new(
        NS,
        json!({
            "extension": subject,
            "runtime": { "bin": "/dev/null", "flavour": "builtin" },
            "lifecycle": lifecycle,
        }),
    );
    rubix_core::create_record(store.raw(), &record)
        .await
        .expect("seed control record");
}

#[tokio::test]
async fn the_surface_reads_and_drives_a_builtin_extension() {
    let TestApp { app, store } = boot("ext_admin", &[Capability::ExtensionManage]).await;
    seed_control_record(&store, "obs-ext", "stop").await;

    // ---- list ----
    let (status, body) = send(&app, authed("GET", "/extensions", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().expect("array");
    let row = rows
        .iter()
        .find(|r| r["id"] == "obs-ext")
        .expect("obs-ext listed");
    assert_eq!(row["flavour"], "builtin");
    assert_eq!(row["state"], "stopped");
    assert_eq!(row["lifecycle"], "stop");

    // ---- detail ----
    let (status, body) = send(&app, authed("GET", "/extensions/obs-ext", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "obs-ext");
    assert_eq!(body["metrics"]["lifecycle_state"], "stopped");
    assert_eq!(body["content"]["extension"], "obs-ext");

    // ---- process: builtin reports not-running ----
    let (status, body) = send(&app, authed("GET", "/extensions/obs-ext/process", Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "ext.process.not_running");

    // ---- metrics: meaningful doc, zero counters ----
    let (status, body) = send(&app, authed("GET", "/extensions/obs-ext/metrics", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["lifecycle_state"], "stopped");
    assert_eq!(body["commands_total"], 0);

    // ---- events: empty ring ----
    let (status, body) = send(&app, authed("GET", "/extensions/obs-ext/events", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["events"].as_array().unwrap().len(), 0);

    // ---- unknown id ----
    let (status, _) = send(&app, authed("GET", "/extensions/ghost", Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // ---- lifecycle: granted start crosses the gate (builtin → no child) ----
    let (status, body) = send(
        &app,
        authed("POST", "/extensions/obs-ext/lifecycle", json!({ "action": "start" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "granted start succeeds: {body}");
    assert_eq!(body["action"], "start");
    assert!(body["state"].is_null(), "builtin start has no live handle");
    assert!(
        body["correlation_id"].as_str().is_some_and(|c| !c.is_empty()),
        "gate stamped a correlation id"
    );

    // The command was counted, and the gated record now reads `start`.
    let (_, metrics) = send(&app, authed("GET", "/extensions/obs-ext/metrics", Value::Null)).await;
    assert_eq!(metrics["commands_total"], 1);
    let (_, detail) = send(&app, authed("GET", "/extensions/obs-ext", Value::Null)).await;
    assert_eq!(detail["content"]["lifecycle"], "start");

    // ---- bad action ----
    let (status, _) = send(
        &app,
        authed("POST", "/extensions/obs-ext/lifecycle", json!({ "action": "teleport" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_out_of_grant_lifecycle_is_forbidden() {
    // The principal holds no extension-manage grant.
    let TestApp { app, store } = boot("ext_admin_deny", &[]).await;
    seed_control_record(&store, "locked-ext", "stop").await;

    let (status, _) = send(
        &app,
        authed("POST", "/extensions/locked-ext/lifecycle", json!({ "action": "start" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // And the gated record was not flipped.
    let (_, detail) = send(&app, authed("GET", "/extensions/locked-ext", Value::Null)).await;
    assert_eq!(detail["content"]["lifecycle"], "stop");
}
