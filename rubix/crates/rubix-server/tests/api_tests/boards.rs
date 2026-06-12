//! Stored boards: CRUD + versioning over HTTP, run-by-slug, and the scheduler
//! firing an interval board over the store.

use std::time::Duration;

use axum::http::StatusCode;
use rubix_server::scheduler::{BoardRecord, Scheduler, Trigger};
use rubix_server::store::Store;
use serde_json::json;
use uuid::Uuid;

use super::harness::TestApp;

/// A minimal valid graph: read a point. Reused across the CRUD tests where the
/// graph content does not matter.
fn read_graph(point: &str) -> serde_json::Value {
    json!({
        "nodes": [{"id": "r1", "component": "read_point", "config": {"point": point}}],
        "connections": []
    })
}

#[tokio::test]
async fn create_get_list_delete_roundtrip() {
    let app = TestApp::new();

    let body = json!({
        "slug": "night-setback",
        "display_name": "Night setback",
        "trigger": {"kind": "interval", "seconds": 60},
        "board": read_graph("nube/hq/ahu-3/temp")
    });
    let (status, created) = app.request("POST", "/api/v1/boards", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["version"], json!(1));
    assert_eq!(created["enabled"], json!(true));

    // Get by slug returns the latest version.
    let (status, got) = app
        .request("GET", "/api/v1/boards/night-setback", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{got}");
    assert_eq!(got["slug"], json!("night-setback"));

    // List shows it once.
    let (status, list) = app.request("GET", "/api/v1/boards", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Delete, then it's gone.
    let (status, _) = app
        .request("DELETE", "/api/v1/boards/night-setback", None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app
        .request("GET", "/api/v1/boards/night-setback", None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn republish_increments_version_and_get_returns_latest() {
    let app = TestApp::new();
    let make = |seconds: u64| {
        json!({
            "slug": "loop", "display_name": "Loop",
            "trigger": {"kind": "interval", "seconds": seconds},
            "board": read_graph("nube/hq/ahu-3/temp")
        })
    };
    let (s1, v1) = app.request("POST", "/api/v1/boards", Some(make(30))).await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(v1["version"], json!(1));
    let (s2, v2) = app.request("POST", "/api/v1/boards", Some(make(45))).await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(v2["version"], json!(2));

    // GET resolves to v2; list still shows the slug once (latest only).
    let (_, got) = app.request("GET", "/api/v1/boards/loop", None).await;
    assert_eq!(got["version"], json!(2));
    assert_eq!(got["trigger"]["seconds"], json!(45));
    let (_, list) = app.request("GET", "/api/v1/boards", None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn zero_interval_rejected() {
    let app = TestApp::new();
    let body = json!({
        "slug": "bad", "display_name": "Bad",
        "trigger": {"kind": "interval", "seconds": 0},
        "board": read_graph("nube/hq/ahu-3/temp")
    });
    let (status, _) = app.request("POST", "/api/v1/boards", Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn run_stored_board_commands_a_point() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let temp = app.create_point(&equip, "sensor", "temp").await;
    let _fan = app.create_point(&equip, "cmd", "fan").await;

    // Seed the sensor.
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{temp}/cur"),
            Some(json!({"value": 19.0})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Store a manual board that reads temp → commands fan at prio 8.
    let board = json!({
        "slug": "copy-temp", "display_name": "Copy temp to fan",
        "trigger": {"kind": "manual"},
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
    let (status, _) = app.request("POST", "/api/v1/boards", Some(board)).await;
    assert_eq!(status, StatusCode::CREATED);

    // Run it by slug; the write reaches the store.
    let (status, body) = app
        .request("POST", "/api/v1/boards/copy-temp/run", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, fan) = app
        .request("GET", &format!("/api/v1/points/{_fan}"), None)
        .await;
    assert_eq!(fan["point"]["cur_value"], json!(19.0));
}

/// The scheduler's interval loop runs a stored board over the store: a 1s
/// interval board that copies a sensor to a command point, observed taking
/// effect within a couple of ticks. Drives the loop directly (no HTTP).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduler_interval_fires_a_board() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("sched.db")).expect("store");

    // Provision a sensor + command point and seed the sensor.
    let app = TestApp::with_store_at(store.clone());
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let temp = app.create_point(&equip, "sensor", "temp").await;
    let fan = app.create_point(&equip, "cmd", "fan").await;
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{temp}/cur"),
            Some(json!({"value": 42.0})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Persist a 1s interval board copying temp → fan.
    let record = BoardRecord {
        id: Uuid::new_v4(),
        slug: "tick".into(),
        version: 1,
        display_name: "Tick".into(),
        enabled: true,
        trigger: Trigger::Interval { seconds: 1 },
        graph: serde_json::from_value(json!({
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
        }))
        .unwrap(),
        created_at: chrono::Utc::now(),
    };
    store.create_board(&record).expect("store board");

    let scheduler = Scheduler::launch(store.clone(), None, None, vec![record]);
    assert_eq!(scheduler.active(), 1);

    // Within ~3s (first tick lands at 1s) the board should have commanded fan.
    let mut commanded = false;
    for _ in 0..30 {
        let (_, fan_body) = app
            .request("GET", &format!("/api/v1/points/{fan}"), None)
            .await;
        if fan_body["point"]["cur_value"] == json!(42.0) {
            commanded = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    scheduler.shutdown().await;
    assert!(commanded, "scheduled board did not command the point");
}

/// A board with an `emit_spark` node records a finding via the store-backed
/// `PointAccess::emit_spark`. Run by slug; the spark then lists under the site.
#[tokio::test]
async fn emit_spark_board_records_a_finding() {
    let app = TestApp::new();
    let site = app.create_site().await;

    let board = json!({
        "slug": "ahu-conflict", "display_name": "AHU heat/cool conflict",
        "trigger": {"kind": "manual"},
        "board": {
            "nodes": [{
                "id": "s1", "component": "emit_spark",
                "config": {
                    "site": "nube/hq", "rule": "heat_cool_conflict",
                    "severity": "fault",
                    "message": "simultaneous heat and cool on AHU-3"
                }
            }],
            "connections": []
        }
    });
    let (status, _) = app.request("POST", "/api/v1/boards", Some(board)).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = app
        .request("POST", "/api/v1/boards/ahu-conflict/run", None)
        .await;
    assert_eq!(status, StatusCode::OK);

    // The finding was persisted and lists under the site.
    let (status, sparks) = app
        .request("GET", &format!("/api/v1/sparks?site_id={site}"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{sparks}");
    let arr = sparks.as_array().expect("sparks array");
    assert_eq!(arr.len(), 1, "{sparks}");
    assert_eq!(arr[0]["rule"], json!("heat_cool_conflict"));
    assert_eq!(arr[0]["severity"], json!("fault"));
    assert_eq!(arr[0]["message"], json!("simultaneous heat and cool on AHU-3"));
}
