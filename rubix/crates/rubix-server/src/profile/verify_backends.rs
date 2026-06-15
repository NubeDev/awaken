//! Fail the boot closed when a profile needs a backend the build does not carry.
//!
//! Cloud requires the Postgres backend (WS-10, a cloud-only connector). The
//! `cloud` feature pulls `postgres` in, so a correctly built cloud binary always
//! has it — but a misconfigured build (cloud profile selected on a binary whose
//! `postgres` feature was stripped) must not start in a degraded state. This check
//! runs once at boot, after [`select`](super::select) and before serving, and
//! returns an error rather than silently dropping the backend
//! (`rubix/docs/sessions/WS-14.md`, "fails closed at boot"). Edge requires no such
//! backend, so its check is always a pass.

use rubix_core::Profile as ProfileKind;

use super::ProfileError;
use super::define::Profile;

impl Profile {
    /// Assert every backend this profile requires is compiled into the build.
    ///
    /// A cloud profile requires the Postgres backend; if the `postgres` feature is
    /// absent the boot fails closed. Edge requires nothing here. The check reads
    /// only compile-time `cfg` flags, so it is deterministic and runs before any
    /// socket is bound.
    ///
    /// # Errors
    /// [`ProfileError::MissingBackend`] naming the absent backend when a required
    /// one is not in the build.
    pub fn verify_backends(&self) -> Result<(), ProfileError> {
        if self.kind == ProfileKind::Cloud && !cfg!(feature = "postgres") {
            return Err(ProfileError::MissingBackend {
                profile: "cloud",
                backend: "postgres",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "edge")]
    #[test]
    fn edge_requires_no_backend() {
        assert!(super::super::edge::profile().verify_backends().is_ok());
    }

    // A cloud profile is only constructible when the `cloud` feature is on, which
    // pulls in `postgres` — so on any real build the cloud check passes. The
    // fail-closed path (cloud profile, postgres absent) is unreachable from a
    // valid feature set and is asserted at the boot integration level instead.
    #[cfg(feature = "cloud")]
    #[test]
    fn cloud_with_postgres_in_the_build_passes() {
        assert!(super::super::cloud::profile().verify_backends().is_ok());
    }
}
