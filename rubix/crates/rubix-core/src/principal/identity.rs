//! The one identity model for users and extensions.
//!
//! `rubix/docs/SCOPE.md` principle 5: everything is a scoped principal. A
//! principal is the subject of every authorization decision — the same type is
//! used by the SurrealDB-native data-record scope (this workstream) and the
//! app-enforced capability layer (a later workstream). Both key off this one
//! identity.

use serde::{Deserialize, Serialize};

use crate::id::Id;

use super::{PrincipalKind, Role};

/// A scoped principal: the subject of authentication and authorization.
///
/// `namespace` is the tenant the principal is bound to; reads issued for this
/// principal are confined to that namespace by SurrealDB row-level permissions
/// (`rubix/STACK-DEISGN.md`, contract #1/#2). `kind` and `role` describe the
/// principal but do not themselves widen the data scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    /// Stable identifier for the principal.
    pub subject: Id,
    /// The namespace (tenant) the principal is scoped to.
    pub namespace: String,
    /// Whether the principal is a user or an extension service account.
    pub kind: PrincipalKind,
    /// The principal's coarse authority band within its namespace.
    pub role: Role,
}

impl Principal {
    /// Build a principal bound to a namespace.
    #[must_use]
    pub fn new(
        subject: Id,
        namespace: impl Into<String>,
        kind: PrincipalKind,
        role: Role,
    ) -> Self {
        Self {
            subject,
            namespace: namespace.into(),
            kind,
            role,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Id, Principal, PrincipalKind, Role};

    #[test]
    fn principal_carries_subject_namespace_kind_role() {
        let subject = Id::from_raw("p-1");
        let principal = Principal::new(subject.clone(), "tenant-a", PrincipalKind::User, Role::Viewer);
        assert_eq!(principal.subject, subject);
        assert_eq!(principal.namespace, "tenant-a");
        assert_eq!(principal.kind, PrincipalKind::User);
        assert_eq!(principal.role, Role::Viewer);
    }

    #[test]
    fn principal_round_trips_through_json() {
        let principal = Principal::new(
            Id::from_raw("ext-7"),
            "tenant-b",
            PrincipalKind::Extension,
            Role::Operator,
        );
        let json = serde_json::to_string(&principal).expect("serialise");
        let decoded: Principal = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(decoded, principal);
    }
}
