//! Integration: the teams & memberships surface over HTTP.
//!
//! Exercises the real route table on kv-mem: team CRUD, membership add/remove
//! (subject prefix stripped on the wire), team capability grants and the
//! inheritance that makes a member exercise a team's grant, and the negative
//! cases — unknown team/principal `404`, non-admin `403`. Every admin call runs
//! as the namespace admin from the fixture; mutations route through the gate and
//! leave audit rows.

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{ADMIN_FULL_SUBJECT, ADMIN_SECRET, TestApp, boot_admin};

/// Send a request through the app, returning the status and JSON body (or null).
async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("route responds");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, json)
}

/// An authenticated JSON request carrying the admin credentials.
fn authed(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-rubix-subject", ADMIN_FULL_SUBJECT)
        .header("x-rubix-secret", ADMIN_SECRET)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

#[tokio::test]
async fn team_crud_and_membership_round_trip() {
    let TestApp { app, .. } = boot_admin("teams_crud", &[]).await;

    // CREATE a team.
    let (status, team) = send(
        &app,
        authed(
            "POST",
            "/teams",
            json!({ "slug": "engineers", "display_name": "Engineers" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(team["slug"], json!("engineers"));
    assert_eq!(team["display_name"], json!("Engineers"));

    // LIST + GET.
    let (status, list) = send(&app, authed("GET", "/teams", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().expect("teams").len(), 1);
    let (status, got) = send(&app, authed("GET", "/teams/engineers", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["display_name"], json!("Engineers"));

    // Adding an unknown principal is a 404 (no orphan member).
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/teams/engineers/members",
            json!({ "subject": "ghost" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Provision a principal, then add it as a member — the wire subject is local.
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/principals",
            json!({ "subject": "alice", "kind": "user", "role": "operator", "secret": "pw" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, member) = send(
        &app,
        authed(
            "POST",
            "/teams/engineers/members",
            json!({ "subject": "alice" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(member["subject"], json!("alice"));

    // LIST members returns the API-local subject (prefix stripped).
    let (status, members) =
        send(&app, authed("GET", "/teams/engineers/members", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    let subjects: Vec<&str> = members
        .as_array()
        .expect("members")
        .iter()
        .filter_map(|m| m["subject"].as_str())
        .collect();
    assert_eq!(subjects, vec!["alice"]);

    // Remove the member, then the list is empty.
    let (status, _) = send(
        &app,
        authed("DELETE", "/teams/engineers/members/alice", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, members) =
        send(&app, authed("GET", "/teams/engineers/members", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(members.as_array().expect("members").is_empty());

    // DELETE the team.
    let (status, _) = send(&app, authed("DELETE", "/teams/engineers", Value::Null)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, authed("GET", "/teams/engineers", Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_team_grant_is_inherited_by_a_member_over_http() {
    // ExternalQuery gates POST /query; an operator without it 403s, with it 200s.
    let TestApp { app, .. } = boot_admin("teams_inherit", &[]).await;

    // A team, a principal, and the member link.
    send(
        &app,
        authed("POST", "/teams", json!({ "slug": "analysts" })),
    )
    .await;
    send(
        &app,
        authed(
            "POST",
            "/principals",
            json!({ "subject": "alice", "kind": "user", "role": "operator", "secret": "pw" }),
        ),
    )
    .await;
    send(
        &app,
        authed(
            "POST",
            "/teams/analysts/members",
            json!({ "subject": "alice" }),
        ),
    )
    .await;

    // Grant ExternalQuery to the TEAM.
    let (status, grant) = send(
        &app,
        authed("PUT", "/teams/analysts/grants/external-query", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(grant["capability"], json!("external-query"));
    assert_eq!(grant["subject"], json!("team:analysts"));

    // The team grant lists under the team.
    let (status, grants) = send(&app, authed("GET", "/teams/analysts/grants", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(grants.as_array().expect("grants").len(), 1);

    // Alice (the member) can now run a query — proving the capability is inherited.
    let query_req = Request::builder()
        .method("POST")
        .uri("/query")
        .header("x-rubix-subject", "rubix_alice")
        .header("x-rubix-secret", "pw")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "sql": "SELECT 1 AS one" }).to_string()))
        .expect("build query request");
    let (status, _) = send(&app, query_req).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a team member must inherit the team's ExternalQuery grant"
    );

    // Revoke the team grant; the member loses access (403 again).
    let (status, _) = send(
        &app,
        authed(
            "DELETE",
            "/teams/analysts/grants/external-query",
            Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let denied_req = Request::builder()
        .method("POST")
        .uri("/query")
        .header("x-rubix-subject", "rubix_alice")
        .header("x-rubix-secret", "pw")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "sql": "SELECT 1 AS one" }).to_string()))
        .expect("build query request");
    let (status, _) = send(&app, denied_req).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoking the team grant must remove inherited access"
    );
}

#[tokio::test]
async fn team_endpoints_reject_a_non_admin() {
    let TestApp { app, .. } = boot_admin("teams_role_guard", &[]).await;
    // Provision an operator through the admin surface.
    send(
        &app,
        authed(
            "POST",
            "/principals",
            json!({ "subject": "carol", "kind": "user", "role": "operator", "secret": "pw" }),
        ),
    )
    .await;

    // Act as carol (operator) — the admin guard forbids listing teams.
    let req = Request::builder()
        .method("GET")
        .uri("/teams")
        .header("x-rubix-subject", "rubix_carol")
        .header("x-rubix-secret", "pw")
        .body(Body::empty())
        .expect("build request");
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
