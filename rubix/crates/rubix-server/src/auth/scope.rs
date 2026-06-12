//! RBAC scope: the org→team→site hierarchy a principal is confined to.
//! STACK-DEISGN.md "RBAC with org → team → site/app scoping". Site matching
//! reuses the path-boundary shape of [`rubix_driver::Capability::covers`] rather
//! than inventing a parallel scheme: a broader scope covers a narrower request,
//! never a sibling that merely shares a string prefix.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The org/team/site a principal may act within. A scope grants access to its
/// own level and everything beneath it: an org-level principal reaches every
/// team and site under that org; a site-level principal reaches only that site.
///
/// `team`/`site` are `None` at the broader levels. The hierarchy is strict —
/// a `site` may only be set when a `team` is, and a `team` only when an `org`
/// is — so an unscoped (`org: None`) principal is global (operators, internal
/// service accounts), validated by [`Scope::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub struct Scope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
}

impl Scope {
    /// A global scope: no org/team/site bound. Reaches every resource.
    pub fn global() -> Self {
        Scope::default()
    }

    /// An org-wide scope.
    pub fn org(org: impl Into<String>) -> Self {
        Scope {
            org: Some(org.into()),
            team: None,
            site: None,
        }
    }

    /// True when this scope is unbounded (global). Such a principal passes every
    /// RBAC site check; reserved for operators and internal service accounts.
    pub fn is_global(&self) -> bool {
        self.org.is_none()
    }

    /// Reject an ill-formed hierarchy: a `team` without an `org`, or a `site`
    /// without a `team`. The levels nest; a gap is a malformed claim.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.team.is_some() && self.org.is_none() {
            return Err("scope team set without an org");
        }
        if self.site.is_some() && self.team.is_none() {
            return Err("scope site set without a team");
        }
        Ok(())
    }

    /// True when this scope authorizes access to `target`: every level this
    /// scope binds must equal the same level on `target`, and `target` may be
    /// equal-or-narrower. A global scope (no org) covers everything.
    ///
    /// This is the [`rubix_driver::Capability::covers`] idea lifted to the
    /// org/team/site tuple — a broader grant covers a narrower request, a
    /// sibling never does.
    pub fn covers(&self, target: &Scope) -> bool {
        level_covers(&self.org, &target.org)
            && level_covers(&self.team, &target.team)
            && level_covers(&self.site, &target.site)
    }
}

/// One hierarchy level: a `None` bound on the grant is a wildcard (covers any
/// target at that level); a bound grant requires the target to match exactly.
fn level_covers(grant: &Option<String>, target: &Option<String>) -> bool {
    match grant {
        None => true,
        Some(g) => target.as_deref() == Some(g.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(org: Option<&str>, team: Option<&str>, site: Option<&str>) -> Scope {
        Scope {
            org: org.map(str::to_string),
            team: team.map(str::to_string),
            site: site.map(str::to_string),
        }
    }

    #[test]
    fn global_scope_covers_any_target() {
        let g = Scope::global();
        assert!(g.is_global());
        assert!(g.covers(&scope(Some("nube"), Some("ops"), Some("hq"))));
        assert!(g.covers(&Scope::global()));
    }

    #[test]
    fn org_scope_covers_its_descendants_only() {
        let s = Scope::org("nube");
        assert!(s.covers(&scope(Some("nube"), None, None)));
        assert!(s.covers(&scope(Some("nube"), Some("ops"), Some("hq"))));
        // Sibling org is never covered.
        assert!(!s.covers(&scope(Some("acme"), None, None)));
        // A bound grant is not covered by a broader-than-it target request.
        assert!(!s.covers(&Scope::global()));
    }

    #[test]
    fn site_scope_is_the_narrowest() {
        let s = scope(Some("nube"), Some("ops"), Some("hq"));
        assert!(s.covers(&scope(Some("nube"), Some("ops"), Some("hq"))));
        assert!(!s.covers(&scope(Some("nube"), Some("ops"), Some("dc1"))));
        assert!(!s.covers(&scope(Some("nube"), Some("facilities"), Some("hq"))));
    }

    #[test]
    fn validate_rejects_hierarchy_gaps() {
        assert!(scope(None, Some("ops"), None).validate().is_err());
        assert!(scope(Some("nube"), None, Some("hq")).validate().is_err());
        assert!(scope(Some("nube"), Some("ops"), Some("hq")).validate().is_ok());
        assert!(Scope::global().validate().is_ok());
    }
}
