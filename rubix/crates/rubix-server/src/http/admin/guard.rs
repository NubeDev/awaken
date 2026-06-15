//! The admin-in-namespace transport guard shared by the admin surfaces.
//!
//! Every admin mutation is guarded two ways (`rubix/docs/design/ADMIN-API.md`,
//! "Authorization model"): a transport guard that the principal is an `Admin` in
//! the target namespace, and — where applicable — a capability grant checked by
//! the gate. This module owns the first: a thin check over the
//! [`Authenticated`](crate::auth::Authenticated) principal that admin handlers
//! call before touching the store. Admin endpoints operate on
//! `auth.principal.namespace`, which is correct for the in-namespace rule; cross-
//! tenant administration is the onboarding/root path, not header auth
//! (ADMIN-API Cross-cutting).

use rubix_core::{Principal, Role};

use crate::error::ApiError;

/// Ensure `principal` is an `Admin`, returning its namespace as the admin scope.
///
/// The admin surfaces operate within the caller's own namespace, so the namespace
/// is taken from the principal, never from the request — there is no cross-tenant
/// admin write path through this guard (mirrors the gate command's namespace rule).
///
/// # Errors
/// Returns [`ApiError::Forbidden`] if the principal's role is not `Admin`.
pub fn require_admin(principal: &Principal) -> Result<String, ApiError> {
    if principal.role == Role::Admin {
        Ok(principal.namespace.clone())
    } else {
        Err(ApiError::Forbidden(
            "admin role required in this namespace".to_owned(),
        ))
    }
}
