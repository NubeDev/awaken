//! Flow integration: a reflow board's `PointAccess` reads and commands real
//! store points through the priority array.

use axum::http::StatusCode;
use rubix_core::PointValue;
use rubix_flow::PointAccess;
use rubix_server::flow::StorePointAccess;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn store_point_access_reads_writes_and_histories() {
    let (app, store) = TestApp::with_store();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let _point = app.create_point(&equip, "cmd", "fan").await;
    let keyexpr = "nube/hq/ahu-3/fan";

    let access = StorePointAccess::new(store);

    // No command yet → no effective value.
    assert_eq!(access.read_point(keyexpr).await.unwrap(), None);

    // Command priority 8 → becomes the effective value, readable back.
    let effective = access
        .write_point(keyexpr, 8, PointValue::Bool(true))
        .await
        .unwrap();
    assert_eq!(effective, Some(PointValue::Bool(true)));
    assert_eq!(
        access.read_point(keyexpr).await.unwrap(),
        Some(PointValue::Bool(true))
    );

    // The command landed in history.
    let his = access.query_his(keyexpr, 10).await.unwrap();
    assert_eq!(his.len(), 1);
    assert_eq!(his[0].value, PointValue::Bool(true));
}

#[tokio::test]
async fn run_board_endpoint_reads_then_commands_a_point() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let temp = app.create_point(&equip, "sensor", "temp").await;
    let _fan = app.create_point(&equip, "cmd", "fan").await;

    // Seed the sensor's current value.
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{temp}/cur"),
            Some(json!({"value": 21.5})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // A board that reads temp and commands fan with the read value at prio 8.
    let board = json!({
        "board": {
            "nodes": [
                {"id": "r1", "component": "read_point",
                 "config": {"point": "nube/hq/ahu-3/temp"}},
                {"id": "w1", "component": "write_point",
                 "config": {"point": "nube/hq/ahu-3/fan", "priority": 8}}
            ],
            "connections": [
                {"from_node": "r1", "from_port": "output",
                 "to_node": "w1", "to_port": "value"}
            ]
        }
    });
    let (status, body) = app.request("POST", "/api/v1/boards/run", Some(board)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // The write node forwarded the commanded value on its `output` port. (The
    // read node's `output` is consumed by routing to w1, so it isn't surfaced.)
    let outputs = body["outputs"].as_array().expect("outputs array");
    let write_out = outputs
        .iter()
        .find(|o| o["node"] == "w1" && o["port"] == "output")
        .expect("write output");
    assert_eq!(write_out["value"], json!(21.5));

    // The board's write reached the store: fan now reads 21.5 at the effective
    // value (commanded at priority 8 from the board).
    let (status, fan) = app
        .request("GET", &format!("/api/v1/points/{_fan}"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{fan}");
    assert_eq!(fan["point"]["cur_value"], json!(21.5));
}

#[tokio::test]
async fn run_board_rejects_unknown_component() {
    let app = TestApp::new();
    let board = json!({
        "board": {
            "nodes": [{"id": "x", "component": "frobnicate", "config": {}}],
            "connections": []
        }
    });
    let (status, body) = app.request("POST", "/api/v1/boards/run", Some(board)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn unknown_keyexpr_is_an_error() {
    let (_app, store) = TestApp::with_store();
    let access = StorePointAccess::new(store);
    assert!(access.read_point("no/such/point/here").await.is_err());
}

/// Regression: republishing an interval board (a save creates a new version with
/// a new row id) must *replace* its scheduler loop, not add another. Keyed by
/// the board's stable identity, the loop count stays at one — without this, every
/// save left the prior loop running and the board fired once per save.
#[tokio::test]
async fn republishing_a_board_does_not_leak_a_second_loop() {
    let (app, state) = TestApp::with_state();
    let scheduler = state.scheduler.as_ref().expect("scheduler");

    let body = |name: &str| {
        json!({
            "org": "nube",
            "slug": "pacer",
            "display_name": name,
            "trigger": {"kind": "interval", "seconds": 1},
            "board": {
                "nodes": [{"id": "t1", "component": "trigger",
                           "config": {"every": 1, "unit": "sec"}}],
                "connections": []
            }
        })
    };

    let (status, _) = app
        .request("POST", "/api/v1/boards?org=nube", Some(body("v1")))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(scheduler.active(), 1, "one loop after first publish");

    // Republish twice (each is a new version with a new row id).
    for name in ["v2", "v3"] {
        let (status, _) = app
            .request("POST", "/api/v1/boards?org=nube", Some(body(name)))
            .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(
            scheduler.active(),
            1,
            "republish replaces the loop rather than leaking another ({name})"
        );
    }

    // Disabling removes the loop entirely.
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/boards/pacer?org=nube",
            Some(json!({"enabled": false})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(scheduler.active(), 0, "disabled board has no loop");
}
