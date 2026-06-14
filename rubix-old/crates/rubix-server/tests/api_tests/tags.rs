//! Entity-tag API: full-replace PUT / GET, reverse lookup, key listing, the
//! entity-own authz (edit to write, view to read), rejection of unknown/foreign
//! ids, the board-delete sweep, and the injection boundary (free-form tag values
//! bind as parameters). See docs/design/page-context-and-nav.md §3.

use axum::http::StatusCode;
use chrono::Utc;
use rubix_core::Dashboard;
use rubix_server::auth::{pat, AdminLevel, Role, Scope, TokenRecord};
use rubix_server::store::{Store, UserRecord};
use uuid::Uuid;

use super::harness::TestApp;

fn seed_dashboard(store: &Store, org: &str, slug: &str) -> Uuid {
    let id = Uuid::new_v4();
    store
        .create_dashboard(&Dashboard {
            id,
            org: org.into(),
            site_id: None,
            slug: slug.into(),
            title: slug.into(),
            variables: Vec::new(),
            created_at: Utc::now(),
        })
        .expect("seed dashboard");
    id
}

/// Mint a read-only (Viewer) PAT for `org`.
fn seed_viewer_pat(store: &Store, org: &str) -> String {
    let minted = pat::mint();
    store
        .create_token(&TokenRecord {
            id: minted.id.clone(),
            secret_hash: minted.secret_hash,
            name: "seed".into(),
            role: Role::Viewer,
            scope: Scope::org(org),
            created_at: Utc::now(),
            revoked_at: None,
        })
        .expect("seed token");
    store
        .create_user(&UserRecord {
            id: Uuid::new_v4(),
            org: org.into(),
            subject: minted.id.clone(),
            email: format!("{}@{org}.test", minted.id),
            display_name: "viewer".into(),
            admin_level: AdminLevel::None,
            created_at: Utc::now(),
        })
        .expect("seed user");
    minted.plaintext
}

#[tokio::test]
async fn put_get_replace_and_reverse_lookup() {
    let (app, store) = TestApp::with_store();
    let dash = seed_dashboard(&store, "kfc", "ops");

    // PUT a full set.
    let (status, body) = app
        .request(
            "PUT",
            &format!("/api/v1/tags/dashboard/{dash}"),
            Some(serde_json::json!({"building": "hq", "floor": "3"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["building"], "hq");

    // GET reads it back.
    let (status, got) = app
        .request("GET", &format!("/api/v1/tags/dashboard/{dash}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["floor"], "3");

    // PUT again replaces wholesale (floor drops out).
    let (status, _) = app
        .request(
            "PUT",
            &format!("/api/v1/tags/dashboard/{dash}"),
            Some(serde_json::json!({"building": "annex"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, got) = app
        .request("GET", &format!("/api/v1/tags/dashboard/{dash}"), None)
        .await;
    assert_eq!(got["building"], "annex");
    assert!(got.get("floor").is_none(), "replaced set drops floor: {got}");

    // Reverse lookup: which dashboards carry tags.
    let (status, entities) = app
        .request("GET", "/api/v1/tags/entities/dashboard?org=kfc", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{entities}");
    assert_eq!(entities[dash.to_string()]["building"], "annex");

    // Key listing.
    let (status, keys) = app
        .request("GET", "/api/v1/tags/keys?org=kfc&kind=dashboard", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(keys.as_array().unwrap(), &vec![serde_json::json!("building")]);
}

#[tokio::test]
async fn unknown_kind_is_404() {
    let app = TestApp::new();
    let id = Uuid::new_v4();
    let (status, _) = app
        .request("GET", &format!("/api/v1/tags/widget/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn nonexistent_id_is_404() {
    let app = TestApp::new();
    let id = Uuid::new_v4();
    let (status, _) = app
        .request(
            "PUT",
            &format!("/api/v1/tags/dashboard/{id}"),
            Some(serde_json::json!({"a": "b"})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn viewer_cannot_write_tags() {
    let (app, store) = TestApp::with_auth();
    let dash = seed_dashboard(&store, "kfc", "ops");
    let viewer = seed_viewer_pat(&store, "kfc");

    // Viewer can read (org-scope read), but not write the entity's tags.
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/tags/dashboard/{dash}"), &viewer, None)
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = app
        .request_as(
            "PUT",
            &format!("/api/v1/tags/dashboard/{dash}"),
            &viewer,
            Some(serde_json::json!({"x": "y"})),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "viewer cannot edit tags");
}

#[tokio::test]
async fn board_delete_sweeps_tags_and_nodes() {
    let app = TestApp::new();
    // Create a dashboard through the API (gives a deletable id with proper org).
    let (status, dash) = app
        .request(
            "POST",
            "/api/v1/dashboards",
            Some(serde_json::json!({
                "org": "kfc", "slug": "ops", "title": "Ops", "variables": []
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{dash}");
    let id = dash["id"].as_str().unwrap().to_string();

    // Tag it, and mount a nav node on it.
    app.request(
        "PUT",
        &format!("/api/v1/tags/dashboard/{id}"),
        Some(serde_json::json!({"building": "hq"})),
    )
    .await;
    let (_, node) = app
        .request(
            "POST",
            "/api/v1/nav",
            Some(serde_json::json!({
                "org": "kfc", "title": "Ops Board",
                "target": {"kind": "dashboard", "dashboard_id": id}
            })),
        )
        .await;
    let node_id = node["id"].as_str().unwrap().to_string();

    // Delete the dashboard.
    let (status, _) = app
        .request("DELETE", &format!("/api/v1/dashboards/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Tags swept: reverse lookup no longer carries the id.
    let (_, entities) = app
        .request("GET", "/api/v1/tags/entities/dashboard?org=kfc", None)
        .await;
    assert!(entities.get(&id).is_none(), "tags swept on delete: {entities}");

    // The dependent node was swept to a group (no dangling dashboard target).
    let (status, swept) = app
        .request("GET", &format!("/api/v1/nav/{node_id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{swept}");
    assert_eq!(swept["target"]["kind"], "group", "node swept to group: {swept}");
}

#[tokio::test]
async fn injection_shaped_tag_value_binds_not_executes() {
    let (app, store) = TestApp::with_store();
    let dash = seed_dashboard(&store, "kfc", "ops");
    let evil = "hq'); DROP TABLE entity_tags;--";
    let (status, body) = app
        .request(
            "PUT",
            &format!("/api/v1/tags/dashboard/{dash}"),
            Some(serde_json::json!({"building": evil})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["building"], evil);
    // Table intact: read back the verbatim value.
    let (_, got) = app
        .request("GET", &format!("/api/v1/tags/dashboard/{dash}"), None)
        .await;
    assert_eq!(got["building"], evil);
}
