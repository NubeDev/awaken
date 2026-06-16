//! A capability grant: one capability attached to one principal.
//!
//! A grant is the unit the app-enforced authz layer stores and checks
//! (`rubix/docs/SCOPE.md`, "Capabilities are grants"). It binds a
//! [`Capability`] to a principal's subject within that principal's namespace.
//! The namespace is part of the grant so a grant never leaks across tenants —
//! a check matches subject *and* namespace.

use rubix_core::Principal;

use crate::capability::kind::Capability;

/// The subject prefix marking a grant whose holder is a **team**, not a
/// principal. A grant with subject `team:engineers` is held by the team, and
/// flows to every member when their effective grants are resolved
/// (`rubix/docs/SCOPE.md`, "Capabilities are grants"; team inheritance in
/// [`team`](crate::team)). A principal subject never collides with this because
/// principal subjects are the prefixed `{namespace}_{local}` storage form.
pub const TEAM_SUBJECT_PREFIX: &str = "team:";

/// One capability granted to one **subject** (a principal or a team), scoped to
/// a namespace.
///
/// The subject is either a principal's full storage subject or a team subject
/// (`team:{slug}`). Both share the one grant table; a team grant is resolved to
/// the team's members at check time, so the storage shape is identical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    /// The subject the capability is granted to — a principal subject or a
    /// `team:{slug}` team subject.
    pub subject: String,
    /// The namespace the grant is confined to (the grantee's tenant).
    pub namespace: String,
    /// The capability the subject may exercise.
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

    /// Build a grant attaching `capability` to a **team** by `slug` in
    /// `namespace`.
    ///
    /// The subject is the `team:{slug}` form, so the grant is held by the team
    /// and inherited by its members when their effective grants are resolved.
    #[must_use]
    pub fn for_team(slug: &str, namespace: &str, capability: Capability) -> Self {
        Self {
            subject: team_subject(slug),
            namespace: namespace.to_owned(),
            capability,
        }
    }
}

/// The grant subject string for a team `slug` (`team:{slug}`).
#[must_use]
pub fn team_subject(slug: &str) -> String {
    format!("{TEAM_SUBJECT_PREFIX}{slug}")
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
