//! The persisted shape of a principal at the SurrealDB boundary.
//!
//! The `principal` table holds one row per identity. The row carries the secret
//! used by the record access method's `SIGNIN` query, plus the identity fields
//! the gate hydrates into a [`Principal`](rubix_core::Principal). The secret
//! never leaves this crate's authentication path.

use rubix_core::{Id, Principal, PrincipalKind, Role};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue, ToSql};

/// The table principals are stored in (mirrors the access method's `SIGNIN`).
pub(crate) const PRINCIPAL_TABLE: &str = "principal";

/// SurrealDB-facing principal: the reserved `id` thing plus identity + secret.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct PrincipalRow {
    pub(crate) id: RecordId,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) role: String,
    pub(crate) secret: String,
}

impl PrincipalRow {
    /// Project a domain principal plus its secret into a persisted row.
    pub(crate) fn new(principal: &Principal, secret: impl Into<String>) -> Self {
        Self {
            id: RecordId::new(PRINCIPAL_TABLE, principal.subject.as_str()),
            namespace: principal.namespace.clone(),
            kind: kind_str(principal.kind).to_owned(),
            role: role_str(principal.role).to_owned(),
            secret: secret.into(),
        }
    }

    /// Reconstruct the domain [`Principal`], dropping the secret.
    pub(crate) fn into_principal(self) -> Option<Principal> {
        Some(Principal::new(
            Id::from_raw(record_key(&self.id)),
            self.namespace,
            kind_from_str(&self.kind)?,
            role_from_str(&self.role)?,
        ))
    }
}

fn kind_str(kind: PrincipalKind) -> &'static str {
    match kind {
        PrincipalKind::User => "user",
        PrincipalKind::Extension => "extension",
    }
}

fn kind_from_str(raw: &str) -> Option<PrincipalKind> {
    match raw {
        "user" => Some(PrincipalKind::User),
        "extension" => Some(PrincipalKind::Extension),
        _ => None,
    }
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::Viewer => "viewer",
        Role::Operator => "operator",
        Role::Admin => "admin",
    }
}

fn role_from_str(raw: &str) -> Option<Role> {
    match raw {
        "viewer" => Some(Role::Viewer),
        "operator" => Some(Role::Operator),
        "admin" => Some(Role::Admin),
        _ => None,
    }
}

/// The raw string form of a principal id's key (the part after `principal:`).
fn record_key(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => other.to_sql(),
    }
}
