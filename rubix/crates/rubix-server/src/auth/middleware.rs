//! Axum auth middleware. When auth is enabled it requires a valid `Authorization:
//! Bearer …` header, verifies it (OIDC JWT or PAT), and attaches the resulting
//! [`Principal`] to the request extensions for downstream RBAC gates. When auth
//! is disabled (edge default) it is not installed at all, so requests pass
//! untouched — today's behavior.
//!
//! Health and the OpenAPI document stay public so liveness probes and API
//! discovery work before a caller has a token.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use super::error::AuthError;
use super::principal::Principal;
use super::verify::Authenticator;

/// Paths that bypass auth even when it is enforced: liveness and API discovery.
fn is_public(path: &str) -> bool {
    matches!(path, "/healthz" | "/api-docs/openapi.json")
}

/// Extract the bearer value from an `Authorization` header.
fn bearer(req: &Request) -> Option<&str> {
    req.headers()
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// The enforcing layer. Installed only when [`AuthConfig`](super::AuthConfig) is
/// enabled; rejects any non-public request lacking a valid bearer.
pub async fn require_auth(
    State(auth): State<Authenticator>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    if is_public(req.uri().path()) {
        return Ok(next.run(req).await);
    }
    let token = bearer(&req).ok_or(AuthError::MissingToken)?;
    let principal: Principal = auth.verify(token)?;
    req.extensions_mut().insert(principal);
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_paths_bypass() {
        assert!(is_public("/healthz"));
        assert!(is_public("/api-docs/openapi.json"));
        assert!(!is_public("/api/v1/sites"));
    }

    #[test]
    fn parses_bearer_value() {
        let req = Request::builder()
            .header(axum::http::header::AUTHORIZATION, "Bearer abc.def.ghi")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(bearer(&req), Some("abc.def.ghi"));

        let none = Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(bearer(&none), None);

        let basic = Request::builder()
            .header(axum::http::header::AUTHORIZATION, "Basic Zm9v")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(bearer(&basic), None);
    }
}
