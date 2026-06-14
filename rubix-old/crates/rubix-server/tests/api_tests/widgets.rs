use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn create_and_list_widget() {
    let app = TestApp::new();
    let site = app.create_site().await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_value",
                "title": "AHU-3 fan", "target": "nube/hq/ahu-3/fan"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["kind"], "point_value");

    let (status, body) = app
        .request("GET", &format!("/api/v1/widgets?site_id={site}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["title"], "AHU-3 fan");
    assert_eq!(body[0]["target"], "nube/hq/ahu-3/fan");
}

#[tokio::test]
async fn get_and_delete_widget() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let (_, created) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_value",
                "title": "AHU-3 fan", "target": "nube/hq/ahu-3/fan"
            })),
        )
        .await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, body) = app
        .request("GET", &format!("/api/v1/widgets/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["title"], "AHU-3 fan");

    let (status, _) = app
        .request("DELETE", &format!("/api/v1/widgets/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = app
        .request("GET", &format!("/api/v1/widgets/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_missing_widget_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "DELETE",
            "/api/v1/widgets/00000000-0000-0000-0000-000000000000",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn widget_for_missing_site_is_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": "00000000-0000-0000-0000-000000000000",
                "kind": "board_output", "title": "t", "target": "b1"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn empty_title_is_400() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_value", "title": "  ", "target": "x"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// A `datasource` widget carries its native SQL in `query` (target is the
/// datasource id). It round-trips through create + get.
#[tokio::test]
async fn datasource_widget_round_trips_with_query() {
    let app = TestApp::new();
    let site = app.create_site().await;

    let (status, created) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "datasource",
                "title": "Historian daily", "target": "historian",
                "query": "SELECT time_bucket('1 day', ts) d, avg(v) FROM r GROUP BY 1"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["kind"], "datasource");
    assert_eq!(created["target"], "historian");
    assert!(created["query"].as_str().unwrap().contains("time_bucket"));

    let id = created["id"].as_str().unwrap();
    let (status, got) = app
        .request("GET", &format!("/api/v1/widgets/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{got}");
    assert_eq!(got["query"], created["query"]);
}

/// A tile's presentation `settings` (grid layout + chart config) round-trip
/// through PATCH and surface on the next GET/list; an explicit `null` clears
/// them.
#[tokio::test]
async fn patch_widget_settings_round_trips() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let (_, created) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_history",
                "title": "Demand", "target": "nube/hq/meter/kw"
            })),
        )
        .await;
    let id = created["id"].as_str().unwrap().to_string();
    assert!(created["settings"].is_null(), "fresh tile has no settings");

    // Set a grid cell + chart config.
    let (status, patched) = app
        .request(
            "PATCH",
            &format!("/api/v1/widgets/{id}"),
            Some(json!({
                "settings": {
                    "layout": { "x": 2, "y": 0, "w": 4, "h": 3 },
                    "config": { "type": "bar" }
                }
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_eq!(patched["settings"]["layout"]["w"], 4);
    assert_eq!(patched["settings"]["config"]["type"], "bar");

    // Persisted: a fresh GET sees them.
    let (_, got) = app
        .request("GET", &format!("/api/v1/widgets/{id}"), None)
        .await;
    assert_eq!(got["settings"]["layout"]["x"], 2);

    // An explicit null clears them back to the default rendering.
    let (status, cleared) = app
        .request(
            "PATCH",
            &format!("/api/v1/widgets/{id}"),
            Some(json!({ "settings": null })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(cleared["settings"].is_null());
}

/// PATCH against an unknown widget id is a 404.
#[tokio::test]
async fn patch_missing_widget_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/widgets/00000000-0000-0000-0000-000000000000",
            Some(json!({ "settings": { "layout": { "x": 0, "y": 0, "w": 1, "h": 1 } } })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// A `datasource` widget without `query` is a 400 — the SQL is its binding.
#[tokio::test]
async fn datasource_widget_without_query_is_400() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "datasource",
                "title": "Historian", "target": "historian"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// `query` on a non-datasource kind is a 400 — those carry their whole binding
/// in `target`, so a stray `query` is a malformed tile, not silently dropped.
#[tokio::test]
async fn query_on_point_widget_is_400() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_value",
                "title": "AHU-3 fan", "target": "nube/hq/ahu-3/fan",
                "query": "SELECT 1"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
