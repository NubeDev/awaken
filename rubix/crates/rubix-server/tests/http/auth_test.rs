//! Integration: the login-token flow over HTTP — login, bearer-authenticated
//! access, `/auth/me`, and logout revocation.
//!
//! Drives the real route table on kv-mem (`rubix/docs/sessions/WS-16.md`):
//! `POST /auth/login` returns an opaque token; that token authorizes a record
//! write and read via `Authorization: Bearer`; `GET /auth/me` reflects the
//! principal and its grants; `POST /auth/logout` revokes the token so it no
//! longer authenticates.

#[path = "../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_gate::Capability;
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{SECRET, SUBJECT, TestApp, boot};

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

fn json_post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

fn bearer(method: &str, uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

#[tokio::test]
async fn login_token_authenticates_then_logout_revokes() {
    let TestApp { app, .. } = boot("server_auth_flow", &[Capability::IngestPublish]).await;

    // LOGIN
    let (status, login) = send(
        &app,
        json_post(
            "/auth/login",
            json!({ "subject": SUBJECT, "secret": SECRET }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = login["token"].as_str().expect("token").to_owned();
    assert!(!token.is_empty());
    assert!(login["expires"].as_str().is_some());

    // The bearer token authorizes a gated write.
    let (status, created) = send(
        &app,
        bearer(
            "POST",
            "/records",
            &token,
            json!({ "content": { "temp": 21 } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(created["content"]["temp"], json!(21));

    // /auth/me reflects the principal and its grant.
    let (status, me) = send(&app, bearer("GET", "/auth/me", &token, Value::Null)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["subject"], json!(SUBJECT));
    assert_eq!(me["role"], json!("operator"));
    let caps = me["capabilities"].as_array().expect("capabilities");
    assert!(caps.iter().any(|c| c == "ingest-publish"));

    // LOGOUT revokes the token.
    let (status, _) = send(&app, bearer("POST", "/auth/logout", &token, Value::Null)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The revoked token no longer authenticates.
    let (status, _) = send(&app, bearer("GET", "/auth/me", &token, Value::Null)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_with_a_wrong_secret_is_unauthorized() {
    let TestApp { app, .. } = boot("server_auth_badlogin", &[]).await;
    let (status, _) = send(
        &app,
        json_post(
            "/auth/login",
            json!({ "subject": SUBJECT, "secret": "wrong" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_invalid_bearer_token_is_unauthorized() {
    let TestApp { app, .. } = boot("server_auth_badtoken", &[]).await;
    let (status, _) = send(&app, bearer("GET", "/auth/me", "not-a-token", Value::Null)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
