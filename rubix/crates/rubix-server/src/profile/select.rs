//! Map a `RUBIX_PROFILE` name to one of the compiled-in profiles.
//!
//! Selection is fail-closed (`rubix/docs/sessions/WS-14.md`): a name that is not
//! one of the compiled-in profiles is rejected at boot with a clear error, never
//! silently defaulted. A name that is *known* but whose feature was not built
//! (e.g. `cloud` on an edge-only binary) is rejected distinctly from an outright
//! unknown name, so the operator can tell a typo from a missing build. With no
//! `RUBIX_PROFILE` set, the binary uses the default profile — edge when built,
//! else the single compiled-in profile.

use super::ProfileError;
use super::define::Profile;

/// The environment variable selecting the deployment profile.
pub const PROFILE_ENV: &str = "RUBIX_PROFILE";

/// Resolve the deployment profile from `RUBIX_PROFILE` (or the default if unset).
///
/// Reads the environment once at boot. When the variable is unset the default
/// profile is used: edge when the `edge` feature is built, otherwise the single
/// compiled-in profile. When set, the value must name a compiled-in profile.
///
/// # Errors
/// - [`ProfileError::Unknown`] if the name is not a recognized profile.
/// - [`ProfileError::NotCompiled`] if the name is recognized but its feature was
///   not built into this binary.
pub fn from_env() -> Result<Profile, ProfileError> {
    match std::env::var(PROFILE_ENV) {
        Ok(name) if !name.trim().is_empty() => select(name.trim()),
        _ => Ok(default_profile()),
    }
}

/// Map an explicit profile name to its compiled-in [`Profile`].
///
/// The match is exhaustive over the recognized names so an unrecognized name is
/// [`ProfileError::Unknown`]; a recognized name whose feature is absent is
/// [`ProfileError::NotCompiled`]. The `cfg`-gated arms are the only place the
/// compiled-in set is enumerated.
///
/// # Errors
/// See [`from_env`].
pub fn select(name: &str) -> Result<Profile, ProfileError> {
    match name {
        "edge" => {
            #[cfg(feature = "edge")]
            {
                Ok(super::edge::profile())
            }
            #[cfg(not(feature = "edge"))]
            {
                Err(ProfileError::NotCompiled(name.to_owned()))
            }
        }
        "cloud" => {
            #[cfg(feature = "cloud")]
            {
                Ok(super::cloud::profile())
            }
            #[cfg(not(feature = "cloud"))]
            {
                Err(ProfileError::NotCompiled(name.to_owned()))
            }
        }
        other => Err(ProfileError::Unknown(other.to_owned())),
    }
}

/// The profile used when `RUBIX_PROFILE` is unset.
///
/// Edge is the default whenever it is compiled in; an edge-less build (only
/// `cloud`) defaults to cloud. The `compile_error!` guard in the module barrel
/// guarantees at least one of these arms is live. Also the fallback `AppState`
/// constructor's profile for callers (and tests) that do not select one.
#[must_use]
pub fn default_profile() -> Profile {
    #[cfg(feature = "edge")]
    {
        super::edge::profile()
    }
    #[cfg(all(not(feature = "edge"), feature = "cloud"))]
    {
        super::cloud::profile()
    }
}

#[cfg(test)]
mod tests {
    use super::{ProfileError, select};

    #[test]
    fn an_unknown_profile_name_is_rejected() {
        let err = select("staging").unwrap_err();
        assert!(matches!(err, ProfileError::Unknown(name) if name == "staging"));
    }

    #[cfg(feature = "edge")]
    #[test]
    fn edge_resolves_when_compiled() {
        use super::super::define::NamespaceStrategy;
        let p = select("edge").unwrap();
        assert_eq!(p.namespace_strategy, NamespaceStrategy::Single);
    }

    #[cfg(feature = "cloud")]
    #[test]
    fn cloud_resolves_when_compiled() {
        use super::super::define::NamespaceStrategy;
        let p = select("cloud").unwrap();
        assert_eq!(p.namespace_strategy, NamespaceStrategy::PerTenant);
    }

    #[cfg(not(feature = "cloud"))]
    #[test]
    fn cloud_is_rejected_as_uncompiled_on_an_edge_only_build() {
        let err = select("cloud").unwrap_err();
        assert!(matches!(err, ProfileError::NotCompiled(name) if name == "cloud"));
    }
}
