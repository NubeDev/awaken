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

    /// Require an admin (super-admin or org-admin) whose scope covers `org`.
    /// Gates the identity/authorization management surfaces (users, teams,
    /// grants).
    ///
    /// Unlike the resource gates above, this is **not** a no-op when auth is
    /// disabled: management routes demand a real principal, so a dev/edge server
    /// with no principal is *denied* rather than waved through. This is a
    /// deliberate deviation from the open-by-default convention, scoped to
    /// management mutations (`docs/design/authz-rbac.md`). super-admin = global
    /// `Admin`; org-admin = `Admin` whose scope covers `org`.
    pub fn require_admin(&self, org: &str) -> Result<&Principal, AuthError> {
        match &self.0 {
            Some(p) if p.role.can_admin() && p.scope.covers(&Scope::org(org)) => Ok(p),
            Some(p) => Err(AuthError::Forbidden(format!(
                "subject `{}` is not an admin of org `{org}`",
                p.subject
            ))),
            None => Err(AuthError::Forbidden(
                "management route requires an authenticated admin".into(),
            )),
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
            user_id: None,
            team_ids: Vec::new(),
        }))
    }

    #[test]
    fn require_admin_demands_real_covering_admin() {
        // Auth-off (no principal) is denied — management is not open-by-default.
        assert!(matches!(
            RequestPrincipal(None).require_admin("nube"),
            Err(AuthError::Forbidden(_))
        ));
        // Org-admin covers its own org, not a sibling.
        let org_admin = principal(Scope::org("nube"), Role::Admin);
        assert!(org_admin.require_admin("nube").is_ok());
        assert!(matches!(
            org_admin.require_admin("acme"),
            Err(AuthError::Forbidden(_))
        ));
        // Super-admin (global Admin) covers any org.
        let super_admin = principal(Scope::global(), Role::Admin);
        assert!(super_admin.require_admin("nube").is_ok());
        assert!(super_admin.require_admin("acme").is_ok());
        // A non-admin role is denied even at a covering scope.
        let op = principal(Scope::global(), Role::Operator);
        assert!(matches!(
            op.require_admin("nube"),
            Err(AuthError::Forbidden(_))
        ));
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
