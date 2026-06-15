//! Wire shapes for the auth surface: login, the issued token, and the current
//! principal + grants.
//!
//! The login route exchanges a subject + secret for an opaque bearer token
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Auth — close the
//! session-issuance gap"); `/auth/me` reflects the authenticated principal and
//! its capability grants so the UI can render what it may do
//! (ADMIN-UI open question 4).

use rubix_core::Principal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The body of a login request: the principal's subject and shared secret.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// The principal's subject (its `principal` record key).
    pub subject: String,
    /// The shared secret proving the bearer is the principal.
    pub secret: String,
}

/// The response to a successful login: the opaque token and its expiry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LoginResponse {
    /// The opaque bearer token to send as `Authorization: Bearer <token>`.
    pub token: String,
    /// When the token expires (RFC 3339, UTC).
    pub expires: String,
}

/// The current principal and the capabilities it holds.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MeResponse {
    /// The principal's subject id.
    pub subject: String,
    /// The namespace (tenant) the principal is scoped to.
    pub namespace: String,
    /// Whether the principal is a `user` or an `extension`.
    pub kind: String,
    /// The principal's role band (`viewer`/`operator`/`admin`).
    pub role: String,
    /// The capability grants the principal holds, as their wire strings.
    pub capabilities: Vec<String>,
}

impl MeResponse {
    /// Build the response from a principal and its granted capability strings.
    #[must_use]
    pub fn new(principal: &Principal, capabilities: Vec<String>) -> Self {
        Self {
            subject: principal.subject.to_string(),
            namespace: principal.namespace.clone(),
            kind: kind_str(principal.kind),
            role: role_str(principal.role),
            capabilities,
        }
    }
}

/// The wire string for a principal kind (matches the serialized domain form).
fn kind_str(kind: rubix_core::PrincipalKind) -> String {
    match kind {
        rubix_core::PrincipalKind::User => "user",
        rubix_core::PrincipalKind::Extension => "extension",
    }
    .to_owned()
}

/// The wire string for a role band (matches the serialized domain form).
fn role_str(role: rubix_core::Role) -> String {
    match role {
        rubix_core::Role::Viewer => "viewer",
        rubix_core::Role::Operator => "operator",
        rubix_core::Role::Admin => "admin",
    }
    .to_owned()
}
