//! The edge profile defaults.
//!
//! Edge is the default build (`rubix/docs/SCOPE.md`, "Edge and cloud profiles"):
//! a single SurrealDB namespace, no multi-tenancy code path (the gate resolves to
//! the one tenant), authentication not required for a local operator, and the
//! sync shipper off by default. This module is always compiled — `edge` is the
//! default feature and a build with only `cloud` still wants the type to exist
//! for parity; the [`profile`] constructor is the single source of these values.

use rubix_core::Profile as ProfileKind;

use super::define::{NamespaceStrategy, Profile};

/// The compiled-in edge profile.
///
/// Single-namespace, auth-not-required, sync-off — the offline-first defaults a
/// node runs with when no `RUBIX_PROFILE` override is set.
#[must_use]
pub fn profile() -> Profile {
    Profile {
        kind: ProfileKind::Edge,
        namespace_strategy: NamespaceStrategy::Single,
        auth_required: false,
        sync_enabled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::define::NamespaceStrategy;
    use super::profile;
    use rubix_core::Profile as ProfileKind;

    #[test]
    fn edge_carries_single_namespace_and_auth_not_required() {
        let p = profile();
        assert_eq!(p.kind, ProfileKind::Edge);
        assert_eq!(p.namespace_strategy, NamespaceStrategy::Single);
        assert!(!p.auth_required);
        assert!(!p.sync_enabled);
        assert!(!p.is_multi_tenant());
    }
}
