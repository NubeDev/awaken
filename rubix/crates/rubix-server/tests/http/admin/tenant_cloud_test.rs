//! Integration: tenant onboarding and deletion on the cloud profile.
//!
//! Surface 3 of `rubix/docs/design/ADMIN-API.md` exercised on the multi-tenant
//! path: a root/system principal onboards a tenant (bootstrap + first admin +
//! registry write → `201`), the registry lists it, the freshly-provisioned tenant
//! admin authenticates and administers its own namespace (scoped by
//! `$auth.namespace`), and `DELETE ?confirm=` purges the namespace and deregisters
//! it. The whole file compiles only under `--features cloud` — an edge build has no
//! multi-tenant code path to test.

#![cfg(feature = "cloud")]

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_server::profile::select;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{ADMIN_FULL_SUBJECT, ADMIN_SECRET, TestApp, boot_admin_with_profile};

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

/// Build a request authenticated as the root/system admin (no tenant header — the
/// control surface signs into the configured root namespace).
fn root(method: &str, uri: &str, body: Value) -> Request<Body> {
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
async fn tenant_onboards_lists_and_deletes_on_cloud() {
    let profile = select("cloud").expect("cloud is compiled in");
    let TestApp { app, .. } = boot_admin_with_profile("admin_tenant_cloud", &[], profile).await;

    // ONBOARD — bootstrap namespace + first admin + registry record.
    let (status, created) = send(
        &app,
        root(
            "POST",
            "/tenants",
            json!({ "id": "acme", "admin_subject": "owner", "admin_secret": "owner-pw" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "onboard body: {created:?}");
    assert_eq!(created["id"], json!("acme"));
    assert_eq!(created["namespace"], json!("tenant_acme"));
    assert!(
        created["created_at"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "created_at should be an RFC3339 timestamp"
    );

    // Re-onboard the same id is a conflict.
    let (status, _) = send(
        &app,
        root(
            "POST",
            "/tenants",
            json!({ "id": "acme", "admin_subject": "owner", "admin_secret": "x" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // LIST — the registry reports the onboarded tenant.
    let (status, list) = send(&app, root("GET", "/tenants", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = list
        .as_array()
        .expect("tenants")
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert!(ids.contains(&"acme"), "registry missing acme: {ids:?}");

    // The provisioned tenant admin authenticates and administers its own
    // namespace: its reads scope by `$auth.namespace` (= tenant_acme) regardless of
    // the signin infrastructure namespace, so it sees its own principals only.
    let tenant_call = Request::builder()
        .method("GET")
        .uri("/principals")
        .header("x-rubix-subject", "tenant_acme_owner")
        .header("x-rubix-secret", "owner-pw")
        .body(Body::empty())
        .expect("build request");
    let (status, principals) = send(&app, tenant_call).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "tenant admin should list its principals"
    );
    let subjects: Vec<&str> = principals
        .as_array()
        .expect("principals")
        .iter()
        .filter_map(|p| p["subject"].as_str())
        .collect();
    assert!(
        subjects.contains(&"owner"),
        "tenant admin missing: {subjects:?}"
    );

    // DELETE without confirmation is refused.
    let (status, _) = send(&app, root("DELETE", "/tenants/acme", Value::Null)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // DELETE with confirmation purges the namespace and deregisters it.
    let (status, _) = send(
        &app,
        root("DELETE", "/tenants/acme?confirm=acme", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The registry no longer lists it, and the tenant admin can no longer sign in.
    let (status, list) = send(&app, root("GET", "/tenants", Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = list
        .as_array()
        .expect("tenants")
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert!(!ids.contains(&"acme"), "acme should be gone: {ids:?}");

    let gone = Request::builder()
        .method("GET")
        .uri("/principals")
        .header("x-rubix-subject", "tenant_acme_owner")
        .header("x-rubix-secret", "owner-pw")
        .body(Body::empty())
        .expect("build request");
    let (status, _) = send(&app, gone).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "purged admin must not authenticate"
    );
}
