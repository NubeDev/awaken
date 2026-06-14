//! Nav-tree API: CRUD, nesting/reorder/reparent, org isolation, cross-org
//! dashboard-target rejection, the `nav_node` grant view-filter, default-tree
//! seeding on org provision, and the injection boundary (free-form titles bind
//! as parameters, never execute). See docs/design/page-context-and-nav.md.

use axum::http::StatusCode;
use chrono::Utc;
use rubix_core::{Dashboard, NavNode, NavTarget};
use rubix_server::auth::{pat, AdminLevel, Role, Scope, TokenRecord};
use rubix_server::store::{GrantRecord, Permission, Store, SubjectKind, UserRecord};
use uuid::Uuid;

use super::harness::TestApp;

/// Mint a PAT resolving to a freshly-seeded user with `scope`, optionally
/// holding `nav_node:<id>` read grants in `grant_org`. Returns the bearer.
fn seed_viewer_pat(
    store: &Store,
    org: &str,
    scope: Scope,
    grant_org: &str,
    node_grants: &[Uuid],
) -> String {
    let minted = pat::mint();
    let user_id = Uuid::new_v4();
    store
        .create_token(&TokenRecord {
            id: minted.id.clone(),
            secret_hash: minted.secret_hash,
            name: "seed".into(),
            role: Role::Viewer,
            scope,
            created_at: Utc::now(),
            revoked_at: None,
        })
        .expect("seed token");
    store
        .create_user(&UserRecord {
            id: user_id,
            org: org.into(),
            subject: minted.id.clone(),
            email: format!("{}@{org}.test", minted.id),
            display_name: "seed user".into(),
            admin_level: AdminLevel::None,
            created_at: Utc::now(),
        })
        .expect("seed user");
    for node in node_grants {
        store
            .create_grant(&GrantRecord {
                id: Uuid::new_v4(),
                org: grant_org.into(),
                subject_kind: SubjectKind::User,
                subject_id: user_id.to_string(),
                resource_kind: "nav_node".into(),
                resource_ref: format!("nav_node:{node}"),
                permission: Permission::Read,
                created_at: Utc::now(),
            })
            .expect("seed grant");
    }
    minted.plaintext
}

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

/// Seed a root group node directly in the store; returns its id.
fn seed_nav_group(store: &Store, org: &str, title: &str) -> Uuid {
    let id = Uuid::new_v4();
    store
        .create_nav_node(&NavNode {
            id,
            org: org.into(),
            parent_id: None,
            title: title.into(),
            sort_order: 0,
            target: NavTarget::Group,
            context: None,
            icon: None,
            accent: None,
        })
        .expect("seed nav node");
    id
}

/// Create a node over the unauthenticated edge path; returns its id.
async fn create_node(app: &TestApp, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    app.request("POST", "/api/v1/nav", Some(body)).await
}

