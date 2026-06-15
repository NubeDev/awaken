//! Integration: the cloud build selects per-tenant (WS-14, `--features cloud`).
//!
//! Under cloud the namespace is resolved per tenant (`rubix/docs/SCOPE.md`, "Edge
//! and cloud profiles"): two distinct tenants resolve to two distinct namespaces,
//! and a request that carries no tenant is rejected rather than collapsing onto a
//! shared namespace. The whole file is compiled only under `--features cloud` — an
//! edge-only build has no cloud code path to test.

#![cfg(feature = "cloud")]

#[path = "fixture.rs"]
mod fixture;

use fixture::boot;
use rubix_server::profile::{NamespaceStrategy, ProfileError, select};

#[tokio::test]
async fn cloud_boots_per_tenant_and_isolates_tenant_namespaces() {
    let profile = select("cloud").expect("cloud is compiled in");
    let state = boot("profile_cloud_boot", profile).await;

    assert_eq!(state.profile.namespace_strategy, NamespaceStrategy::PerTenant);
    assert!(state.profile.is_multi_tenant());
    assert!(state.profile.auth_required);

    let acme = state
        .profile
        .resolve_namespace(&state.namespace, Some("acme"))
        .expect("cloud resolves acme");
    let globex = state
        .profile
        .resolve_namespace(&state.namespace, Some("globex"))
        .expect("cloud resolves globex");
    assert_ne!(acme, globex, "distinct tenants get distinct namespaces");

    // A request with no tenant must be rejected under cloud, not collapsed.
    let err = state
        .profile
        .resolve_namespace(&state.namespace, None)
        .expect_err("cloud rejects a tenant-less request");
    assert_eq!(err, ProfileError::TenantRequired);
}
