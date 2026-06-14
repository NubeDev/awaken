use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn list_orgs_groups_sites_by_org() {
    let app = TestApp::new();
    // Two sites under "kfc", one under "bk".
    app.create_site_with("kfc", "hq").await;
    app.create_site_with("kfc", "depot").await;
    app.create_site_with("bk", "hq").await;

    let (status, body) = app.request("GET", "/api/v1/orgs", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let orgs = body.as_array().unwrap();
    assert_eq!(orgs.len(), 2);

    // Sorted by org (BTreeMap): bk, then kfc.
    assert_eq!(orgs[0]["org"], "bk");
    assert_eq!(orgs[0]["site_count"], 1);
    assert_eq!(orgs[1]["org"], "kfc");
    assert_eq!(orgs[1]["site_count"], 2);
    let kfc_sites = orgs[1]["sites"].as_array().unwrap();
    assert_eq!(kfc_sites.len(), 2);
}

#[tokio::test]
async fn list_orgs_filters_by_org_query() {
    let app = TestApp::new();
    app.create_site_with("kfc", "hq").await;
    app.create_site_with("bk", "hq").await;

    let (status, body) = app.request("GET", "/api/v1/orgs?org=kfc", None).await;
    assert_eq!(status, StatusCode::OK);
    let orgs = body.as_array().unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0]["org"], "kfc");
}

#[tokio::test]
async fn provision_org_creates_first_site() {
    let app = TestApp::new();
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/orgs",
            Some(json!({
                "org": "wendys", "slug": "hq",
                "display_name": "Wendy's HQ", "tags": {"site": true}
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["org"], "wendys");
    assert_eq!(body["site_count"], 1);
    assert_eq!(body["sites"][0], "hq");

    // The provisioned tenant now shows up in the org list and the site list.
    let (_, orgs) = app.request("GET", "/api/v1/orgs?org=wendys", None).await;
    assert_eq!(orgs.as_array().unwrap().len(), 1);
    let (_, sites) = app.request("GET", "/api/v1/sites?org=wendys", None).await;
    assert_eq!(sites.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn provision_org_rejects_bad_slug() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/orgs",
            Some(json!({"org": "KFC Corp", "slug": "hq", "display_name": "X"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
