//! Deployment profiles: the same binary configured as an edge node or a cloud
//! deployment (`rubix/docs/SCOPE.md`, "Edge and cloud profiles";
//! `rubix/docs/sessions/WS-14.md`).
//!
//! A profile is chosen at two layers. A *cargo feature* (`edge` default / `cloud`)
//! decides which profiles are compiled into the binary; at least one must be —
//! [the guard below](#compile-error) refuses a build with neither. At boot,
//! `RUBIX_PROFILE` then [`select`]s among the compiled-in profiles, rejecting an
//! unknown or uncompiled name with no silent fallback. The selected [`Profile`]
//! centralizes the per-profile defaults (store kind, namespace strategy, auth
//! required, sync on/off), is verified to have every backend it needs in the build
//! ([`verify_backends`]), and is threaded into `AppState` so the gate resolves a
//! request's tenant namespace per profile ([`resolve_tenant`]).

// <a id="compile-error"></a>
// A profile must always be compiled in — a binary with neither feature has no
// deployment shape to boot into. Fail the build, not the boot.
#[cfg(not(any(feature = "edge", feature = "cloud")))]
compile_error!(
    "rubix-server requires a deployment profile feature: enable `edge` (default) \
     or `cloud`. A build with neither has no profile to select at boot."
);

#[cfg(feature = "cloud")]
mod cloud;
mod define;
#[cfg(feature = "edge")]
mod edge;
mod resolve_tenant;
mod select;
mod verify_backends;

pub use define::{NamespaceStrategy, Profile};
pub use select::{PROFILE_ENV, default_profile, from_env, select};

/// A failure selecting, resolving, or verifying a deployment profile.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProfileError {
    /// `RUBIX_PROFILE` named a profile that does not exist.
    #[error("unknown profile `{0}` (expected `edge` or `cloud`)")]
    Unknown(String),
    /// `RUBIX_PROFILE` named a real profile whose feature was not built in.
    #[error("profile `{0}` is not compiled into this binary (rebuild with its feature)")]
    NotCompiled(String),
    /// A multi-tenant profile resolved a request that carried no tenant.
    #[error("a tenant is required under the cloud profile but none was provided")]
    TenantRequired,
    /// The selected profile requires a backend the build does not carry.
    #[error("profile `{profile}` requires the `{backend}` backend, which is not in this build")]
    MissingBackend {
        /// The profile that required the backend.
        profile: &'static str,
        /// The backend feature that is absent.
        backend: &'static str,
    },
}
