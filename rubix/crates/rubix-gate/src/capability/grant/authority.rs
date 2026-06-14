//! Decide whether a grantor may grant or revoke a capability — fail closed.
//!
//! Grant administration is itself a privileged action: `rubix/docs/SCOPE.md`
//! ("Capabilities are grants") requires that a principal cannot hand itself or
//! another principal a capability it lacks the authority to confer. Two rules,
//! both fail-closed:
//!
//! - **Same scope.** The grantor may only administer grants inside its own
//!   namespace; a grant whose namespace differs from the grantor's is refused,
//!   so a grant can never cross a tenant boundary (mirrors the row-level read
//!   scope of WS-03, here enforced in app code because grants are app-enforced).
//! - **Authority band.** Only an [`Admin`](rubix_core::Role::Admin) holds the
//!   authority to confer or revoke capabilities. A Viewer or Operator is denied,
//!   which also blocks self-escalation: a non-admin cannot grant itself a
//!   capability it does not have.

use rubix_core::{Principal, Role};

use super::model::Grant;

/// Whether `grantor` may create or revoke `grant`.
///
/// Returns `false` (deny) unless the grantor is an admin operating within its
/// own namespace and the grant targets that same namespace.
#[must_use]
pub(crate) fn may_administer(grantor: &Principal, grant: &Grant) -> bool {
    grantor.role == Role::Admin && grantor.namespace == grant.namespace
}

#[cfg(test)]
mod tests {
    use rubix_core::{Id, Principal, PrincipalKind, Role};

    use super::{Grant, may_administer};
    use crate::capability::Capability;

    fn principal(subject: &str, namespace: &str, role: Role) -> Principal {
        Principal::new(Id::from_raw(subject), namespace, PrincipalKind::User, role)
    }

    #[test]
    fn admin_may_administer_grants_in_its_own_namespace() {
        let grantor = principal("root", "tenant-a", Role::Admin);
        let grantee = principal("alice", "tenant-a", Role::Viewer);
        let grant = Grant::new(&grantee, Capability::RuleInvoke);
        assert!(may_administer(&grantor, &grant));
    }

    #[test]
    fn a_non_admin_may_not_administer_grants() {
        let grantee = principal("alice", "tenant-a", Role::Viewer);
        let grant = Grant::new(&grantee, Capability::RuleInvoke);
        for role in [Role::Viewer, Role::Operator] {
            let grantor = principal("bob", "tenant-a", role);
            assert!(!may_administer(&grantor, &grant), "{role:?} must be denied");
        }
    }

    #[test]
    fn an_admin_may_not_administer_grants_in_another_namespace() {
        let grantor = principal("root", "tenant-a", Role::Admin);
        let grantee = principal("eve", "tenant-b", Role::Viewer);
        let grant = Grant::new(&grantee, Capability::IngestPublish);
        assert!(!may_administer(&grantor, &grant));
    }
}
