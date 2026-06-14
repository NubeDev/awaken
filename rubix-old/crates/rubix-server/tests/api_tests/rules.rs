//! Stored-rule routes + the live rule-node board path (RULES_ENGINE L1–L3).

use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

const ORG: &str = "nube";

async fn create_rule(app: &TestApp, name: &str, script: &str) -> (StatusCode, serde_json::Value) {
    app.request(
        "POST",
        &format!("/api/v1/orgs/{ORG}/rules"),
        Some(json!({ "name": name, "script": script })),
    )
    .await
}

#[tokio::test]
async fn rule_crud_round_trip() {
    let app = TestApp::new();

    let (status, body) = create_rule(&app, "temp-high", "finding(\"warning\", \"hot\")").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["name"], "temp-high");
    assert_eq!(body["org"], ORG);

    // List shows it.
    let (status, body) = app
        .request("GET", &format!("/api/v1/orgs/{ORG}/rules"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Get by name.
    let (status, body) = app
        .request("GET", &format!("/api/v1/orgs/{ORG}/rules/temp-high"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["script"], "finding(\"warning\", \"hot\")");

    // Update the script.
    let (status, body) = app
        .request(
            "PUT",
            &format!("/api/v1/orgs/{ORG}/rules/temp-high"),
            Some(json!({ "script": "finding(\"fault\", \"very hot\")" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["script"], "finding(\"fault\", \"very hot\")");

    // Delete.
    let (status, _) = app
        .request(
            "DELETE",
            &format!("/api/v1/orgs/{ORG}/rules/temp-high"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app
        .request("GET", &format!("/api/v1/orgs/{ORG}/rules/temp-high"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn duplicate_name_in_org_conflicts() {
    let app = TestApp::new();
    let (s1, _) = create_rule(&app, "dup", "clear()").await;
    assert_eq!(s1, StatusCode::CREATED);
    let (s2, _) = create_rule(&app, "dup", "clear()").await;
    assert_eq!(s2, StatusCode::CONFLICT);
}

#[tokio::test]
async fn missing_rule_is_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request("GET", &format!("/api/v1/orgs/{ORG}/rules/nope"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn referencing_lists_the_change_impact() {
    let app = TestApp::new();
    create_rule(&app, "rollup-temp", "finding(\"info\", \"base\")").await;
    create_rule(
        &app,
        "ahu-health",
        "let r = rule(\"rollup-temp\", df, #{}); if r.flagged { finding(\"fault\", \"unhealthy\") }",
    )
    .await;
    create_rule(&app, "unrelated", "finding(\"info\", \"x\")").await;

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/orgs/{ORG}/rules/rollup-temp/referencing"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let names: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["ahu-health"]);
}

/// Seed a site/equip/point with one current sample so a `query_his` node returns
/// a one-row frame. Returns the temp point keyexpr prefix the board addresses.
async fn seed_point(app: &TestApp) {
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let temp = app.create_point(&equip, "sensor", "temp").await;
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{temp}/cur"),
            Some(json!({"value": 30.0})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

fn rule_board(rule_config: serde_json::Value) -> serde_json::Value {
    json!({
        "board": {
            "nodes": [
                {"id": "q1", "component": "query_his",
                 "config": {"point": "nube/hq/ahu-3/temp"}},
                {"id": "rule1", "component": "rule", "config": rule_config},
                {"id": "e1", "component": "emit_spark",
                 "config": {"site": "nube/hq", "rule": "temp-check", "severity": "info",
                            "point": "nube/hq/ahu-3/temp"}}
            ],
            "connections": [
                {"from_node": "q1", "from_port": "output",
                 "to_node": "rule1", "to_port": "input"},
                {"from_node": "rule1", "from_port": "finding",
                 "to_node": "e1", "to_port": "finding"}
            ]
        }
    })
}

#[tokio::test]
async fn inline_rule_board_emits_spark_with_rule_severity() {
    // L1 + L3 live over HTTP: query → rule → emit_spark, the rule's `fault`
    // severity overriding emit_spark's static `info` config.
    let app = TestApp::new();
    seed_point(&app).await;

    let board = rule_board(json!({ "script": "finding(\"fault\", \"too hot\")" }));
    let (status, body) = app.request("POST", "/api/v1/boards/run", Some(board)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, sparks) = app
        .request("GET", "/api/v1/sparks?rule=temp-check", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let sparks = sparks.as_array().unwrap();
    assert_eq!(sparks.len(), 1, "{sparks:?}");
    assert_eq!(sparks[0]["severity"], "fault");
    assert_eq!(sparks[0]["message"], "too hot");
    // The `point` config on emit_spark is resolved to the implicated point id, so
    // the finding links back to its point (the UI's "Implicated points").
    let points = sparks[0]["point_ids"].as_array().unwrap();
    assert_eq!(points.len(), 1, "expected one implicated point: {sparks:?}");
}

#[tokio::test]
async fn stored_rule_board_resolves_by_name() {
    // L2 live over HTTP: a stored rule is created, then a board references it by
    // name and the verdict drives the spark.
    let app = TestApp::new();
    seed_point(&app).await;
    create_rule(&app, "temp-high", "finding(\"warning\", \"stored fired\")").await;

    let board = rule_board(json!({ "rule": "temp-high" }));
    let (status, body) = app.request("POST", "/api/v1/boards/run", Some(board)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, sparks) = app
        .request("GET", "/api/v1/sparks?rule=temp-check", None)
        .await;
    let sparks = sparks.as_array().unwrap();
    assert_eq!(sparks.len(), 1, "{sparks:?}");
    assert_eq!(sparks[0]["severity"], "warning");
    assert_eq!(sparks[0]["message"], "stored fired");
}

#[tokio::test]
async fn missing_stored_rule_fails_closed_no_spark() {
    // Fail-closed resolution: a board referencing an absent stored rule errors
    // the node and emits no spark.
    let app = TestApp::new();
    seed_point(&app).await;

    let board = rule_board(json!({ "rule": "does-not-exist" }));
    let (status, body) = app.request("POST", "/api/v1/boards/run", Some(board)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, sparks) = app
        .request("GET", "/api/v1/sparks?rule=temp-check", None)
        .await;
    assert!(sparks.as_array().unwrap().is_empty());
}

// --- dry-run (RULES_ENGINE debugger spine) ----------------------------------

const TEMP_KEYEXPR: &str = "nube/hq/ahu-3/temp";

#[tokio::test]
async fn dry_run_inline_flags_and_returns_frame() {
    // The seeded point has one sample at 30.0. A flagging inline rule returns a
    // flagged verdict plus the frame it saw, so the UI can chart the input.
    let app = TestApp::new();
    seed_point(&app).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/orgs/{ORG}/rules/dry-run"),
            Some(json!({
                "script": "finding(\"warning\", \"hot\").with_value(30.0)",
                "point": TEMP_KEYEXPR,
                "limit": 100
            })),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["flagged"], true);
    assert_eq!(body["result"]["severity"], "warning");
    assert_eq!(body["result"]["message"], "hot");
    assert_eq!(body["result"]["value"], 30.0);
    assert_eq!(body["frame"]["row_count"], 1);
    assert_eq!(body["frame"]["rows"][0]["value"], 30.0);
}

#[tokio::test]
async fn dry_run_clear_returns_unflagged() {
    let app = TestApp::new();
    seed_point(&app).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/orgs/{ORG}/rules/dry-run"),
            Some(json!({ "script": "clear()", "point": TEMP_KEYEXPR })),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["flagged"], false);
}

#[tokio::test]
async fn dry_run_stored_rule_resolves_by_name() {
    let app = TestApp::new();
    seed_point(&app).await;
    create_rule(&app, "temp-high", "finding(\"fault\", \"stored hit\")").await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/orgs/{ORG}/rules/dry-run"),
            Some(json!({ "rule": "temp-high", "point": TEMP_KEYEXPR })),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["flagged"], true);
    assert_eq!(body["result"]["severity"], "fault");
    assert_eq!(body["result"]["message"], "stored hit");
}

#[tokio::test]
async fn dry_run_compile_error_is_bad_request() {
    let app = TestApp::new();
    seed_point(&app).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/orgs/{ORG}/rules/dry-run"),
            Some(json!({ "script": "this is not valid rhai (((", "point": TEMP_KEYEXPR })),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn dry_run_without_point_runs_on_empty_frame() {
    // No `point` is a compile/shape check: the rule runs over an empty frame.
    let app = TestApp::new();

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/orgs/{ORG}/rules/dry-run"),
            Some(json!({ "script": "clear()" })),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["flagged"], false);
    assert_eq!(body["frame"]["row_count"], 0);
}

#[tokio::test]
async fn dry_run_requires_exactly_one_source() {
    let app = TestApp::new();

    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/orgs/{ORG}/rules/dry-run"),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/orgs/{ORG}/rules/dry-run"),
            Some(json!({ "script": "clear()", "rule": "temp-high" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
