//! Integration: `GET`/`PATCH /prefs` and post-read unit conversion (§2).
//!
//! Proves the preferences wiring: a fresh principal gets the canonical defaults,
//! a `PATCH` persists through the gate and a later `GET` reflects it, and a query
//! declaring a quantity column converts its values to the principal's unit system
//! once that system is imperial (the cache still holds raw metric values).

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

#[tokio::test]
async fn prefs_default_then_patch_then_read_back() {
    let TestApp { app, .. } = boot("server_prefs", &[Capability::IngestPublish]).await;

    // A fresh principal gets the canonical defaults.
    let (status, prefs) = send(&app, authed("GET", "/prefs", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(prefs["units"], json!("metric"));
    assert_eq!(prefs["datetime"], json!("%Y-%m-%d %H:%M:%S"));

    // Patch units + timezone; the response reflects the merge.
    let (status, patched) = send(
        &app,
        authed(
            "PATCH",
            "/prefs",
            json!({ "units": "imperial", "timezone": "Australia/Sydney" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["units"], json!("imperial"));
    assert_eq!(patched["timezone"], json!("Australia/Sydney"));
    // Datetime was not in the patch, so it keeps its default.
    assert_eq!(patched["datetime"], json!("%Y-%m-%d %H:%M:%S"));

    // A later GET reads the persisted record back.
    let (status, reread) = send(&app, authed("GET", "/prefs", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reread["units"], json!("imperial"));
    assert_eq!(reread["timezone"], json!("Australia/Sydney"));
}

#[tokio::test]
async fn an_unknown_unit_system_is_rejected() {
    let TestApp { app, .. } = boot("server_prefs_bad", &[Capability::IngestPublish]).await;
    let (status, _) = send(
        &app,
        authed("PATCH", "/prefs", json!({ "units": "furlongs" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_declared_quantity_column_converts_to_the_principal_unit_system() {
    let TestApp { app, .. } = boot(
        "server_prefs_convert",
        &[Capability::IngestPublish, Capability::ExternalQuery],
    )
    .await;

    // Seed a record carrying a canonical (metric) temperature of 100°C.
    let (status, _) = send(
        &app,
        authed("POST", "/records", json!({ "content": { "temp": 100.0 } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let query = json!({
        "sql": "SELECT CAST(json_get(json_get(content, 'content'), 'temp') AS DOUBLE) AS temp FROM record",
        "quantities": { "temp": "temperature" }
    });

    // With the default metric preference the value is unchanged (100°C).
    let (status, metric) = send(&app, authed("POST", "/query", query.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert!((metric["rows"][0]["temp"].as_f64().unwrap() - 100.0).abs() < 1e-9);

    // Switch the principal to imperial; the same query now returns 212°F — the
    // conversion is a post-read per-caller layer over raw cached metric values.
    let (status, _) = send(
        &app,
        authed("PATCH", "/prefs", json!({ "units": "imperial" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, imperial) = send(&app, authed("POST", "/query", query)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        (imperial["rows"][0]["temp"].as_f64().unwrap() - 212.0).abs() < 1e-6,
        "imperial preference converts °C to °F: {imperial:?}"
    );
}
