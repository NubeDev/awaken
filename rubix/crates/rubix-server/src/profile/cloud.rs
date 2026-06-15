//! The cloud profile defaults.
//!
//! Cloud is namespace-per-tenant with teams/users via SurrealDB auth and the sync
//! shipper on (`rubix/docs/SCOPE.md`, "Edge and cloud profiles"). It is compiled
//! only under the `cloud` feature — a plain edge build carries no cloud code path
//! at all. Because cloud requires the Postgres backend (WS-10, a cloud-only
//! connector), the `cloud` feature pulls in `postgres`; that the backend is
//! present in the build is checked at boot by
//! [`verify_backends`](super::verify_backends), which fails closed when it is not.

use rubix_core::Profile as ProfileKind;

use super::define::{NamespaceStrategy, Profile};

/// The compiled-in cloud profile.
///
/// Namespace-per-tenant, auth-required, sync-on — the multi-tenant defaults a
/// cloud deployment runs with. Selecting it requires the build to have been
/// compiled with `--features cloud`; absent it, [`select`](super::select) rejects
/// the name rather than falling back.
#[must_use]
pub fn profile() -> Profile {
    Profile {
        kind: ProfileKind::Cloud,
        namespace_strategy: NamespaceStrategy::PerTenant,
        auth_required: true,
        sync_enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::super::define::NamespaceStrategy;
    use super::profile;
    use rubix_core::Profile as ProfileKind;

    #[test]
    fn cloud_carries_per_tenant_and_auth_required() {
        let p = profile();
        assert_eq!(p.kind, ProfileKind::Cloud);
        assert_eq!(p.namespace_strategy, NamespaceStrategy::PerTenant);
        assert!(p.auth_required);
        assert!(p.sync_enabled);
        assert!(p.is_multi_tenant());
    }
}
