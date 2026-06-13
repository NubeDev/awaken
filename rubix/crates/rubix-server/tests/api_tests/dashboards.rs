use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn create_list_get_patch_delete_site_dashboard() {
    let app = TestApp::new();
    let site = app.create_site_with("kfc", "hq").await;

    // Create a site-scoped board.
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/dashboards",
            Some(json!({"org": "kfc", "site_id": site, "slug": "energy", "title": "Energy"})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["site_id"], site);
    assert_eq!(body["slug"], "energy");

    // List by org returns it.
    let (status, list) = app.request("GET", "/api/v1/dashboards?org=kfc", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Filter by site.
    let (_, list) = app
        .request(
            "GET",
            &format!("/api/v1/dashboards?org=kfc&site_id={site}"),
            None,
        )
        .await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Get by id.
    let (status, got) = app
        .request("GET", &format!("/api/v1/dashboards/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["title"], "Energy");

    // Patch the title; slug/site unchanged.
    let (status, patched) = app
        .request(
            "PATCH",
            &format!("/api/v1/dashboards/{id}"),
            Some(json!({"title": "Energy Overview"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["title"], "Energy Overview");
    assert_eq!(patched["slug"], "energy");

    // Delete.
    let (status, _) = app
        .request("DELETE", &format!("/api/v1/dashboards/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app
        .request("GET", &format!("/api/v1/dashboards/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_org_overview_dashboard_has_no_site() {
    let app = TestApp::new();
    app.create_site_with("kfc", "hq").await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/dashboards",
            Some(json!({"org": "kfc", "slug": "portfolio", "title": "Portfolio"})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body.get("site_id").is_none() || body["site_id"].is_null());
    assert_eq!(body["slug"], "portfolio");
}

#[tokio::test]
async fn duplicate_slug_in_same_scope_conflicts() {
    let app = TestApp::new();
    let site = app.create_site_with("kfc", "hq").await;
    let make = json!({"org": "kfc", "site_id": site, "slug": "energy", "title": "E"});
    let (s1, _) = app
        .request("POST", "/api/v1/dashboards", Some(make.clone()))
        .await;
    assert_eq!(s1, StatusCode::CREATED);
    let (s2, _) = app.request("POST", "/api/v1/dashboards", Some(make)).await;
    assert_eq!(s2, StatusCode::CONFLICT);
}

#[tokio::test]
async fn widget_pins_to_a_dashboard_and_lists_by_it() {
    let app = TestApp::new();
    let site = app.create_site_with("kfc", "hq").await;
    let (_, dash) = app
        .request(
            "POST",
            "/api/v1/dashboards",
            Some(json!({"org": "kfc", "site_id": site, "slug": "energy", "title": "Energy"})),
        )
        .await;
    let dashboard_id = dash["id"].as_str().unwrap().to_string();

    let (status, w) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "dashboard_id": dashboard_id, "site_id": site,
                "kind": "point_value", "title": "Fan", "target": "kfc/hq/ahu-3/fan"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{w}");
    assert_eq!(w["dashboard_id"], dashboard_id);

    // List filtered by dashboard.
    let (status, list) = app
        .request(
            "GET",
            &format!("/api/v1/widgets?dashboard_id={dashboard_id}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"], "Fan");
}

#[tokio::test]
async fn widget_without_dashboard_lands_on_site_default() {
    let app = TestApp::new();
    let site = app.create_site_with("kfc", "hq").await;

    // Legacy path: no dashboard_id → server creates/uses the site's default.
    let (status, w) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_value",
                "title": "Fan", "target": "kfc/hq/ahu-3/fan"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{w}");
    assert!(w["dashboard_id"].as_str().is_some());

    // That default dashboard is now listable under the org.
    let (_, list) = app.request("GET", "/api/v1/dashboards?org=kfc", None).await;
    let slugs: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["slug"].as_str().unwrap())
        .collect();
    assert!(
        slugs.contains(&"default"),
        "default dashboard exists: {list}"
    );
}
