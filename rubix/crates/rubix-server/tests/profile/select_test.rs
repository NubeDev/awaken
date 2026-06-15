//! Integration: profile selection is fail-closed (WS-14).
//!
//! An unknown `RUBIX_PROFILE` name is rejected with a clear error and no silent
//! fallback; the compiled-in profile resolves to its expected namespace strategy.
//! `select` is driven by name directly rather than through the process
//! environment, so these assertions are stable under the test harness's parallel
//! execution (a shared `RUBIX_PROFILE` env var would race across test binaries).

use rubix_server::profile::{NamespaceStrategy, ProfileError, select};

#[test]
fn an_unknown_profile_name_is_rejected_with_no_fallback() {
    let err = select("staging").expect_err("unknown profile must be rejected");
    assert!(
        matches!(err, ProfileError::Unknown(ref name) if name == "staging"),
        "expected Unknown(staging), got {err:?}",
    );
}

#[cfg(feature = "edge")]
#[test]
fn edge_resolves_to_a_single_namespace_strategy() {
    let profile = select("edge").expect("edge is compiled in");
    assert_eq!(profile.namespace_strategy, NamespaceStrategy::Single);
    assert!(!profile.auth_required);
    assert!(!profile.is_multi_tenant());
}

#[cfg(feature = "cloud")]
#[test]
fn cloud_resolves_to_a_per_tenant_strategy() {
    let profile = select("cloud").expect("cloud is compiled in");
    assert_eq!(profile.namespace_strategy, NamespaceStrategy::PerTenant);
    assert!(profile.auth_required);
    assert!(profile.is_multi_tenant());
}

#[cfg(not(feature = "cloud"))]
#[test]
fn cloud_is_rejected_as_uncompiled_on_an_edge_only_build() {
    let err = select("cloud").expect_err("cloud is not compiled into an edge-only build");
    assert!(
        matches!(err, ProfileError::NotCompiled(ref name) if name == "cloud"),
        "expected NotCompiled(cloud), got {err:?}",
    );
}
