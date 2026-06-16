//! Integration: a record CRUDs over HTTP through the gate, writing audit rows.
//!
//! The WS-16 Done definition (`rubix/docs/sessions/WS-16.md`): create→get→update
//! →delete a record over HTTP, assert the mutations went through the WS-05 gate
//! (an audit row exists carrying the correlation id) while reads ran on the WS-03
//! scoped session.

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_gate::Capability;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{SECRET, SUBJECT, TestApp, boot};

/// Send a request through the app, returning the status and JSON body (or null).
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

/// Build an authenticated JSON request carrying the principal credentials.
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

#[tokio::test]
async fn record_round_trips_through_the_gate_with_audit_rows() {
    let TestApp { app, store } = boot("server_crud", &[Capability::IngestPublish]).await;

    // CREATE
    let (status, created) = send(
        &app,
        authed("POST", "/records", json!({ "content": { "temp": 21.5 } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["id"].as_str().expect("created id").to_owned();
    assert_eq!(created["content"]["temp"], json!(21.5));

    // GET on the scoped session
    let (status, fetched) = send(&app, authed("GET", &format!("/records/{id}"), Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["id"], json!(id));

    // UPDATE
    let (status, updated) = send(
        &app,
        authed(
            "PATCH",
            &format!("/records/{id}"),
            json!({ "content": { "temp": 30.0 } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["content"]["temp"], json!(30.0));

    // DELETE
    let (status, _) = send(
        &app,
        authed("DELETE", &format!("/records/{id}"), Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // GET after delete is not found
    let (status, _) = send(&app, authed("GET", &format!("/records/{id}"), Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The three mutations each wrote an audit row carrying a correlation id.
    let mut rows = store
        .raw()
        .query("SELECT action, target, correlation_id FROM audit WHERE target = $t")
        .bind(("t", id.clone()))
        .await
        .expect("query audit");
    let audited: Vec<Value> = rows.take(0).expect("audit rows");
    let actions: Vec<&str> = audited
        .iter()
        .filter_map(|row| row["action"].as_str())
        .collect();
    assert!(
        actions.contains(&"create"),
        "missing create audit: {audited:?}"
    );
    assert!(
        actions.contains(&"update"),
        "missing update audit: {audited:?}"
    );
    assert!(
        actions.contains(&"delete"),
        "missing delete audit: {audited:?}"
    );
    for row in &audited {
        let corr = row["correlation_id"].as_str().expect("correlation id");
        assert!(
            !corr.is_empty(),
            "audit row missing correlation id: {row:?}"
        );
    }
}

#[tokio::test]
async fn unauthenticated_create_is_rejected() {
    let app = boot("server_crud_unauth", &[Capability::IngestPublish])
        .await
        .app;
    let request = Request::builder()
        .method("POST")
        .uri("/records")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "content": {} }).to_string()))
        .expect("build request");
    let (status, _) = send(&app, request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_without_the_grant_is_forbidden() {
    // Boot with no capabilities granted: the gate denies the write.
    let app = boot("server_crud_nogrant", &[]).await.app;
    let (status, _) = send(&app, authed("POST", "/records", json!({ "content": {} }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
