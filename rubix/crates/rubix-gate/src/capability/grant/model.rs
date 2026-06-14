//! A capability grant: one capability attached to one principal.
//!
//! A grant is the unit the app-enforced authz layer stores and checks
//! (`rubix/docs/SCOPE.md`, "Capabilities are grants"). It binds a
//! [`Capability`] to a principal's subject within that principal's namespace.
//! The namespace is part of the grant so a grant never leaks across tenants —
//! a check matches subject *and* namespace.

use rubix_core::Principal;

use crate::capability::kind::Capability;

/// One capability granted to one principal, scoped to a namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    /// The subject of the principal the capability is granted to.
    pub subject: String,
    /// The namespace the grant is confined to (the grantee's tenant).
    pub namespace: String,
    /// The capability the principal may exercise.
    pub capability: Capability,
}

impl Grant {
    /// Build a grant attaching `capability` to `principal`.
    ///
    /// The grant inherits the principal's subject and namespace, so it is bound
    /// to that identity within that tenant only.
    #[must_use]
    pub fn new(principal: &Principal, capability: Capability) -> Self {
        Self {
            subject: principal.subject.to_string(),
            namespace: principal.namespace.clone(),
            capability,
        }
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::{Id, Principal, PrincipalKind, Role};

    use super::{Capability, Grant};

    #[test]
    fn grant_inherits_subject_and_namespace_from_the_principal() {
        let principal = Principal::new(
            Id::from_raw("ext-9"),
            "tenant-a",
            PrincipalKind::Extension,
            Role::Operator,
        );
        let grant = Grant::new(&principal, Capability::IngestPublish);
        assert_eq!(grant.subject, "ext-9");
        assert_eq!(grant.namespace, "tenant-a");
        assert_eq!(grant.capability, Capability::IngestPublish);
    }
}
