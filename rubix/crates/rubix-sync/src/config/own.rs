//! The ownership decision: who owns a piece of config, cloud or edge.
//!
//! Config (dashboards, rules, tags, datasource defs) is the only surface that
//! ever reconciles (`rubix/docs/SCOPE.md`, "Sync and conflict model"). Ownership
//! is the **first** rule, taken before any last-write-wins: the cloud owns shared
//! and per-tenant config; the edge owns local-only config. Where ownership is
//! unambiguous there is no conflict to resolve — the owner's version wins
//! outright, and LWW is never consulted. Only where overlap is unavoidable does
//! reconciliation fall through to [`last_write_wins`](super::last_write_wins).
//!
//! This mirrors the WS-14 profile split: cloud owns the multi-tenant/shared
//! surface, edge owns the single-namespace local surface.

/// The scope a piece of config is declared in.
///
/// `Shared` and `Tenant` config is authored on and owned by the cloud;
/// `LocalOnly` config is authored on and owned by the edge. The scope is a
/// property of the config definition, not of where a write happened to arrive —
/// so a stray edge edit to shared config does not transfer ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// Config shared across all tenants — cloud-owned.
    Shared,
    /// Per-tenant config — cloud-owned (the cloud is the multi-tenant authority).
    Tenant,
    /// Config that exists only on one edge — edge-owned.
    LocalOnly,
}

/// Which side owns a piece of config and therefore wins a reconcile outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// The cloud owns this config; its version is authoritative.
    Cloud,
    /// The edge owns this config; its version is authoritative.
    Edge,
}

/// Decide the owner of config in `scope`.
///
/// Cloud owns shared and per-tenant config; edge owns local-only config. This is
/// total over the scope, so every config item has a definite owner — there is no
/// "unowned" config whose conflict would have to be guessed.
#[must_use]
pub fn owner_of(scope: ConfigScope) -> Owner {
    match scope {
        ConfigScope::Shared | ConfigScope::Tenant => Owner::Cloud,
        ConfigScope::LocalOnly => Owner::Edge,
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigScope, Owner, owner_of};

    #[test]
    fn cloud_owns_shared_and_tenant_config() {
        assert_eq!(owner_of(ConfigScope::Shared), Owner::Cloud);
        assert_eq!(owner_of(ConfigScope::Tenant), Owner::Cloud);
    }

    #[test]
    fn edge_owns_local_only_config() {
        assert_eq!(owner_of(ConfigScope::LocalOnly), Owner::Edge);
    }
}
