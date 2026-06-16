//! Integration: a rule CRUDs over HTTP through the gate, dry-runs without firing.
//!
//! Rules persist as `kind:"rule"` records, so a create/update/delete must cross
//! the WS-05 gate on the `RuleDefine` grant (an audit row is written) while the
//! list/get reads run on the WS-03 scoped session. On top of CRUD this exercises
//! the two studio surfaces: a side-effect-free dry-run that resolves a binding
//! against seeded history and returns a verdict + frame *without* recording an
//! insight, and the referencing (blast-radius) read. Name uniqueness is a `409`
//! and a non-compiling script a `400`, caught before the write.

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

/// A single-binding temperature rule body the studio would POST.
fn temp_rule_body(name: &str) -> Value {
    json!({
        "name": name,
        "script": "#{ fired: temp > 25.0, value: temp, reason: \"too hot\" }",
        "inputs": [{
            "name": "temp",
            "table": "records",
            "field": "temperature",
            "grain": "minute",
            "aggregate": "avg"
        }],
        "subrules": [],
        "output": "high-temperature"
    })
}

#[tokio::test]
async fn rule_round_trips_through_the_gate_with_audit_rows() {
    let TestApp { app, store } = boot(
        "server_rules_crud",
        &[Capability::RuleDefine, Capability::IngestPublish],
    )
    .await;

    // CREATE
    let (status, created) = send(&app, authed("POST", "/rules", temp_rule_body("temp-high"))).await;
    assert_eq!(status, StatusCode::OK, "create: {created:?}");
    assert_eq!(created["name"], json!("temp-high"));
    assert_eq!(created["inputs"][0]["field"], json!("temperature"));
    let id = created["id"].as_str().expect("rule id").to_owned();

    // LIST + GET on the scoped session
    let (status, listed) = send(&app, authed("GET", "/rules", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().expect("list").len(), 1);

    let (status, fetched) = send(&app, authed("GET", "/rules/temp-high", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["name"], json!("temp-high"));

    // UPDATE — replace the script; the name is immutable and carried through.
    let (status, updated) = send(
        &app,
        authed(
            "PATCH",
            "/rules/temp-high",
            json!({
                "script": "temp > 30.0",
                "inputs": [{
                    "name": "temp", "table": "records", "field": "temperature",
                    "grain": "minute", "aggregate": "avg"
                }],
                "subrules": [],
                "output": "high-temperature"
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update: {updated:?}");
    assert_eq!(updated["script"], json!("temp > 30.0"));

    // DELETE, then GET is not found.
    let (status, _) = send(&app, authed("DELETE", "/rules/temp-high", Value::Null)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, authed("GET", "/rules/temp-high", Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The three mutations each wrote an audit row carrying a correlation id.
    let mut rows = store
        .raw()
        .query("SELECT action, correlation_id FROM audit WHERE target = $t")
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
}

#[tokio::test]
async fn a_duplicate_name_is_a_conflict_and_a_broken_script_is_bad_request() {
    let app = boot("server_rules_validate", &[Capability::RuleDefine])
        .await
        .app;

    let (status, _) = send(&app, authed("POST", "/rules", temp_rule_body("dup"))).await;
    assert_eq!(status, StatusCode::OK);

    // Same name again → 409.
    let (status, _) = send(&app, authed("POST", "/rules", temp_rule_body("dup"))).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A non-compiling script → 400 before the write.
    let broken = json!({
        "name": "broken", "script": "temp >", "inputs": [], "output": "x"
    });
    let (status, _) = send(&app, authed("POST", "/rules", broken)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A non-slug name → 400.
    let bad_name = json!({
        "name": "Bad Name", "script": "true", "inputs": [], "output": "x"
    });
    let (status, _) = send(&app, authed("POST", "/rules", bad_name)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_dry_run_resolves_the_binding_and_returns_a_verdict_without_firing() {
    let TestApp { app, store } = boot(
        "server_rules_dryrun",
        &[Capability::RuleDefine, Capability::IngestPublish],
    )
    .await;

    // Seed two temperature readings so the minute rollup has a bucket to resolve.
    for value in [28.0, 32.0] {
        let (status, _) = send(
            &app,
            authed(
                "POST",
                "/records",
                json!({ "content": { "temperature": value } }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    // Dry-run the on-screen draft (no stored rule required — the name is a label).
    let (status, verdict) = send(
        &app,
        authed(
            "POST",
            "/rules/draft/dryrun",
            json!({
                "script": "#{ fired: temp > 25.0, value: temp, reason: \"hot\" }",
                "inputs": [{
                    "name": "temp", "table": "records", "field": "temperature",
                    "grain": "minute", "aggregate": "avg"
                }],
                "subrules": []
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "dry-run: {verdict:?}");
    assert_eq!(verdict["fired"], json!(true));
    assert_eq!(verdict["reason"], json!("hot"));
    // The avg of the two readings was the value the rule decided on.
    assert_eq!(verdict["value"], json!(30.0));
    // The frame the binding saw is returned for the debugger chart.
    assert_eq!(verdict["inputs"][0]["name"], json!("temp"));
    assert!(
        !verdict["inputs"][0]["buckets"]
            .as_array()
            .expect("buckets")
            .is_empty(),
        "the resolved binding must carry its window buckets"
    );

    // A dry-run records no insight — the insights table stays empty.
    let mut rows = store
        .raw()
        .query("SELECT count() AS n FROM record WHERE content.content.kind = 'high-temperature' GROUP ALL")
        .await
        .expect("count insights");
    let counted: Vec<Value> = rows.take(0).expect("rows");
    let n = counted.first().and_then(|r| r["n"].as_i64()).unwrap_or(0);
    assert_eq!(n, 0, "a dry-run must not record an insight");
}

#[tokio::test]
async fn referencing_lists_the_rules_that_compose_a_rule() {
    let app = boot("server_rules_ref", &[Capability::RuleDefine])
        .await
        .app;

    // A leaf rule and a parent that composes it.
    let (status, _) = send(&app, authed("POST", "/rules", temp_rule_body("hot"))).await;
    assert_eq!(status, StatusCode::OK);
    let parent = json!({
        "name": "hot-and-humid",
        "script": "#{ fired: invoke(\"hot\") > 0.5, value: 1.0, reason: \"composed\" }",
        "inputs": [],
        "subrules": ["hot"],
        "output": "comfort"
    });
    let (status, _) = send(&app, authed("POST", "/rules", parent)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, refs) = send(&app, authed("GET", "/rules/hot/referencing", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = refs
        .as_array()
        .expect("refs")
        .iter()
        .filter_map(|r| r["name"].as_str())
        .collect();
    assert_eq!(names, vec!["hot-and-humid"]);
}

#[tokio::test]
async fn a_create_without_the_rule_define_grant_is_forbidden() {
    let app = boot("server_rules_nogrant", &[]).await.app;
    let (status, _) = send(&app, authed("POST", "/rules", temp_rule_body("x"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
