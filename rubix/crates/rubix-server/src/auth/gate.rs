//! The RBAC gate domain routes call to authorize a request against a target
//! [`Scope`]. When auth is enabled the middleware has attached a [`Principal`];
//! the gate reads it from request extensions and checks scope coverage. When
//! auth is disabled there is no principal, so the gate is a no-op — preserving
//! the edge profile's open behavior.
//!
//! Handlers obtain a [`RequestPrincipal`] via the axum extractor below, then
//! call [`RequestPrincipal::authorize_read`] / [`authorize_write`] with the
//! resource scope. Routes that do not bind a resource to a site (health, the
//! token admin surface) need only an authenticated caller.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use super::error::AuthError;
use super::principal::Principal;
use super::scope::Scope;

/// The principal attached to a request, or `None` when auth is disabled. An
/// axum extractor: handlers add a `principal: RequestPrincipal` parameter to
/// gate themselves.
#[derive(Debug, Clone)]
pub struct RequestPrincipal(pub Option<Principal>);

impl<S: Sync> FromRequestParts<S> for RequestPrincipal {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(RequestPrincipal(
            parts.extensions.get::<Principal>().cloned(),
        ))
    }
}

impl RequestPrincipal {
    /// Authorize a read of a resource in `target`. Passes when auth is disabled
    /// (no principal) or when the principal's scope covers the target.
    pub fn authorize_read(&self, target: &Scope) -> Result<(), AuthError> {
        match &self.0 {
            None => Ok(()),
            Some(p) if p.may_read(target) => Ok(()),
            Some(p) => Err(AuthError::Forbidden(format!(
                "subject `{}` may not read scope {:?}",
                p.subject, target
            ))),
        }
    }

    /// Authorize a write of a resource in `target`. Passes when auth is disabled,
    /// else requires a write-capable role whose scope covers the target.
    pub fn authorize_write(&self, target: &Scope) -> Result<(), AuthError> {
        match &self.0 {
            None => Ok(()),
            Some(p) if p.may_write(target) => Ok(()),
            Some(p) => Err(AuthError::Forbidden(format!(
                "subject `{}` may not write scope {:?}",
                p.subject, target
            ))),
        }
    }

    /// Authorize a read of a domain resource at `org`/`site` (the natural key of
    /// a [`rubix_core::Site`]). Passes when auth is disabled or the principal's
    /// scope covers the org/site.
    pub fn authorize_site_read(&self, org: &str, site: &str) -> Result<(), AuthError> {
        match &self.0 {
            None => Ok(()),
            Some(p) if p.scope.covers_resource(org, site) => Ok(()),
            Some(p) => Err(AuthError::Forbidden(format!(
                "subject `{}` may not read site `{org}/{site}`",
                p.subject
            ))),
        }
    }

    /// Authorize a write of a domain resource at `org`/`site`. Requires a
    /// write-capable role whose scope covers the org/site.
    pub fn authorize_site_write(&self, org: &str, site: &str) -> Result<(), AuthError> {
        match &self.0 {
            None => Ok(()),
            Some(p) if p.role.can_write() && p.scope.covers_resource(org, site) => Ok(()),
            Some(p) => Err(AuthError::Forbidden(format!(
                "subject `{}` may not write site `{org}/{site}`",
                p.subject
            ))),
        }
    }

    /// Require any authenticated caller (used by routes with no per-resource
    /// scope). A no-op when auth is disabled.
    pub fn require_authenticated(&self) -> Result<&Principal, AuthError> {
        // When auth is off there is no principal to return; callers that need a
        // concrete principal only run on the enabled path, where the middleware
        // guarantees one. Surface a clear error rather than fabricate identity.
        self.0
            .as_ref()
            .ok_or_else(|| AuthError::Forbidden("route requires an authenticated caller".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::principal::Role;

    fn principal(scope: Scope, role: Role) -> RequestPrincipal {
        RequestPrincipal(Some(Principal {
            subject: "u1".into(),
            scope,
            role,
        }))
    }

    #[test]
    fn disabled_auth_is_open() {
        let none = RequestPrincipal(None);
        assert!(none.authorize_read(&Scope::org("nube")).is_ok());
        assert!(none.authorize_write(&Scope::org("nube")).is_ok());
    }

    #[test]
    fn in_scope_operator_reads_and_writes() {
        let p = principal(Scope::org("nube"), Role::Operator);
        assert!(p.authorize_read(&Scope::org("nube")).is_ok());
        assert!(p.authorize_write(&Scope::org("nube")).is_ok());
    }

    #[test]
    fn viewer_reads_but_is_denied_writes() {
        let p = principal(Scope::org("nube"), Role::Viewer);
        assert!(p.authorize_read(&Scope::org("nube")).is_ok());
        assert!(matches!(
            p.authorize_write(&Scope::org("nube")),
            Err(AuthError::Forbidden(_))
        ));
    }

    #[test]
    fn cross_scope_is_denied() {
        let p = principal(Scope::org("nube"), Role::Operator);
        assert!(matches!(
            p.authorize_read(&Scope::org("acme")),
            Err(AuthError::Forbidden(_))
        ));
    }
}
