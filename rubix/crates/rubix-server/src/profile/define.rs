//! The per-profile configuration read once at boot and threaded into `AppState`.
//!
//! A [`Profile`] centralizes the deployment defaults that differ between an edge
//! node and a cloud deployment (`rubix/docs/SCOPE.md`, "Edge and cloud profiles";
//! `rubix/docs/sessions/WS-14.md`): which store kind backs it, how it resolves a
//! tenant namespace, whether authentication is required, and whether the
//! edge↔cloud sync shipper is on. The per-profile values come from [`edge`] and
//! [`cloud`]; this module only defines the shape. Selection among the compiled-in
//! profiles is [`select`](super::select), which never invents a profile — it maps
//! a known name to one of these or rejects it.

use rubix_core::Profile as ProfileKind;

/// How a request's tenant namespace is resolved.
///
/// Edge has no multi-tenancy code path: every request resolves to the one
/// configured namespace (`rubix/docs/SCOPE.md`, "No multi-tenancy on edge").
/// Cloud resolves a namespace per tenant. Threading this into the gate is
/// [`resolve_tenant`](super::resolve_tenant::resolve_namespace).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceStrategy {
    /// One fixed namespace for every request (edge).
    Single,
    /// A namespace per tenant (cloud).
    PerTenant,
}

/// The deployment profile resolved at boot.
///
/// Built by [`edge`](super::edge::profile) or [`cloud`](super::cloud::profile)
/// and threaded into `AppState` so every route reads the same per-profile
/// defaults. It is a value, not a trait object — the variants differ only in
/// their data, not their behavior, so the gate branches on
/// [`namespace_strategy`](Profile::namespace_strategy) rather than dispatching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Which deployment kind this profile is — the `rubix-core` tag carried into
    /// the store/runtime config so the engine layer sees the same choice.
    pub kind: ProfileKind,
    /// How a request's tenant namespace is resolved.
    pub namespace_strategy: NamespaceStrategy,
    /// Whether a request must carry an authenticated principal. Edge allows an
    /// unauthenticated local operator; cloud always requires auth.
    pub auth_required: bool,
    /// Whether the edge↔cloud sync shipper (WS-15) runs under this profile.
    pub sync_enabled: bool,
}

impl Profile {
    /// `true` when this profile carries a multi-tenancy code path.
    ///
    /// The gate uses this to skip per-tenant resolution entirely on edge — there
    /// is no tenant to derive, so the one configured namespace is authoritative.
    #[must_use]
    pub fn is_multi_tenant(&self) -> bool {
        matches!(self.namespace_strategy, NamespaceStrategy::PerTenant)
    }
}
