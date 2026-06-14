//! Audit-read + undo/redo HTTP surface (docs/design/audit-and-undo.md "Audit read
//! surface", "Undo/Redo"). Driven over the enforced (PAT) path so the recorded
//! actor is a real `User { subject }` and the admin capability gate has a principal
//! to authorize. Proves: editing a dashboard records one row with before/after;
//! `GET /audit` returns it, filterable and org-isolated, admin-gated; `POST /undo`
//! restores it and returns the touched ids; `POST /redo` re-applies.

use axum::http::StatusCode;
use chrono::Utc;
use rubix_server::auth::{pat, AdminLevel, Role, Scope, TokenRecord};
use rubix_server::store::{Store, UserRecord};
use serde_json::json;
use uuid::Uuid;

use super::harness::TestApp;

/// Mint a PAT resolving to a seeded admin user in `org`. Returns the bearer; the
/// recorded change actor's subject is this PAT id.
fn seed_admin(store: &Store, org: &str, level: AdminLevel) -> String {
    let minted = pat::mint();
    store
        .create_token(&TokenRecord {
            id: minted.id.clone(),
            secret_hash: minted.secret_hash,
            name: "audit-admin".into(),
            role: Role::Operator,
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
            display_name: "audit admin".into(),
            admin_level: level,
            created_at: Utc::now(),
        })
        .expect("seed user");
    minted.plaintext
}

/// Create a dashboard over HTTP and return its id; the create is recorded as a
/// `dashboard` change attributed to the bearer's subject.
async fn create_dashboard(app: &TestApp, bearer: &str, org: &str, slug: &str) -> Uuid {
    let (status, body) = app
        .request_as(
            "POST",
            "/api/v1/dashboards",
            bearer,
            Some(json!({"org": org, "slug": slug, "title": "Overview"})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
async fn editing_a_dashboard_records_and_audit_lists_it() {
    let (app, store) = TestApp::with_auth();
    let bearer = seed_admin(&store, "nube", AdminLevel::OrgAdmin);
    let id = create_dashboard(&app, &bearer, "nube", "overview").await;

    // Patch the title — recorded as an update with before/after.
    let (status, _) = app
        .request_as(
            "PATCH",
            &format!("/api/v1/dashboards/{id}"),
            &bearer,
            Some(json!({"title": "Renamed"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // The audit log shows both rows, newest-first, with snapshots.
    let (status, rows) = app
        .request_as("GET", "/api/v1/audit?org=nube", &bearer, None)
        .await;
    assert_eq!(status, StatusCode::OK, "{rows}");
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "create + update recorded");
    assert_eq!(rows[0]["op"], "update");
    assert_eq!(rows[0]["before"]["title"], "Overview");
    assert_eq!(rows[0]["after"]["title"], "Renamed");
    assert_eq!(rows[0]["actor"]["kind"], "user");

    // Filter by kind + resource narrows to this dashboard's timeline.
    let (status, timeline) = app
        .request_as(
            "GET",
            &format!("/api/v1/audit/dashboard/{id}?org=nube"),
            &bearer,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(timeline.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn audit_read_is_admin_gated_and_org_isolated() {
    let (app, store) = TestApp::with_auth();
    let nube_admin = seed_admin(&store, "nube", AdminLevel::OrgAdmin);
    let acme_admin = seed_admin(&store, "acme", AdminLevel::OrgAdmin);
    let id = create_dashboard(&app, &nube_admin, "nube", "overview").await;
    let _ = id;

    // A non-admin (plain operator, no admin level) is refused the audit surface.
    let minted = pat::mint();
    store
        .create_token(&TokenRecord {
            id: minted.id.clone(),
            secret_hash: minted.secret_hash,
            name: "op".into(),
            role: Role::Operator,
            scope: Scope::org("nube"),
            created_at: Utc::now(),
            revoked_at: None,
        })
        .expect("seed token");
    store
        .create_user(&UserRecord {
            id: Uuid::new_v4(),
            org: "nube".into(),
            subject: minted.id.clone(),
            email: "op@nube.test".into(),
            display_name: "op".into(),
            admin_level: AdminLevel::None,
            created_at: Utc::now(),
        })
        .expect("seed user");
    let (status, _) = app
        .request_as("GET", "/api/v1/audit?org=nube", &minted.plaintext, None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "non-admin is refused audit read");

    // acme's admin cannot read nube's log (cross-org), and sees nothing of its own.
    let (status, _) = app
        .request_as("GET", "/api/v1/audit?org=nube", &acme_admin, None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "cross-org audit read denied");
    let (status, rows) = app
        .request_as("GET", "/api/v1/audit?org=acme", &acme_admin, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(rows.as_array().unwrap().is_empty(), "acme's log has no nube rows");
}

#[tokio::test]
async fn undo_then_redo_round_trips_a_dashboard_edit() {
    let (app, store) = TestApp::with_auth();
    let bearer = seed_admin(&store, "nube", AdminLevel::OrgAdmin);
    let id = create_dashboard(&app, &bearer, "nube", "overview").await;

    // Undo the create → the dashboard is gone, and the touched id is returned.
    let (status, body) = app
        .request_as("POST", "/api/v1/undo", &bearer, Some(json!({"org": "nube"})))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["touched"][0], id.to_string());
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/dashboards/{id}"), &bearer, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "undo removed the created board");

    // Redo → the dashboard is back.
    let (status, body) = app
        .request_as("POST", "/api/v1/redo", &bearer, Some(json!({"org": "nube"})))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["touched"][0], id.to_string());
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/dashboards/{id}"), &bearer, None)
        .await;
    assert_eq!(status, StatusCode::OK, "redo restored the board");
}

#[tokio::test]
async fn undo_is_per_actor() {
    let (app, store) = TestApp::with_auth();
    let alice = seed_admin(&store, "nube", AdminLevel::OrgAdmin);
    let bob = seed_admin(&store, "nube", AdminLevel::OrgAdmin);
    let alice_board = create_dashboard(&app, &alice, "nube", "alice").await;

    // Bob has made no change: his undo is a no-op and never pops alice's group.
    let (status, body) = app
        .request_as("POST", "/api/v1/undo", &bob, Some(json!({"org": "nube"})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["group"].is_null(), "bob has nothing to undo");
    // Alice's board still exists.
    let (status, _) = app
        .request_as(
            "GET",
            &format!("/api/v1/dashboards/{alice_board}"),
            &alice,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "another actor's undo left alice's board intact");
}
