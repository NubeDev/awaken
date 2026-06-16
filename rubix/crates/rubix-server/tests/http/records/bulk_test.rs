//! Integration: bulk record CRUD over HTTP (`BULK-AND-JOBS.md`, "Bulk record CRUD").
//!
//! Covers the design's bulk test surface: per-item isolation (one item fails, the
//! rest commit, envelope stays `200`); `bulk-submit` without the per-item write cap
//! yields all-items-forbidden; and deadline promotion returns `202` with the
//! already-committed statuses plus a ticket, the WS/poll carrying the rest.

#[path = "../../fixture/mod.rs"]
mod fixture;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_gate::Capability;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{NS, SECRET, SUBJECT, TestJobApp, boot_jobs};
use rubix_server::AppState;
use rubix_server::jobs::JobLimits;

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

fn ticketed_get(uri: &str, ticket: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {ticket}"))
        .body(Body::empty())
        .expect("build request")
}

/// Seed a raw record directly on the store (bypassing the gate) — used to enable
/// strict mode and declare a collection so a per-item validation failure is
/// deterministic without depending on a duplicate-id store error.
async fn seed_record(state: &AppState, content: Value) {
    state
        .store
        .raw()
        .query("CREATE record CONTENT { namespace: $ns, content: $content, created: time::now(), updated: time::now() }")
        .bind(("ns", NS.to_owned()))
        .bind(("content", content))
        .await
        .expect("seed record");
}

#[tokio::test]
async fn one_item_fails_while_the_rest_commit() {
    let TestJobApp { app, state } = boot_jobs(
        "bulk_isolation",
        &[Capability::BulkSubmit, Capability::IngestPublish],
        JobLimits::default(),
        Duration::from_secs(30),
    )
    .await;

    // Strict mode + one empty-schema collection: a `widget` validates, an unknown
    // kind is rejected — a deterministic per-item failure.
    seed_record(
        &state,
        json!({ "kind": "_namespace_settings", "strict": true }),
    )
    .await;
    seed_record(
        &state,
        json!({ "kind": "collection", "name": "widget", "schema": [] }),
    )
    .await;

    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/records/bulk",
            json!({ "items": [
                { "key": "good", "op": "create", "body": { "kind": "widget", "v": 1 } },
                { "key": "bad",  "op": "create", "body": { "kind": "mystery" } }
            ] }),
        ),
    )
    .await;

    // The envelope is 200 even though one item failed (per-item isolation).
    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().expect("items");
    assert_eq!(items.len(), 2);
    let good = items.iter().find(|i| i["key"] == json!("good")).unwrap();
    let bad = items.iter().find(|i| i["key"] == json!("bad")).unwrap();
    assert_eq!(good["status"], json!("created"));
    assert!(good["id"].as_str().is_some());
    assert_eq!(bad["status"], json!("failed"));
    assert!(bad["error"].as_str().unwrap().contains("strict"));
}

#[tokio::test]
async fn bulk_submit_without_the_per_item_cap_forbids_every_item() {
    // The principal may open a bulk job (bulk-submit) but holds no record-write cap,
    // so every item is denied at its own apply() — yet the envelope is still 200.
    let TestJobApp { app, .. } = boot_jobs(
        "bulk_no_item_cap",
        &[Capability::BulkSubmit],
        JobLimits::default(),
        Duration::from_secs(30),
    )
    .await;

    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/records/bulk",
            json!({ "items": [
                { "key": "a", "op": "create", "body": { "v": 1 } },
                { "key": "b", "op": "create", "body": { "v": 2 } }
            ] }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().expect("items");
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i["status"] == json!("failed")));
    assert!(
        items
            .iter()
            .all(|i| i["error"].as_str().unwrap().contains("forbidden"))
    );
}

#[tokio::test]
async fn without_bulk_submit_the_envelope_is_forbidden() {
    let TestJobApp { app, .. } = boot_jobs(
        "bulk_no_submit",
        &[Capability::IngestPublish],
        JobLimits::default(),
        Duration::from_secs(30),
    )
    .await;
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/records/bulk",
            json!({ "items": [{ "key": "a", "op": "create", "body": {} }] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn deadline_promotion_returns_202_with_committed_statuses() {
    // A zero deadline promotes after the first commit: the 202 carries that one
    // committed item, and the spawned job commits the rest (visible via the poll).
    let TestJobApp { app, .. } = boot_jobs(
        "bulk_promote",
        &[Capability::BulkSubmit, Capability::IngestPublish],
        JobLimits::default(),
        Duration::ZERO,
    )
    .await;

    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/records/bulk",
            json!({ "items": [
                { "key": "i0", "op": "create", "body": { "v": 0 } },
                { "key": "i1", "op": "create", "body": { "v": 1 } },
                { "key": "i2", "op": "create", "body": { "v": 2 } }
            ] }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let job_id = body["job_id"].as_str().expect("job_id").to_owned();
    let ticket = body["ticket"].as_str().expect("ticket").to_owned();
    // The 202 carries exactly the item(s) committed before promotion.
    let committed = body["committed"].as_array().expect("committed");
    assert_eq!(committed.len(), 1);
    assert_eq!(committed[0]["key"], json!("i0"));
    assert_eq!(committed[0]["status"], json!("created"));

    // The job commits the remaining items; the poll's buffered result carries them.
    let mut result = Vec::new();
    for _ in 0..200 {
        let (s, poll) = send(&app, ticketed_get(&format!("/bulk/jobs/{job_id}"), &ticket)).await;
        assert_eq!(s, StatusCode::OK);
        if poll["status"] == json!("completed") {
            result = poll["result"].as_array().cloned().unwrap_or_default();
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // The union of the 202 body (i0) and the job result (i1, i2) is the full set.
    let keys: Vec<&str> = result.iter().filter_map(|i| i["key"].as_str()).collect();
    assert!(
        keys.contains(&"i1") && keys.contains(&"i2"),
        "job streamed the rest: {result:?}"
    );
    assert_eq!(result.len(), 2);
}
