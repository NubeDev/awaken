//! Integration: readings bulk-append and read back over HTTP, off the gate.
//!
//! The data-plane contract (`rubix/docs/design/READINGS-TIMESERIES.md`): `POST
//! /readings` is **not** `POST /records` — it takes one `readings-append`
//! capability decision per request, writes append-only on the owner handle (no
//! audit/undo per sample), and the deterministic `(series, at)` id makes a
//! re-append idempotent. `GET /readings` reads the window back on the scoped
//! session, `at`-ordered. This test exercises append → read → re-append (no-op)
//! over the real router, plus the fail-closed denial when the grant is absent.

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
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, json)
}

/// Build an authenticated request carrying the principal credentials.
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
async fn readings_append_then_window_read_round_trips_off_the_gate() {
    let TestApp { app, store } = boot("server_readings", &[Capability::ReadingsAppend]).await;

    // Append three back-dated samples for one series (out of order on the wire).
    let body = json!({
        "series": "reg-1",
        "samples": [
            { "at": "2026-06-14T12:00:00Z", "value": 21.0 },
            { "at": "2026-06-14T10:00:00Z", "value": 20.0 },
            { "at": "2026-06-14T11:00:00Z", "value": 22.0 }
        ]
    });
    let (status, appended) = send(&app, authed("POST", "/readings", body.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(appended["appended"], json!(3));

    // Read the window back on the scoped session — at-ordered, lean shape.
    let (status, rows) = send(
        &app,
        authed(
            "GET",
            "/readings?series=reg-1&from=2026-06-14T00:00:00Z&to=2026-06-15T00:00:00Z",
            Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let values: Vec<f64> = rows
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["value"].as_f64().expect("value"))
        .collect();
    // Ordered by measurement instant: 10:00→20.0, 11:00→22.0, 12:00→21.0 — the
    // wire order (12:00 first) does not survive; `at` decides the order.
    assert_eq!(values, [20.0, 22.0, 21.0], "ordered by measurement instant");
    // `series` comes back as the bare register id for a direct `series == id` join.
    assert_eq!(rows[0]["series"], json!("reg-1"));

    // Re-append the same batch: deterministic ids make it an idempotent no-op, so
    // a second window read still returns exactly three rows.
    let (status, _) = send(&app, authed("POST", "/readings", body)).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<Value> = store
        .raw()
        .query("SELECT VALUE id FROM reading")
        .await
        .expect("count")
        .take(0)
        .expect("take");
    assert_eq!(ids.len(), 3, "re-append does not duplicate rows");
}

#[tokio::test]
async fn append_without_the_grant_is_forbidden() {
    // Boot with no capabilities: the once-per-request check fails closed.
    let app = boot("server_readings_nogrant", &[]).await.app;
    let body = json!({ "series": "reg-1", "samples": [{ "at": "2026-06-14T10:00:00Z", "value": 1.0 }] });
    let (status, _) = send(&app, authed("POST", "/readings", body)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_malformed_sample_timestamp_is_rejected() {
    let app = boot("server_readings_badts", &[Capability::ReadingsAppend])
        .await
        .app;
    let body = json!({ "series": "reg-1", "samples": [{ "at": "not-a-date", "value": 1.0 }] });
    let (status, _) = send(&app, authed("POST", "/readings", body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
