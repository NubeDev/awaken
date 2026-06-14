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
        "org": "nube",
        "slug": "night-setback",
        "display_name": "Night setback",
        "trigger": {"kind": "interval", "seconds": 60},
        "board": read_graph("nube/hq/ahu-3/temp")
    });
    let (status, created) = app.request("POST", "/api/v1/boards?org=nube", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["version"], json!(1));
    assert_eq!(created["enabled"], json!(true));

    // Get by slug returns the latest version.
    let (status, got) = app
        .request("GET", "/api/v1/boards/night-setback?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{got}");
    assert_eq!(got["slug"], json!("night-setback"));

    // List shows it once.
    let (status, list) = app.request("GET", "/api/v1/boards?org=nube", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Delete, then it's gone.
    let (status, _) = app
        .request("DELETE", "/api/v1/boards/night-setback?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app
        .request("GET", "/api/v1/boards/night-setback?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn republish_increments_version_and_get_returns_latest() {
    let app = TestApp::new();
    let make = |seconds: u64| {
        json!({
            "org": "nube",
        "slug": "loop", "display_name": "Loop",
            "trigger": {"kind": "interval", "seconds": seconds},
            "board": read_graph("nube/hq/ahu-3/temp")
        })
    };
    let (s1, v1) = app.request("POST", "/api/v1/boards?org=nube", Some(make(30))).await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(v1["version"], json!(1));
    let (s2, v2) = app.request("POST", "/api/v1/boards?org=nube", Some(make(45))).await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(v2["version"], json!(2));

    // GET resolves to v2; list still shows the slug once (latest only).
    let (_, got) = app.request("GET", "/api/v1/boards/loop?org=nube", None).await;
    assert_eq!(got["version"], json!(2));
    assert_eq!(got["trigger"]["seconds"], json!(45));
    let (_, list) = app.request("GET", "/api/v1/boards?org=nube", None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn patch_board_edits_metadata_on_latest_version() {
    let app = TestApp::new();
    let create = json!({
        "org": "nube",
        "slug": "setback", "display_name": "Setback",
        "trigger": {"kind": "interval", "seconds": 60},
        "board": read_graph("nube/hq/ahu-3/temp")
    });
    let (status, _) = app.request("POST", "/api/v1/boards?org=nube", Some(create)).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/boards/setback?org=nube",
            Some(json!({"display_name": "Night setback", "enabled": false})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["display_name"], "Night setback");
    assert_eq!(body["enabled"], json!(false));
    assert_eq!(body["version"], json!(1));

    // Persisted: a fresh GET reflects the patch.
    let (_, got) = app.request("GET", "/api/v1/boards/setback?org=nube", None).await;
    assert_eq!(got["display_name"], "Night setback");
    assert_eq!(got["enabled"], json!(false));
}

#[tokio::test]
async fn patch_missing_board_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/boards/nope?org=nube",
            Some(json!({"enabled": false})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn zero_interval_rejected() {
    let app = TestApp::new();
    let body = json!({
        "org": "nube",
        "slug": "bad", "display_name": "Bad",
        "trigger": {"kind": "interval", "seconds": 0},
        "board": read_graph("nube/hq/ahu-3/temp")
    });
    let (status, _) = app.request("POST", "/api/v1/boards?org=nube", Some(body)).await;
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
        "org": "nube",
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
    let (status, _) = app.request("POST", "/api/v1/boards?org=nube", Some(board)).await;
    assert_eq!(status, StatusCode::CREATED);

    // Run it by slug; the write reaches the store.
    let (status, body) = app
        .request("POST", "/api/v1/boards/copy-temp/run?org=nube", None)
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
        org: "nube".into(),
        site_id: None,
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

    let scheduler = Scheduler::launch(store.clone(), None, None, None, vec![record]);
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
        "org": "nube",
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
    let (status, _) = app.request("POST", "/api/v1/boards?org=nube", Some(board)).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = app
        .request("POST", "/api/v1/boards/ahu-conflict/run?org=nube", None)
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
    assert_eq!(
        arr[0]["message"],
        json!("simultaneous heat and cool on AHU-3")
    );
}

/// The component catalogue lists every built-in node with its ports and config
/// schema, so the editor can render a config form without hardcoding fields.
#[tokio::test]
async fn components_catalogue_exposes_schema() {
    let app = TestApp::new();
    let (status, components) = app.request("GET", "/api/v1/boards/components?org=nube", None).await;
    assert_eq!(status, StatusCode::OK, "{components}");

    let arr = components.as_array().expect("components array");

    // Every built-in is catalogued (the registry-coverage unit test in
    // rubix-flow pins the exact set; here we just confirm the known nodes
    // surface over HTTP, so adding a component doesn't churn this assertion).
    let names: Vec<&str> = arr
        .iter()
        .map(|c| c["component"].as_str().unwrap())
        .collect();
    for expected in [
        "read_point",
        "write_point",
        "query_his",
        "rule",
        "trigger",
        "agent_call",
        "emit_spark",
    ] {
        assert!(
            names.contains(&expected),
            "missing {expected} in {components}"
        );
    }

    // write_point carries a typed, bounded `priority` field with a default —
    // exactly the schema the editor needs to render a control, not a guess.
    let write = arr
        .iter()
        .find(|c| c["component"] == "write_point")
        .unwrap();
    let priority = write["config"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "priority")
        .expect("priority field");
    assert_eq!(priority["field_type"], json!("integer"));
    assert_eq!(priority["required"], json!(false));
    assert_eq!(priority["default"], json!(16));
    assert_eq!(priority["min"], json!(1.0));
    assert_eq!(priority["max"], json!(16.0));

    // Ports carry a type so the editor can validate connections: read_point's
    // `output` is a scalar, its `error` is the terminal error class.
    let read = arr.iter().find(|c| c["component"] == "read_point").unwrap();
    let read_out = read["outports"].as_array().unwrap();
    assert_eq!(
        read_out.iter().find(|p| p["id"] == "output").unwrap()["port_type"],
        json!("scalar")
    );
    assert_eq!(
        read_out.iter().find(|p| p["id"] == "error").unwrap()["port_type"],
        json!("error")
    );

    // emit_spark.severity is an enum with options — drives a select control.
    let emit = arr.iter().find(|c| c["component"] == "emit_spark").unwrap();
    let severity = emit["config"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "severity")
        .expect("severity field");
    assert_eq!(severity["field_type"], json!("enum"));
    assert_eq!(severity["options"], json!(["info", "warning", "fault"]));
}

/// A new flow is created empty (no nodes/connections) with a manual trigger —
/// the path the editor's "New flow" action takes before any node is dragged in.
#[tokio::test]
async fn create_empty_manual_board() {
    let app = TestApp::new();
    let body = json!({
        "org": "nube",
        "slug": "blank-flow",
        "display_name": "Blank flow",
        "trigger": {"kind": "manual"},
        "board": {"nodes": [], "connections": []}
    });
    let (status, created) = app.request("POST", "/api/v1/boards?org=nube", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["version"], json!(1));
    assert_eq!(created["graph"]["nodes"], json!([]));
}

/// A board run's per-node outputs surface on the live-outputs endpoint, so a
/// client can see the values an enabled board produces without re-running it.
/// Uses a `read_point` board over a seeded point for a deterministic output.
#[tokio::test]
async fn board_outputs_endpoint_exposes_last_run_values() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let temp = app.create_point(&equip, "sensor", "temp").await;
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{temp}/cur"),
            Some(json!({"value": 21.5})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let board = json!({
        "org": "nube",
        "slug": "read-temp", "display_name": "Read temp",
        "trigger": {"kind": "manual"},
        "board": {
            "nodes": [{"id": "r1", "component": "read_point",
                       "config": {"point": "nube/hq/ahu-3/temp"}}],
            "connections": []
        }
    });
    let (status, _) = app.request("POST", "/api/v1/boards?org=nube", Some(board)).await;
    assert_eq!(status, StatusCode::CREATED);

    // Before any run, outputs are empty (board has not produced anything).
    let (status, before) = app
        .request("GET", "/api/v1/boards/read-temp/outputs?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{before}");
    assert_eq!(before.as_array().unwrap().len(), 0);

    // Run it; read_point emits the point's value on `output`.
    let (status, run) = app
        .request("POST", "/api/v1/boards/read-temp/run?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{run}");

    // The run's output is now readable on the outputs endpoint, keyed by node
    // and port, with the value and a capture timestamp.
    let (status, after) = app
        .request("GET", "/api/v1/boards/read-temp/outputs?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    let arr = after.as_array().unwrap();
    let output = arr
        .iter()
        .find(|o| o["node"] == "r1" && o["port"] == "output")
        .expect(&format!("read_point output present: {after}"));
    assert_eq!(output["value"], json!(21.5));
    assert!(output["at"].is_string(), "carries a capture timestamp");
}

/// Deleting a board clears its cached outputs, so a later board reusing the
/// slug never shows the old one's values.
#[tokio::test]
async fn deleting_a_board_clears_its_outputs() {
    let app = TestApp::new();
    let board = json!({
        "org": "nube",
        "slug": "ephemeral", "display_name": "Ephemeral",
        "trigger": {"kind": "manual"},
        "board": {
            "nodes": [{"id": "t1", "component": "trigger", "config": {}}],
            "connections": []
        }
    });
    app.request("POST", "/api/v1/boards?org=nube", Some(board)).await;
    app.request("POST", "/api/v1/boards/ephemeral/run?org=nube", None).await;
    let (_, after) = app
        .request("GET", "/api/v1/boards/ephemeral/outputs?org=nube", None)
        .await;
    assert!(!after.as_array().unwrap().is_empty());

    let (status, _) = app
        .request("DELETE", "/api/v1/boards/ephemeral?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, cleared) = app
        .request("GET", "/api/v1/boards/ephemeral/outputs?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cleared.as_array().unwrap().len(), 0, "outputs cleared on delete");
}

#[tokio::test]
async fn options_points_returns_keyexpr_value_and_display_label() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    app.create_point(&equip, "cmd", "fan").await;

    let (status, body) = app
        .request("GET", "/api/v1/boards/options/points?org=nube&site=hq", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let opts = body.as_array().expect("array");
    let fan = opts
        .iter()
        .find(|o| o["value"] == json!("nube/hq/ahu-3/fan"))
        .expect("fan point present as a keyexpr-valued option");
    assert_eq!(fan["label"], json!("fan"), "label is the display name");
}

#[tokio::test]
async fn options_points_scopes_to_org_and_site() {
    let app = TestApp::new();
    let hq = app.create_site_with("nube", "hq").await;
    let dc = app.create_site_with("nube", "dc").await;
    app.create_point(&app.create_equip(&hq).await, "cmd", "fan").await;
    app.create_point(&app.create_equip(&dc).await, "cmd", "pump").await;

    // Site-scoped: only hq's point.
    let (_, hq_only) = app
        .request("GET", "/api/v1/boards/options/points?org=nube&site=hq", None)
        .await;
    let values: Vec<_> = hq_only
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["value"].as_str().unwrap().to_string())
        .collect();
    assert!(values.iter().any(|v| v.contains("/hq/")));
    assert!(!values.iter().any(|v| v.contains("/dc/")), "dc excluded by site scope");

    // Org-wide: both sites' points.
    let (_, org_wide) = app
        .request("GET", "/api/v1/boards/options/points?org=nube", None)
        .await;
    assert_eq!(org_wide.as_array().unwrap().len(), 2, "org scope returns both points");
}

#[tokio::test]
async fn options_sites_lists_org_site_prefixes() {
    let app = TestApp::new();
    app.create_site_with("nube", "hq").await;

    let (status, body) = app
        .request("GET", "/api/v1/boards/options/sites?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let hq = body
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["value"] == json!("nube/hq"))
        .expect("`{org}/{site}` prefix is the option value");
    assert_eq!(hq["label"], json!("hq"));
}

#[tokio::test]
async fn options_unknown_source_is_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request("GET", "/api/v1/boards/options/bogus?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn options_scoped_source_without_org_is_400() {
    let app = TestApp::new();
    let (status, _) = app.request("GET", "/api/v1/boards/options/points", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn options_datasources_empty_without_registry() {
    let app = TestApp::new();
    let (status, body) = app
        .request("GET", "/api/v1/boards/options/datasources?org=nube", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body.as_array().unwrap().len(), 0, "no registry → empty, not error");
}
