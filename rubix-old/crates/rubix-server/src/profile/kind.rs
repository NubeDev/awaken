//! The profile a node runs as. STACK-DEISGN.md "Single binary, two profiles":
//! one executable, cargo features plus runtime config select edge vs cloud.

use std::fmt;
use std::str::FromStr;

/// Which deployment profile a node runs as.
///
/// The variant set is fixed; which variants are *available* in a given build is
/// gated by the `edge` / `cloud` cargo features (see [`super::select`]). The
/// runtime [`super::Profile`] config carries the per-variant defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    /// Pi-class station: SQLite, local Parquet history, drivers supervised
    /// on-box, no auth. Today's behavior.
    Edge,
    /// Cloud supervisor: Postgres relational store, object-store history, auth
    /// required, no on-box drivers. The heavy backends attach behind this seam;
    /// today the cloud profile boots but fails closed where a backend is absent.
    Cloud,
}

impl ProfileKind {
    /// The canonical lowercase token used by `RUBIX_PROFILE`.
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileKind::Edge => "edge",
            ProfileKind::Cloud => "cloud",
        }
    }
}

impl fmt::Display for ProfileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `RUBIX_PROFILE` carried a token that is not a known profile.
#[derive(Debug, thiserror::Error)]
#[error("unknown profile {0:?}; expected one of: edge, cloud")]
pub struct UnknownProfile(pub String);

impl FromStr for ProfileKind {
    type Err = UnknownProfile;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "edge" => Ok(ProfileKind::Edge),
            "cloud" => Ok(ProfileKind::Cloud),
            other => Err(UnknownProfile(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_tokens_case_insensitively() {
        assert_eq!("edge".parse::<ProfileKind>().unwrap(), ProfileKind::Edge);
        assert_eq!(
            " Cloud ".parse::<ProfileKind>().unwrap(),
            ProfileKind::Cloud
        );
    }

    #[test]
    fn round_trips_through_as_str() {
        for kind in [ProfileKind::Edge, ProfileKind::Cloud] {
            assert_eq!(kind.as_str().parse::<ProfileKind>().unwrap(), kind);
        }
    }

    #[test]
    fn rejects_unknown_token() {
        let err = "gateway".parse::<ProfileKind>().unwrap_err();
        assert_eq!(err.0, "gateway");
    }
}
