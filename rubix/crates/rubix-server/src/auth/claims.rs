//! OIDC JWT claims and their projection onto a [`Principal`]. The platform reads
//! the standard `sub`/`iss`/`exp` plus the org/team/site scope and role claims
//! an issuer is configured to emit (STACK-DEISGN.md "extract subject + org/team/
//! site claims").

use serde::Deserialize;

use super::principal::{Principal, Role};
use super::scope::Scope;

/// The subset of an OIDC token the platform consumes. `exp`/`iss` are validated
/// by the verifier's [`jsonwebtoken::Validation`]; the scope/role claims are
/// project-specific and default to the narrowest interpretation when absent.
#[derive(Debug, Deserialize)]
pub struct Claims {
    /// Stable subject identity.
    pub sub: String,
    /// Org claim; when absent the token has no org binding (global), which the
    /// issuer should only emit for trusted operator/service identities.
    #[serde(default)]
    pub org: Option<String>,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub site: Option<String>,
    /// Role claim; defaults to the read-only `viewer` when the issuer omits it,
    /// so a misconfigured token never silently gains write access.
    #[serde(default)]
    pub role: Option<String>,
}

impl Claims {
    /// Project validated claims onto a [`Principal`]. Fails closed: a malformed
    /// scope hierarchy or an unknown role token is rejected rather than widened.
    pub fn into_principal(self) -> Result<Principal, String> {
        let scope = Scope {
            org: self.org,
            team: self.team,
            site: self.site,
        };
        scope.validate().map_err(str::to_string)?;
        let role = match self.role.as_deref() {
            None => Role::Viewer,
            Some(token) => Role::parse(token).ok_or_else(|| format!("unknown role `{token}`"))?,
        };
        Ok(Principal {
            subject: self.sub,
            scope,
            role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_claims_project_to_a_scoped_operator() {
        let claims = Claims {
            sub: "u1".into(),
            org: Some("nube".into()),
            team: Some("ops".into()),
            site: Some("hq".into()),
            role: Some("operator".into()),
        };
        let p = claims.into_principal().unwrap();
        assert_eq!(p.subject, "u1");
        assert_eq!(p.role, Role::Operator);
        assert_eq!(p.scope, Scope { org: Some("nube".into()), team: Some("ops".into()), site: Some("hq".into()) });
    }

    #[test]
    fn missing_role_defaults_to_viewer() {
        let claims = Claims {
            sub: "u1".into(),
            org: Some("nube".into()),
            team: None,
            site: None,
            role: None,
        };
        assert_eq!(claims.into_principal().unwrap().role, Role::Viewer);
    }

    #[test]
    fn unknown_role_is_rejected() {
        let claims = Claims {
            sub: "u1".into(),
            org: None,
            team: None,
            site: None,
            role: Some("root".into()),
        };
        assert!(claims.into_principal().is_err());
    }

    #[test]
    fn malformed_scope_hierarchy_is_rejected() {
        let claims = Claims {
            sub: "u1".into(),
            org: None,
            team: Some("ops".into()),
            site: None,
            role: Some("operator".into()),
        };
        assert!(claims.into_principal().is_err());
    }
}
