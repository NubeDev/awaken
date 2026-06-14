//! Resolve the runtime profile from `RUBIX_PROFILE` within what the compiled
//! cargo features allow. STACK-DEISGN.md: "cargo-feature + runtime-config
//! selected" — the feature gate is the hard capability boundary, the env var
//! selects among the compiled-in options.

use super::kind::UnknownProfile;
use super::{Profile, ProfileKind};

/// A profile could not be selected at boot.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// `RUBIX_PROFILE` held a token that is not a known profile.
    #[error(transparent)]
    Unknown(#[from] UnknownProfile),
    /// The requested profile is a known profile but was not compiled into this
    /// build (its cargo feature is off).
    #[error("profile {0} is not compiled into this build; rebuild with --features {0}")]
    NotCompiled(ProfileKind),
}

/// True when the given profile kind was compiled into this build.
const fn compiled(kind: ProfileKind) -> bool {
    match kind {
        ProfileKind::Edge => cfg!(feature = "edge"),
        ProfileKind::Cloud => cfg!(feature = "cloud"),
    }
}

/// The profile selected when `RUBIX_PROFILE` is unset: prefer `edge` when its
/// feature is on (the default build), otherwise `cloud`. A build with neither
/// feature fails the `compile_error!` in this crate's `lib.rs`, so one of these
/// is always available.
const fn default_kind() -> ProfileKind {
    if cfg!(feature = "edge") {
        ProfileKind::Edge
    } else {
        ProfileKind::Cloud
    }
}

/// Resolve the profile from a raw `RUBIX_PROFILE` value (`None` = unset).
///
/// Separated from the env read so it is directly testable across feature sets.
pub fn resolve(raw: Option<&str>) -> Result<Profile, ProfileError> {
    let kind = match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(token) => token.parse::<ProfileKind>()?,
        None => default_kind(),
    };
    if !compiled(kind) {
        return Err(ProfileError::NotCompiled(kind));
    }
    Ok(Profile::defaults(kind))
}

/// Resolve the profile from the process environment (`RUBIX_PROFILE`).
pub fn select() -> Result<Profile, ProfileError> {
    let raw = std::env::var("RUBIX_PROFILE").ok();
    resolve(raw.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_resolves_to_the_compiled_default() {
        let p = resolve(None).unwrap();
        assert_eq!(p.kind, default_kind());
    }

    #[test]
    fn blank_is_treated_as_unset() {
        assert_eq!(resolve(Some("   ")).unwrap().kind, default_kind());
    }

    #[test]
    fn unknown_token_is_rejected() {
        assert!(matches!(
            resolve(Some("gateway")),
            Err(ProfileError::Unknown(_))
        ));
    }

    #[cfg(feature = "edge")]
    #[test]
    fn edge_resolves_when_compiled() {
        let p = resolve(Some("edge")).unwrap();
        assert_eq!(p.kind, ProfileKind::Edge);
    }

    #[cfg(feature = "cloud")]
    #[test]
    fn cloud_resolves_when_compiled() {
        let p = resolve(Some("cloud")).unwrap();
        assert_eq!(p.kind, ProfileKind::Cloud);
    }

    #[cfg(not(feature = "cloud"))]
    #[test]
    fn cloud_rejected_when_not_compiled() {
        assert!(matches!(
            resolve(Some("cloud")),
            Err(ProfileError::NotCompiled(ProfileKind::Cloud))
        ));
    }

    #[cfg(not(feature = "edge"))]
    #[test]
    fn edge_rejected_when_not_compiled() {
        assert!(matches!(
            resolve(Some("edge")),
            Err(ProfileError::NotCompiled(ProfileKind::Edge))
        ));
    }
}
