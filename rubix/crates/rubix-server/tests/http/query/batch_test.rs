//! Integration: `POST /query/batch` isolates per-item errors over HTTP.
//!
//! The §3 contract (`rubix/docs/design/DASHBOARDS-SCOPE.md`): a board runs all its
//! panels in one request, one bad panel reports its error while the others render,
//! and the HTTP status stays `200`. Each statement still runs through the same
//! `external-query` capability and the same scoped session as `POST /query`.

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_gate::Capability;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{SECRET, SUBJECT, TestApp, boot};

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

/// Find the batch result with the given key.
fn result_for<'a>(body: &'a Value, key: &str) -> &'a Value {
    body["results"]
        .as_array()
        .expect("results array")
        .iter()
        .find(|r| r["key"] == json!(key))
        .unwrap_or_else(|| panic!("no result for key {key}"))
}

#[tokio::test]
async fn a_bad_panel_reports_its_error_while_others_render() {
    let TestApp { app, .. } = boot(
        "server_batch",
        &[Capability::IngestPublish, Capability::ExternalQuery],
    )
    .await;

    // Seed two records so the good panels return rows.
    for temp in [21.5, 22.5] {
        let (status, _) = send(
            &app,
            authed("POST", "/records", json!({ "content": { "temp": temp } })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/query/batch",
            json!({
                "queries": [
                    { "key": "good", "sql": "SELECT count(*) AS n FROM record" },
                    { "key": "bad-syntax", "sql": "SELEKT oops FROM" },
                    { "key": "not-read", "sql": "DELETE FROM record" },
                    { "key": "bad-time", "sql": "SELECT $__timeBucket(created) FROM record",
                      "time": { "from": "now-1h", "to": "now" } }
                ]
            }),
        ),
    )
    .await;

    // The batch itself succeeds even though individual items fail.
    assert_eq!(status, StatusCode::OK, "batch is 200 despite bad items");

    // The good panel returns its rows + columns.
    let good = result_for(&body, "good");
    assert_eq!(
        good["rows"][0]["n"],
        json!(2),
        "good panel counted both records"
    );
    assert!(good["columns"].is_array(), "good panel carries columns");
    assert!(good.get("error").is_none() || good["error"].is_null());

    // The malformed-SQL panel carries an error, no rows.
    let bad = result_for(&body, "bad-syntax");
    assert!(bad["error"].is_string(), "bad-syntax panel has an error");
    assert!(bad.get("rows").is_none() || bad["rows"].is_null());

    // A non-read statement is rejected by the guard, per item.
    let not_read = result_for(&body, "not-read");
    assert!(
        not_read["error"].is_string(),
        "non-read panel rejected per item"
    );

    // A bucket macro with no grain is a per-item time error, not a batch failure.
    let bad_time = result_for(&body, "bad-time");
    assert!(
        bad_time["error"].is_string(),
        "bad-time panel has a time error"
    );
}

#[tokio::test]
async fn batch_requires_the_external_query_capability() {
    // Granted record-write but NOT external-query.
    let TestApp { app, .. } = boot("server_batch_denied", &[Capability::IngestPublish]).await;

    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/query/batch",
            json!({ "queries": [ { "key": "k", "sql": "SELECT 1" } ] }),
        ),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "no external-query grant is 403"
    );
}

#[tokio::test]
async fn an_empty_batch_is_rejected() {
    let TestApp { app, .. } = boot("server_batch_empty", &[Capability::ExternalQuery]).await;
    let (status, _) = send(
        &app,
        authed("POST", "/query/batch", json!({ "queries": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