#[tokio::test]
async fn nav_crud_nest_reorder_reparent() {
    let app = TestApp::new();

    // Root group.
    let (status, root) = create_node(
        &app,
        serde_json::json!({
            "org": "kfc", "title": "Pages", "sort_order": 0,
            "target": {"kind": "group"}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{root}");
    let root_id = root["id"].as_str().unwrap().to_string();

    // A route child under root.
    let (status, child) = create_node(
        &app,
        serde_json::json!({
            "org": "kfc", "parent_id": root_id, "title": "Sites", "sort_order": 0,
            "target": {"kind": "route", "route": "sites"}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{child}");
    let child_id = child["id"].as_str().unwrap().to_string();
    assert_eq!(child["parent_id"], root_id);

    // A second root group to reparent the child under, plus reorder.
    let (_, root2) = create_node(
        &app,
        serde_json::json!({
            "org": "kfc", "title": "More", "sort_order": 1,
            "target": {"kind": "group"}
        }),
    )
    .await;
    let root2_id = root2["id"].as_str().unwrap().to_string();

    // GET one.
    let (status, got) = app
        .request("GET", &format!("/api/v1/nav/{child_id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{got}");
    assert_eq!(got["title"], "Sites");

    // PATCH: rename + reorder + reparent under root2.
    let (status, patched) = app
        .request(
            "PATCH",
            &format!("/api/v1/nav/{child_id}"),
            Some(serde_json::json!({
                "title": "Sites & Floors", "sort_order": 5, "parent_id": root2_id
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_eq!(patched["title"], "Sites & Floors");
    assert_eq!(patched["sort_order"], 5);
    assert_eq!(patched["parent_id"], root2_id);

    // PATCH parent_id: null → back to root.
    let (status, rooted) = app
        .request(
            "PATCH",
            &format!("/api/v1/nav/{child_id}"),
            Some(serde_json::json!({ "parent_id": null })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{rooted}");
    assert!(rooted["parent_id"].is_null());

    // List: 3 nodes for kfc.
    let (status, list) = app.request("GET", "/api/v1/nav?org=kfc", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 3);

    // DELETE the child; gone afterward.
    let (status, _) = app
        .request("DELETE", &format!("/api/v1/nav/{child_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app
        .request("GET", &format!("/api/v1/nav/{child_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_parent_cascades_children() {
    let app = TestApp::new();
    let (_, root) = create_node(
        &app,
        serde_json::json!({"org": "kfc", "title": "Pages", "target": {"kind": "group"}}),
    )
    .await;
    let root_id = root["id"].as_str().unwrap().to_string();
    let (_, child) = create_node(
        &app,
        serde_json::json!({
            "org": "kfc", "parent_id": root_id, "title": "Sites",
            "target": {"kind": "route", "route": "sites"}
        }),
    )
    .await;
    let child_id = child["id"].as_str().unwrap().to_string();

    let (status, _) = app
        .request("DELETE", &format!("/api/v1/nav/{root_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    // The child cascaded away with its parent.
    let (status, _) = app
        .request("GET", &format!("/api/v1/nav/{child_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn node_cannot_parent_itself() {
    let app = TestApp::new();
    let (_, node) = create_node(
        &app,
        serde_json::json!({"org": "kfc", "title": "Pages", "target": {"kind": "group"}}),
    )
    .await;
    let id = node["id"].as_str().unwrap().to_string();
    let (status, _) = app
        .request(
            "PATCH",
            &format!("/api/v1/nav/{id}"),
            Some(serde_json::json!({ "parent_id": id })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn org_isolation_lists_only_own_org() {
    let app = TestApp::new();
    create_node(
        &app,
        serde_json::json!({"org": "kfc", "title": "K", "target": {"kind": "group"}}),
    )
    .await;
    create_node(
        &app,
        serde_json::json!({"org": "acme", "title": "A", "target": {"kind": "group"}}),
    )
    .await;

    let (_, kfc) = app.request("GET", "/api/v1/nav?org=kfc", None).await;
    assert_eq!(kfc.as_array().unwrap().len(), 1);
    assert_eq!(kfc[0]["title"], "K");
    let (_, acme) = app.request("GET", "/api/v1/nav?org=acme", None).await;
    assert_eq!(acme.as_array().unwrap().len(), 1);
    assert_eq!(acme[0]["title"], "A");
}

#[tokio::test]
async fn cross_org_dashboard_target_rejected() {
    let (app, store) = TestApp::with_store();
    // A dashboard owned by acme.
    let dash = seed_dashboard(&store, "acme", "ops");
    // kfc cannot mount acme's board — a 404 hides existence and org.
    let (status, _) = create_node(
        &app,
        serde_json::json!({
            "org": "kfc", "title": "Stolen",
            "target": {"kind": "dashboard", "dashboard_id": dash.to_string()}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The owning org mounts it fine.
    let (status, _) = create_node(
        &app,
        serde_json::json!({
            "org": "acme", "title": "Ops",
            "target": {"kind": "dashboard", "dashboard_id": dash.to_string()}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn parent_must_be_same_org() {
    let app = TestApp::new();
    let (_, acme_root) = create_node(
        &app,
        serde_json::json!({"org": "acme", "title": "A", "target": {"kind": "group"}}),
    )
    .await;
    let acme_id = acme_root["id"].as_str().unwrap().to_string();
    // kfc node naming an acme parent → 404 (parent not visible in kfc).
    let (status, _) = create_node(
        &app,
        serde_json::json!({
            "org": "kfc", "parent_id": acme_id, "title": "X", "target": {"kind": "group"}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn tree_get_filtered_to_view_grants() {
    let (app, store) = TestApp::with_auth();
    // Two nodes in kfc, seeded directly (the enforced path needs a writer token;
    // this test exercises only the read filter).
    let a_id = seed_nav_group(&store, "kfc", "Visible");
    let b_id = seed_nav_group(&store, "kfc", "Hidden");

    // A viewer whose scope is a *different* org (no Layer-1 read of kfc), but
    // holding a nav_node read grant on node A in kfc. Only A reaches the wire.
    let bearer = seed_viewer_pat(
        &store,
        "other",
        Scope::org("other"),
        "kfc",
        &[a_id],
    );

    let (status, list) = app
        .request_as("GET", "/api/v1/nav?org=kfc", &bearer, None)
        .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 1, "only the granted node is visible: {list}");
    assert_eq!(arr[0]["title"], "Visible");

    // Opening the ungranted node B is a 404 (existence hidden).
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/nav/{b_id}"), &bearer, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn default_tree_seeded_on_org_provision() {
    let app = TestApp::new();
    // Provision a fresh org via the orgs endpoint.
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/orgs",
            Some(serde_json::json!({
                "org": "kfc", "slug": "hq", "display_name": "KFC HQ", "tags": {}
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, list) = app.request("GET", "/api/v1/nav?org=kfc", None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = list.as_array().unwrap();
    // One "Pages" root group + one node per built-in route (11) = 12.
    assert_eq!(arr.len(), 12, "seeded tree: {list}");
    assert!(arr.iter().any(|n| n["title"] == "Pages"));
    assert!(arr.iter().any(|n| n["title"] == "Sites"));
}

#[tokio::test]
async fn injection_shaped_title_binds_not_executes() {
    let app = TestApp::new();
    // A title carrying a SQL drop attempt must persist verbatim and inert.
    let evil = "Pages'); DROP TABLE nav_nodes;--";
    let (status, node) = create_node(
        &app,
        serde_json::json!({"org": "kfc", "title": evil, "target": {"kind": "group"}}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{node}");
    assert_eq!(node["title"], evil);

    // The table still exists and serves the row — a second create proves it.
    let (status, _) = create_node(
        &app,
        serde_json::json!({"org": "kfc", "title": "After", "target": {"kind": "group"}}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, list) = app.request("GET", "/api/v1/nav?org=kfc", None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
}
