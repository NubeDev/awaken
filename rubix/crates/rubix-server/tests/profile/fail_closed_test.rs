//! Integration: cloud fails closed when a required backend is absent (WS-14).
//!
//! Cloud requires the Postgres backend (WS-10). `verify_backends` reads only
//! compile-time `cfg` flags, so its verdict is a function of the build: a cloud
//! profile passes iff `postgres` is in the build, and fails closed
//! ([`ProfileError::MissingBackend`]) otherwise — no degraded fallback path.
//!
//! On a correctly built cloud binary `cloud` pulls `postgres` in, so the pass
//! branch holds. The fail-closed branch is exercised on any build *without*
//! `postgres` by constructing a cloud-kind profile through the public API and
//! asserting the boot check rejects it — the same check `main` runs before binding
//! a socket.

#[cfg(not(feature = "postgres"))]
use rubix_server::profile::ProfileError;

#[cfg(feature = "cloud")]
#[test]
fn cloud_with_its_backend_in_the_build_boots() {
    let profile = rubix_server::profile::select("cloud").expect("cloud is compiled in");
    assert!(
        profile.verify_backends().is_ok(),
        "a cloud build carries postgres (the `cloud` feature pulls it in)",
    );
}

// On a build without the postgres backend, a cloud profile must fail closed. We
// can only reach this assertion on a non-postgres build, so it is gated off the
// `postgres` feature rather than the `cloud` feature (a cloud build always has
// postgres). The profile is constructed directly to model "cloud selected on a
// binary whose backend was stripped".
#[cfg(not(feature = "postgres"))]
#[test]
fn cloud_without_its_backend_fails_closed_at_boot() {
    use rubix_core::Profile as ProfileKind;
    use rubix_server::profile::{NamespaceStrategy, Profile};

    let profile = Profile {
        kind: ProfileKind::Cloud,
        namespace_strategy: NamespaceStrategy::PerTenant,
        auth_required: true,
        sync_enabled: true,
    };

    let err = profile
        .verify_backends()
        .expect_err("cloud must fail closed when postgres is absent from the build");
    assert!(
        matches!(
            err,
            ProfileError::MissingBackend {
                profile: "cloud",
                backend: "postgres"
            }
        ),
        "expected MissingBackend(cloud, postgres), got {err:?}",
    );
}
