//! Auth + RBAC integration: enforced (cloud) posture exercised over the PAT
//! bearer path. The default edge harness leaves auth off, so every other test
//! module still runs unauthenticated — this module proves the enforced path.

use axum::http::StatusCode;
use chrono::Utc;
use rubix_server::auth::{pat, Role, Scope, TokenRecord};
use rubix_server::store::Store;

use super::harness::TestApp;

/// Seed a PAT directly in the store and return its plaintext bearer.
fn seed_pat(store: &Store, role: Role, scope: Scope) -> String {
    let minted = pat::mint();
    store
        .create_token(&TokenRecord {
            id: minted.id,
            secret_hash: minted.secret_hash,
            name: "seed".into(),
            role,
            scope,
            created_at: Utc::now(),
            revoked_at: None,
        })
        .expect("seed token");
    minted.plaintext
}

#[tokio::test]
async fn unauthenticated_request_is_rejected() {
    let (app, _store) = TestApp::with_auth();
    let (status, _body) = app.request("GET", "/api/v1/sites", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn health_and_openapi_stay_public() {
    let (app, _store) = TestApp::with_auth();
    let (status, _) = app.request("GET", "/healthz", None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app.request("GET", "/api-docs/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn invalid_bearer_is_rejected() {
    let (app, _store) = TestApp::with_auth();
    let (status, _) = app
        .request_as("GET", "/api/v1/sites", "rbx_pat_dead.beef", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // A non-PAT bearer routes to the (empty) JWKS verifier and also fails.
    let (status, _) = app
        .request_as("GET", "/api/v1/sites", "not.a.jwt", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn operator_creates_and_reads_within_scope() {
    let (app, store) = TestApp::with_auth();
    let bearer = seed_pat(&store, Role::Operator, Scope::org("nube"));
    let (status, body) = app
        .request_as(
            "POST",
            "/api/v1/sites",
            &bearer,
            Some(serde_json::json!({
                "org": "nube", "slug": "hq", "display_name": "HQ", "tags": {"site": true}
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().unwrap().to_string();
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/sites/{id}"), &bearer, None)
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn cross_org_write_is_forbidden() {
    let (app, store) = TestApp::with_auth();
    let bearer = seed_pat(&store, Role::Operator, Scope::org("nube"));
    let (status, _) = app
        .request_as(
            "POST",
            "/api/v1/sites",
            &bearer,
            Some(serde_json::json!({
                "org": "acme", "slug": "hq", "display_name": "HQ", "tags": {"site": true}
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn viewer_may_read_but_not_write() {
    let (app, store) = TestApp::with_auth();
    // An operator seeds a site, then a viewer in the same org is denied a write.
    let admin = seed_pat(&store, Role::Operator, Scope::global());
    let (status, body) = app
        .request_as(
            "POST",
            "/api/v1/sites",
            &admin,
            Some(serde_json::json!({
                "org": "nube", "slug": "hq", "display_name": "HQ", "tags": {"site": true}
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().unwrap().to_string();

    let viewer = seed_pat(&store, Role::Viewer, Scope::org("nube"));
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/sites/{id}"), &viewer, None)
        .await;
    assert_eq!(status, StatusCode::OK, "viewer reads in scope");
    let (status, _) = app
        .request_as("DELETE", &format!("/api/v1/sites/{id}"), &viewer, None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "viewer cannot write");
}

#[tokio::test]
async fn list_sites_is_scoped_to_the_caller() {
    let (app, store) = TestApp::with_auth();
    let admin = seed_pat(&store, Role::Operator, Scope::global());
    for org in ["nube", "acme"] {
        app.request_as(
            "POST",
            "/api/v1/sites",
            &admin,
            Some(serde_json::json!({
                "org": org, "slug": "hq", "display_name": "HQ", "tags": {"site": true}
            })),
        )
        .await;
    }
    // A nube-scoped caller sees only the nube site.
    let scoped = seed_pat(&store, Role::Viewer, Scope::org("nube"));
    let (status, body) = app.request_as("GET", "/api/v1/sites", &scoped, None).await;
    assert_eq!(status, StatusCode::OK);
    let sites = body.as_array().unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0]["org"], "nube");
}

#[tokio::test]
async fn issue_use_and_revoke_a_pat() {
    let (app, store) = TestApp::with_auth();
    let admin = seed_pat(&store, Role::Operator, Scope::global());

    // Issue a scoped service token through the admin surface.
    let (status, body) = app
        .request_as(
            "POST",
            "/api/v1/tokens",
            &admin,
            Some(serde_json::json!({
                "name": "driver-1", "role": "service",
                "scope": {"org": "nube", "team": "ops", "site": "hq"}
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let issued = body["token"].as_str().unwrap().to_string();
    let token_id = body["record"]["id"].as_str().unwrap().to_string();
    // The secret hash is never echoed.
    assert!(body["record"].get("secret_hash").is_none());

    // The issued token authenticates and is scoped to its org.
    let (status, _) = app.request_as("GET", "/api/v1/sites", &issued, None).await;
    assert_eq!(status, StatusCode::OK);

    // Revoke it; it stops working.
    let (status, _) = app
        .request_as("DELETE", &format!("/api/v1/tokens/{token_id}"), &admin, None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app.request_as("GET", "/api/v1/sites", &issued, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cannot_issue_a_token_broader_than_self() {
    let (app, store) = TestApp::with_auth();
    let scoped = seed_pat(&store, Role::Operator, Scope::org("nube"));
    // A nube-scoped operator cannot mint an acme token.
    let (status, _) = app
        .request_as(
            "POST",
            "/api/v1/tokens",
            &scoped,
            Some(serde_json::json!({
                "name": "escalate", "role": "operator", "scope": {"org": "acme"}
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
