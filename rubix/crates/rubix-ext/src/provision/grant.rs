//! Attach capability grants to an extension (WS-04).
//!
//! What an extension may *do* is the second authz layer: a set of WS-04
//! capability grants on its principal (`rubix/docs/sessions/WS-13.md`, contract
//! #2). The *same* grant mechanism expresses every extension shape — a read-only
//! extension, an ingest-only extension, and an admin extension differ **only** in
//! which capabilities they are granted, never in a separate trust path. A
//! [`GrantProfile`] names a distinct permitted-action set; [`grant_extension`]
//! confers each capability through the WS-04 [`create_grant`](rubix_gate::create_grant)
//! path, which is itself authority-checked (only a namespace admin may confer a
//! grant — fail closed, no self-escalation).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_gate::{Capability, Grant, create_grant};

use crate::error::{ExtError, Result};

/// A named bundle of capabilities expressing one extension shape.
///
/// The three profiles resolve to **distinct** permitted-action sets from the one
/// grant mechanism (`rubix/docs/sessions/WS-13.md`, "Tests"): only the
/// capabilities differ, not the enforcement path. The order within each set is
/// declaration order; duplicates cannot arise because each capability appears at
/// most once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantProfile {
    /// A read-only extension: no cross-plane action capability at all. Its data
    /// reads are still scoped by SurrealDB row-perms (WS-03); it simply holds no
    /// grant to invoke, ingest, or register anything.
    ReadOnly,
    /// An ingest-only extension: may publish into the ingest plane and subscribe
    /// to its Zenoh key-space, nothing else.
    IngestOnly,
    /// An admin extension: may exercise every cross-plane capability the platform
    /// knows about.
    Admin,
}

impl GrantProfile {
    /// The capabilities this profile confers — the permitted action set.
    ///
    /// The three profiles return disjoint-enough sets that they are
    /// distinguishable (`rubix/docs/sessions/WS-13.md`): read-only is empty,
    /// ingest-only is the ingest pair, admin is everything. All three are derived
    /// from the one [`Capability`] enum — no profile invents a capability outside
    /// the WS-04 allow-set.
    #[must_use]
    pub fn capabilities(self) -> &'static [Capability] {
        match self {
            GrantProfile::ReadOnly => &[],
            GrantProfile::IngestOnly => {
                &[Capability::IngestPublish, Capability::ZenohSubscribe]
            }
            GrantProfile::Admin => &Capability::ALL,
        }
    }
}

/// Grant `extension` the capabilities of `profile`, authorized by `grantor`.
///
/// Confers each capability in the profile through the WS-04
/// [`create_grant`](rubix_gate::create_grant) path, which checks `grantor`'s
/// authority before each write (only a namespace admin may confer a grant). A
/// [`ReadOnly`](GrantProfile::ReadOnly) profile writes no grants and returns an
/// empty vector — the extension holds no cross-plane capability, fail closed by
/// default. Returns the grants conferred, in profile order.
///
/// # Errors
/// Returns [`ExtError::Grant`] if `grantor` lacks the authority to confer a
/// grant (not an admin in the extension's namespace) or the write fails. On the
/// first failure no further grants are attempted.
pub async fn grant_extension(
    db: &Surreal<Db>,
    grantor: &Principal,
    extension: &Principal,
    profile: GrantProfile,
) -> Result<Vec<Grant>> {
    let mut conferred = Vec::with_capacity(profile.capabilities().len());
    for &capability in profile.capabilities() {
        let grant = create_grant(db, grantor, extension, capability)
            .await
            .map_err(|e| ExtError::Grant(e.to_string()))?;
        conferred.push(grant);
    }
    Ok(conferred)
}

#[cfg(test)]
mod tests {
    use rubix_gate::Capability;

    use super::GrantProfile;

    #[test]
    fn the_three_profiles_resolve_to_distinct_action_sets() {
        let read_only = GrantProfile::ReadOnly.capabilities();
        let ingest_only = GrantProfile::IngestOnly.capabilities();
        let admin = GrantProfile::Admin.capabilities();

        // Read-only confers no cross-plane capability.
        assert!(read_only.is_empty());
        // Ingest-only is exactly the ingest pair.
        assert_eq!(
            ingest_only,
            &[Capability::IngestPublish, Capability::ZenohSubscribe]
        );
        // Admin is every known capability.
        assert_eq!(admin.len(), Capability::ALL.len());

        // The three sets are pairwise distinct.
        assert_ne!(read_only, ingest_only);
        assert_ne!(ingest_only, admin);
        assert_ne!(read_only, admin);
    }

    #[test]
    fn ingest_only_excludes_admin_only_capabilities() {
        let ingest_only = GrantProfile::IngestOnly.capabilities();
        // An ingest-only extension cannot register a datasource or invoke a rule.
        assert!(!ingest_only.contains(&Capability::DatasourceRegister));
        assert!(!ingest_only.contains(&Capability::RuleInvoke));
        assert!(!ingest_only.contains(&Capability::ExternalQuery));
    }
}
