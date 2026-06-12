//! Tenant scope a tool-calling run is confined to: the `{org}/{site}` a run
//! acts within. STACK-DEISGN.md "Tenancy: org/site hierarchy mirrors into awaken
//! `ScopeId`" — a run carries the org/site it was activated for, and its tools
//! refuse any point/board keyexpr outside that prefix, so an agent invoked for
//! one tenant cannot reach another's points even through its tools.
//!
//! The org/site → `ScopeId` mapping is deterministic and reversible: the scope
//! id is the `{org}/{site}` prefix string, which is also the leading two
//! segments of every point keyexpr (`{org}/{site}/{equip-path}/{point}`). Tool
//! enforcement reuses the path-boundary shape of
//! `rubix_driver::Capability::covers` (lifted here to avoid a driver dependency
//! in the tool layer): a key is covered when it equals the prefix or sits beneath
//! it on a `/` boundary — a sibling that merely shares a string prefix never is.

use serde::{Deserialize, Serialize};

/// The `{org}/{site}` a run is confined to. Maps deterministically onto awaken's
/// tenant `ScopeId` via [`TenantScope::scope_id`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantScope {
    org: String,
    site: String,
}

impl TenantScope {
    /// Build a scope from an org and site slug. Both are keyexpr path segments.
    pub fn new(org: impl Into<String>, site: impl Into<String>) -> Self {
        Self {
            org: org.into(),
            site: site.into(),
        }
    }

    /// Parse a `{org}/{site}` prefix into a scope. Returns `None` unless the
    /// prefix is exactly two non-empty, slash-free segments — the shape every
    /// point keyexpr starts with.
    pub fn parse_prefix(prefix: &str) -> Option<Self> {
        let (org, site) = prefix.split_once('/')?;
        if org.is_empty() || site.is_empty() || site.contains('/') {
            return None;
        }
        Some(Self::new(org, site))
    }

    /// The `{org}/{site}` prefix: the leading two segments of a point keyexpr and
    /// the literal awaken `ScopeId` value for this tenant.
    pub fn prefix(&self) -> String {
        format!("{}/{}", self.org, self.site)
    }

    /// The deterministic awaken `ScopeId` for this tenant. Equal to the
    /// `{org}/{site}` prefix, so a run's scope id round-trips to the keyexpr
    /// prefix its tools gate on.
    pub fn scope_id(&self) -> String {
        self.prefix()
    }

    /// True when `keyexpr` falls within this tenant: equal to the `{org}/{site}`
    /// prefix or beneath it on a `/` boundary. A sibling site that merely shares
    /// a string prefix (`nube/hq2` vs `nube/hq`) is never covered.
    pub fn covers(&self, keyexpr: &str) -> bool {
        let prefix = self.prefix();
        keyexpr == prefix
            || keyexpr
                .strip_prefix(&prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_id_is_the_org_site_prefix() {
        let s = TenantScope::new("nube", "hq");
        assert_eq!(s.scope_id(), "nube/hq");
        assert_eq!(s.prefix(), "nube/hq");
    }

    #[test]
    fn covers_self_and_descendants_only() {
        let s = TenantScope::new("nube", "hq");
        assert!(s.covers("nube/hq"));
        assert!(s.covers("nube/hq/ahu-3/fan"));
        assert!(s.covers("nube/hq/ahu-3/fan/cur"));
        // Sibling site sharing a string prefix is not covered.
        assert!(!s.covers("nube/hq2/ahu-3/fan"));
        // Different site / org never covered.
        assert!(!s.covers("nube/dc1/ahu-3/fan"));
        assert!(!s.covers("acme/hq/ahu-3/fan"));
    }

    #[test]
    fn parse_prefix_requires_two_segments() {
        assert_eq!(
            TenantScope::parse_prefix("nube/hq"),
            Some(TenantScope::new("nube", "hq"))
        );
        // A full keyexpr is more than two segments.
        assert!(TenantScope::parse_prefix("nube/hq/ahu-3").is_none());
        assert!(TenantScope::parse_prefix("nube").is_none());
        assert!(TenantScope::parse_prefix("nube/").is_none());
        assert!(TenantScope::parse_prefix("/hq").is_none());
    }
}
