//! Integration: `POST /query` lowers dashboard variables, injection-safely.
//!
//! The variables contract (`rubix/docs/design/VARIABLES-AND-TEMPLATING.md`): a
//! templated chart sends `$name` / `${name:csv}` / `$__sqlIn(name)` references plus
//! a `variables` array; the backend lowers each value into an escaped SQL literal
//! **before** the read-only guard, so one board serves a fleet and a hostile value
//! binds as a literal rather than executing. These tests drive the real route to
//! prove the selection filters rows and the injection payload is neutralised.

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

/// Seed one record per `(site, temp)` so a variable can filter by site.
async fn seed_sites(app: &axum::Router, rows: &[(&str, f64)]) {
    for (site, temp) in rows {
        let (status, _) = send(
            app,
            authed(
                "POST",
                "/records",
                json!({ "content": { "site": site, "temp": temp } }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "seed {site} landed");
    }
}

#[tokio::test]
async fn a_single_select_variable_filters_to_one_site() {
    let TestApp { app, .. } = boot(
        "server_vars_single",
        &[Capability::IngestPublish, Capability::ExternalQuery],
    )
    .await;
    seed_sites(&app, &[("hq", 21.0), ("hq", 22.0), ("tower", 30.0)]).await;

    // One board authored once; the `$site` selection picks which site's rows.
    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/query",
            json!({
                "sql": "SELECT count(*) AS n FROM record WHERE json_get(json_get(content, 'content'), 'site') = $site",
                "variables": [ { "name": "site", "value": "hq" } ]
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rows"][0]["n"], json!(2), "only the two hq rows match");
}

#[tokio::test]
async fn a_multi_select_expands_a_safe_in_list() {
    let TestApp { app, .. } = boot(
        "server_vars_multi",
        &[Capability::IngestPublish, Capability::ExternalQuery],
    )
    .await;
    seed_sites(&app, &[("hq", 21.0), ("tower", 30.0), ("depot", 12.0)]).await;

    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/query",
            json!({
                "sql": "SELECT count(*) AS n FROM record WHERE json_get(json_get(content, 'content'), 'site') IN $__sqlIn(site)",
                "variables": [ { "name": "site", "value": ["hq", "tower"] } ]
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rows"][0]["n"], json!(2), "hq + tower, not depot");
}

#[tokio::test]
async fn an_injection_payload_binds_as_a_literal_and_runs_safely() {
    let TestApp { app, .. } = boot(
        "server_vars_injection",
        &[Capability::IngestPublish, Capability::ExternalQuery],
    )
    .await;
    seed_sites(&app, &[("hq", 21.0)]).await;

    // A classic break-out payload as the selected value. Lowered as an escaped
    // literal it simply matches no site — the record table is untouched, and the
    // statement stays a single read.
    let (status, body) = send(
        &app,
        authed(
            "POST",
            "/query",
            json!({
                "sql": "SELECT count(*) AS n FROM record WHERE json_get(json_get(content, 'content'), 'site') = $site",
                "variables": [
                    { "name": "site", "value": "'); DROP TABLE record; --" }
                ]
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "the hostile value is just data");
    assert_eq!(body["rows"][0]["n"], json!(0), "no site equals the payload");

    // Prove the table survived: the seeded row is still queryable.
    let (status, after) = send(
        &app,
        authed(
            "POST",
            "/query",
            json!({ "sql": "SELECT count(*) AS n FROM record" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after["rows"][0]["n"], json!(1), "record table intact");
}

#[tokio::test]
async fn an_unknown_explicit_variable_is_a_bad_request() {
    let TestApp { app, .. } = boot("server_vars_unknown", &[Capability::ExternalQuery]).await;

    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/query",
            json!({
                "sql": "SELECT * FROM record WHERE x = ${ghost}",
                "variables": []
            }),
        ),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "an explicit reference to an unsupplied variable is rejected"
    );
}
