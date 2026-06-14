//! The persisted shape of a [`Grant`] at the SurrealDB boundary.
//!
//! Grants live in the `grant` table. Because grants are app-enforced (not a
//! SurrealDB record permission, `rubix/docs/SCOPE.md` "Two authz layers"), the
//! gate writes and reads them on the store handle and does the authority/scope
//! checks itself. The row key is derived from `namespace + subject + capability`
//! so a (principal, capability) pair has exactly one grant row: create is
//! idempotent and revoke targets a single deterministic key.

use surrealdb::types::{RecordId, SurrealValue};

use crate::capability::kind::Capability;

use super::model::Grant;

/// The table capability grants are stored in.
pub(crate) const GRANT_TABLE: &str = "grant";

/// SurrealDB-facing grant: the reserved `id` thing plus the grant fields.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct GrantRow {
    pub(crate) id: RecordId,
    pub(crate) namespace: String,
    pub(crate) subject: String,
    pub(crate) capability: String,
}

impl GrantRow {
    /// Project a domain [`Grant`] into its persisted row.
    pub(crate) fn from_grant(grant: &Grant) -> Self {
        Self {
            id: RecordId::new(GRANT_TABLE, grant_key(grant)),
            namespace: grant.namespace.clone(),
            subject: grant.subject.clone(),
            capability: grant.capability.as_str().to_owned(),
        }
    }

    /// Reconstruct the domain [`Grant`], dropping any unknown capability.
    ///
    /// An unrecognised stored capability resolves to `None` rather than a guess,
    /// so a corrupted or stale row can never be exercised as a grant.
    pub(crate) fn into_grant(self) -> Option<Grant> {
        Some(Grant {
            subject: self.subject,
            namespace: self.namespace,
            capability: Capability::parse(&self.capability)?,
        })
    }
}

/// The deterministic record key for a grant: `namespace:subject:capability`.
///
/// One row per (namespace, subject, capability) triple makes create idempotent
/// and revoke addressable without a scan.
pub(crate) fn grant_key(grant: &Grant) -> String {
    format!(
        "{}:{}:{}",
        grant.namespace,
        grant.subject,
        grant.capability.as_str()
    )
}
