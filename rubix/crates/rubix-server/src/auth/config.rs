//! Auth configuration resolved at boot. The cloud profile requires authenticated
//! requests (`Profile::auth_required`); the edge profile leaves auth off so
//! local/offline stations keep working (STACK-DEISGN.md "auth required" on cloud,
//! absent on edge). OIDC settings come from `RUBIX_OIDC_ISSUER` / `RUBIX_OIDC_JWKS`.

/// The auth posture for this node, resolved from the profile and env.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthConfig {
    /// Auth is off: requests pass without a principal. Edge default, and the
    /// only posture available when no OIDC issuer is configured on an
    /// auth-optional profile.
    Disabled,
    /// Auth is enforced. Bearer tokens are validated as OIDC JWTs (against the
    /// `issuer`/`jwks_url` JWKS) or as PATs (against the `tokens` store).
    Enabled { issuer: String, jwks_url: String },
}

impl AuthConfig {
    /// True when every request must carry a valid bearer.
    pub fn is_enabled(&self) -> bool {
        matches!(self, AuthConfig::Enabled { .. })
    }

    /// Resolve the posture from the profile's `auth_required` flag and the OIDC
    /// env vars. Fails closed: a profile that requires auth but has no issuer
    /// configured is a boot error, not a silent open door.
    ///
    /// `issuer`/`jwks_url` are passed in (read from env by the caller) so this
    /// is unit-testable without touching the process environment.
    pub fn resolve(
        auth_required: bool,
        issuer: Option<&str>,
        jwks_url: Option<&str>,
    ) -> Result<Self, ConfigError> {
        let issuer = issuer.map(str::trim).filter(|s| !s.is_empty());
        let jwks_url = jwks_url.map(str::trim).filter(|s| !s.is_empty());
        match (issuer, jwks_url) {
            (Some(issuer), Some(jwks_url)) => Ok(AuthConfig::Enabled {
                issuer: issuer.to_string(),
                jwks_url: jwks_url.to_string(),
            }),
            (issuer, jwks) => {
                if auth_required {
                    return Err(ConfigError::Required {
                        have_issuer: issuer.is_some(),
                        have_jwks: jwks.is_some(),
                    });
                }
                Ok(AuthConfig::Disabled)
            }
        }
    }
}

/// Auth could not be configured for a profile that requires it.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The profile requires auth but the OIDC issuer/JWKS env is incomplete.
    #[error(
        "this profile requires auth but RUBIX_OIDC_ISSUER/RUBIX_OIDC_JWKS are not both set \
         (issuer set: {have_issuer}, jwks set: {have_jwks})"
    )]
    Required { have_issuer: bool, have_jwks: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_oidc_config_enables_auth() {
        let cfg = AuthConfig::resolve(true, Some("https://iss"), Some("https://iss/jwks")).unwrap();
        assert!(cfg.is_enabled());
    }

    #[test]
    fn edge_without_oidc_is_disabled() {
        let cfg = AuthConfig::resolve(false, None, None).unwrap();
        assert_eq!(cfg, AuthConfig::Disabled);
    }

    #[test]
    fn cloud_without_oidc_fails_closed() {
        assert!(matches!(
            AuthConfig::resolve(true, None, None),
            Err(ConfigError::Required { .. })
        ));
        // Half-configured is just as closed.
        assert!(AuthConfig::resolve(true, Some("https://iss"), None).is_err());
    }

    #[test]
    fn blank_env_is_treated_as_unset() {
        assert_eq!(
            AuthConfig::resolve(false, Some("  "), Some("")).unwrap(),
            AuthConfig::Disabled
        );
    }
}
