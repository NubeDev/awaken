//! Integration: the edge build boots single-namespace (WS-14, `--features edge`).
//!
//! Under edge there is no multi-tenancy code path: every request resolves to the
//! one configured namespace and a tenant hint is ignored
//! (`rubix/docs/SCOPE.md`, "No multi-tenancy on edge"). Booting `AppState` under
//! the edge profile and resolving a namespace — with and without a tenant — must
//! return the configured namespace in both cases.

#![cfg(feature = "edge")]

#[path = "fixture.rs"]
mod fixture;

use fixture::{NS, boot};
use rubix_server::profile::{NamespaceStrategy, select};

#[tokio::test]
async fn edge_boots_single_namespace_and_resolves_to_the_one_tenant() {
    let profile = select("edge").expect("edge is compiled in");
    let state = boot("profile_edge_boot", profile).await;

    assert_eq!(state.profile.namespace_strategy, NamespaceStrategy::Single);
    assert!(!state.profile.is_multi_tenant());

    // No tenant carried: resolves to the configured namespace.
    let resolved = state
        .profile
        .resolve_namespace(&state.namespace, None)
        .expect("edge resolves with no tenant");
    assert_eq!(resolved, NS);

    // A tenant hint is ignored under the single-namespace edge profile.
    let resolved = state
        .profile
        .resolve_namespace(&state.namespace, Some("acme"))
        .expect("edge ignores a tenant hint");
    assert_eq!(resolved, NS);
}
