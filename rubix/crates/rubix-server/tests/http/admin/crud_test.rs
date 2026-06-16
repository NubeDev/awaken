//! Integration: the admin & management surface over HTTP.
//!
//! Exercises the real route table (`rubix/docs/design/ADMIN-API.md`) on kv-mem:
//! principals CRUD with the last-admin guard, grants nested under a principal,
//! the device registry through the gate, and tenant onboarding's edge `409` /
//! root-auth behavior. Every admin call runs as the namespace admin from the
//! fixture; mutations route through the gate, so they leave audit rows.

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_gate::Capability;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{ADMIN_FULL_SUBJECT, ADMIN_SECRET, ADMIN_SUBJECT, TestApp, boot_admin};

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

/// Build an authenticated JSON request carrying the admin credentials.
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
async fn principal_crud_round_trips_and_strips_the_prefix() {
    let TestApp { app, .. } = boot_admin("admin_principal_crud", &[]).await;

    // CREATE — the API-local subject is `alice`; the response echoes it.
    let (status, created) = send(
        &app,
        authed(
            "POST",
            "/principals",
            json!({ "subject": "alice", "kind": "user", "role": "operator", "secret": "pw" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["subject"], json!("alice"));
    assert_eq!(created["role"], json!("operator"));

    // Re-create is a conflict (provision is non-idempotent).
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/principals",
            json!({ "subject": "alice", "kind": "user", "role": "operator", "secret": "pw" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // GET by local subject.
    let (status, fetched) = send(&app, authed("GET", "/principals/alice", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["subject"], json!("alice"));

    // LIST returns the admin and alice, all local subjects.
    let (status, list) = send(&app, authed("GET", "/principals", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    let subjects: Vec<&str> = list
        .as_array()
        .expect("list")
        .iter()
        .filter_map(|p| p["subject"].as_str())
        .collect();
    assert!(subjects.contains(&"alice"), "missing alice: {subjects:?}");
    assert!(
        subjects.contains(&ADMIN_SUBJECT),
        "missing admin: {subjects:?}"
    );

    // PATCH role.
    let (status, patched) = send(
        &app,
        authed("PATCH", "/principals/alice", json!({ "role": "viewer" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["role"], json!("viewer"));

    // DELETE.
    let (status, _) = send(&app, authed("DELETE", "/principals/alice", Value::Null)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, authed("GET", "/principals/alice", Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_last_admin_cannot_be_demoted_or_deleted() {
    let TestApp { app, .. } = boot_admin("admin_last_admin", &[]).await;

    // The fixture admin is the only admin — demoting it is refused.
    let (status, _) = send(
        &app,
        authed(
            "PATCH",
            &format!("/principals/{ADMIN_SUBJECT}"),
            json!({ "role": "operator" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Deleting the last admin is refused too.
    let (status, _) = send(
        &app,
        authed(
            "DELETE",
            &format!("/principals/{ADMIN_SUBJECT}"),
            Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn grants_are_nested_and_require_an_existing_principal() {
    let TestApp { app, .. } = boot_admin("admin_grants", &[]).await;

    // Granting to an unknown subject is a 404 (no orphan grants).
    let (status, _) = send(
        &app,
        authed(
            "PUT",
            "/principals/ghost/grants/ingest-publish",
            Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Provision a target, then grant + list + revoke.
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/principals",
            json!({ "subject": "bob", "kind": "user", "role": "operator", "secret": "pw" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Unknown capability is a 400.
    let (status, _) = send(
        &app,
        authed("PUT", "/principals/bob/grants/not-a-cap", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Grant (idempotent).
    let (status, grant) = send(
        &app,
        authed("PUT", "/principals/bob/grants/ingest-publish", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(grant["capability"], json!("ingest-publish"));
    assert_eq!(grant["subject"], json!("bob"));

    let (status, grants) = send(&app, authed("GET", "/principals/bob/grants", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(grants.as_array().expect("grants").len(), 1);

    // Revoke (idempotent — second revoke is still 204).
    let (status, _) = send(
        &app,
        authed(
            "DELETE",
            "/principals/bob/grants/ingest-publish",
            Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(
        &app,
        authed(
            "DELETE",
            "/principals/bob/grants/ingest-publish",
            Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn device_registry_crud_through_the_gate() {
    // The device routes require the `device-manage` capability for mutation.
    let TestApp { app, store } = boot_admin("admin_devices", &[Capability::DeviceManage]).await;

    // CREATE.
    let (status, created) = send(
        &app,
        authed(
            "POST",
            "/devices",
            json!({ "id": "edge-1", "label": "Gateway 1", "kind": "gateway",
                    "metadata": { "floor": 2 } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["id"], json!("edge-1"));
    assert_eq!(created["kind"], json!("gateway"));
    assert_eq!(created["metadata"]["floor"], json!(2));

    // Duplicate id is a conflict.
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/devices",
            json!({ "id": "edge-1", "label": "dup", "kind": "gateway" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // GET + LIST.
    let (status, got) = send(&app, authed("GET", "/devices/edge-1", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["label"], json!("Gateway 1"));
    let (status, list) = send(&app, authed("GET", "/devices", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().expect("devices").len(), 1);

    // PATCH label only — kind and metadata persist.
    let (status, patched) = send(
        &app,
        authed("PATCH", "/devices/edge-1", json!({ "label": "Renamed" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["label"], json!("Renamed"));
    assert_eq!(patched["kind"], json!("gateway"));
    assert_eq!(patched["metadata"]["floor"], json!(2));

    // DELETE.
    let (status, _) = send(&app, authed("DELETE", "/devices/edge-1", Value::Null)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, authed("GET", "/devices/edge-1", Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Mutations left audit rows (create/update/delete) under the device record id.
    let mut rows = store
        .raw()
        .query("SELECT action FROM audit WHERE target = $t")
        .bind(("t", "rubix_edge-1"))
        .await
        .expect("query audit");
    let audited: Vec<Value> = rows.take(0).expect("audit rows");
    let actions: Vec<&str> = audited
        .iter()
        .filter_map(|r| r["action"].as_str())
        .collect();
    assert!(actions.contains(&"create"), "missing create: {actions:?}");
    assert!(actions.contains(&"delete"), "missing delete: {actions:?}");
}

#[tokio::test]
async fn device_mutation_without_the_capability_is_forbidden() {
    // No device-manage grant: the gate denies the create.
    let TestApp { app, .. } = boot_admin("admin_devices_nogrant", &[]).await;
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/devices",
            json!({ "id": "edge-x", "label": "x", "kind": "sensor" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_onboarding_is_a_conflict_on_the_edge_default_profile() {
    // The fixture admin is the root/system principal (admin in the root
    // namespace), so it passes the system guard; the edge profile then rejects
    // the mutation with 409 — one binary, identical route table.
    let TestApp { app, .. } = boot_admin("admin_tenant_edge", &[]).await;
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/tenants",
            json!({ "id": "acme", "admin_subject": "owner", "admin_secret": "pw" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // GET reports the single configured namespace on edge.
    let (status, list) = send(&app, authed("GET", "/tenants", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().expect("tenants").len(), 1);
    assert_eq!(list[0]["id"], json!("rubix"));
}

#[tokio::test]
async fn admin_endpoints_reject_a_non_admin() {
    // Provision an operator through the admin surface, then drive the surface as
    // that operator — the admin-in-namespace guard forbids it.
    let TestApp { app, .. } = boot_admin("admin_role_guard", &[]).await;
    let (status, _) = send(
        &app,
        authed(
            "POST",
            "/principals",
            json!({ "subject": "carol", "kind": "user", "role": "operator", "secret": "pw" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Now act as carol (operator) — the admin guard forbids it.
    let req = Request::builder()
        .method("GET")
        .uri("/principals")
        .header("x-rubix-subject", "rubix_carol")
        .header("x-rubix-secret", "pw")
        .body(Body::empty())
        .expect("build request");
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
