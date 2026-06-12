//! A persisted PAT / service-account row. Holds the public id, the SHA-256 of
//! the secret (never the secret), the granted [`Scope`] and [`Role`], and a
//! revocation timestamp. The store maps this to/from the `tokens` table; the
//! verifier turns a live (non-revoked) row into a [`Principal`].

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use super::principal::{Principal, Role};
use super::scope::Scope;

/// One `tokens` row. `secret_hash` is omitted from the serialized form so the
/// operator surface never echoes it back.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TokenRecord {
    /// Public token id (the `<id>` half of the PAT, and the principal subject).
    pub id: String,
    /// Hash of the secret half. Internal only; skipped in API responses.
    #[serde(skip)]
    pub secret_hash: String,
    /// Human label for the operator surface.
    pub name: String,
    /// The role a request authenticated with this token assumes.
    pub role: Role,
    /// The org/team/site this token is confined to.
    pub scope: Scope,
    pub created_at: DateTime<Utc>,
    /// Set once the token is revoked; a revoked token is rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
}

impl TokenRecord {
    /// True when the token is still usable (not revoked).
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }

    /// The principal a request bearing this token acts as.
    pub fn principal(&self) -> Principal {
        Principal {
            subject: self.id.clone(),
            scope: self.scope.clone(),
            role: self.role,
        }
    }
}
