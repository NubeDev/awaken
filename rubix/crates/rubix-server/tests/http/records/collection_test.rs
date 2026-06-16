//! Integration: the collection contract enforced over HTTP — typed validation on
//! write and a `?kind=` filter on list.
//!
//! Drives the real route table on kv-mem (`rubix/docs/sessions/WS-16.md`): with a
//! `site` collection registered, a write that violates its schema is refused with
//! 422 (and no record lands), a valid write succeeds, and `GET /records?kind=site`
//! returns only that collection's records while the unfiltered list returns all.

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_core::{COLLECTION_KIND, Record, create_record};
use rubix_gate::Capability;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{NS, SECRET, SUBJECT, TestApp, boot};

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

/// Register the `site` collection directly in the store (a platform/admin act,
/// not a gated tenant write — mirrors the bootstrap seed path).
async fn register_site_collection(store: &rubix_store::StoreHandle) {
    let def = Record::new(
        NS,
        json!({
            "kind": COLLECTION_KIND,
            "name": "site",
            "schema": [
                { "name": "key",  "type": "text",   "required": true },
                { "name": "name", "type": "text",   "required": true },
                { "name": "area", "type": "number" }
            ]
        }),
    );
    create_record(store.raw(), &def)
        .await
        .expect("register collection");
}

#[tokio::test]
async fn invalid_collection_content_is_unprocessable() {
    let TestApp { app, store } =
        boot("server_collection_invalid", &[Capability::IngestPublish]).await;
    register_site_collection(&store).await;

    // Missing required `name`, and `area` is the wrong type.
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/records",
            json!({ "content": { "kind": "site", "key": "s1", "area": "huge" } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn valid_collection_content_is_created() {
    let TestApp { app, store } =
        boot("server_collection_valid", &[Capability::IngestPublish]).await;
    register_site_collection(&store).await;

    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/records",
            json!({ "content": { "kind": "site", "key": "s1", "name": "HQ", "area": 1200 } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["content"]["name"], json!("HQ"));
}

#[tokio::test]
async fn list_filters_by_kind() {
    let TestApp { app, store } = boot("server_collection_list", &[Capability::IngestPublish]).await;
    register_site_collection(&store).await;

    // Two sites and one unrelated record, all through the gate.
    for content in [
        json!({ "content": { "kind": "site", "key": "s1", "name": "HQ" } }),
        json!({ "content": { "kind": "site", "key": "s2", "name": "Depot" } }),
        json!({ "content": { "kind": "memo", "text": "hi" } }),
    ] {
        let (status, _) = send(&app, authed("POST", "/records", content)).await;
        assert_eq!(status, StatusCode::OK);
    }

    // The filtered list returns only sites.
    let (status, sites) = send(&app, authed("GET", "/records?kind=site", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    let sites = sites.as_array().expect("array");
    assert_eq!(sites.len(), 2);
    assert!(sites.iter().all(|r| r["content"]["kind"] == json!("site")));

    // The unfiltered list returns everything (plus the registered collection record).
    let (status, all) = send(&app, authed("GET", "/records", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(all.as_array().expect("array").len() >= 3);
}
