//! The table-backed [`rubix_rules::RuleStore`]: resolves stored rules by name
//! within a board's scope for its rule node and `rule(name, …)` composition.
//!
//! `rubix-rules` is standalone — it loads rules through this trait rather than
//! touching the database. This adapter binds the abstract `load(name)` to the
//! scoped `rules` table via [`Store::load_rule`], which applies the resolution
//! precedence: a **site-scoped** rule wins, else the **org-level** one of the
//! same name. The board's org/site come from its graph keyexprs
//! ([`rubix_flow::BoardGraph::tenant_org`]/`tenant_site`). Resolution is
//! fail-closed: a missing name is a [`rubix_rules::RuleError::Resolve`], never a
//! silent skip, so a composed rule that does not exist fails the rule.

use rubix_rules::{RuleError, RuleStore, StoredRule};

use crate::store::{Store, StoreError};

/// A [`RuleStore`] scoped to one org and (optionally) one site, backed by the
/// relational store. The site is carried as a slug; `load` resolves it to a
/// site id via the board's `{org}/{site}` prefix.
pub struct TableRuleStore {
    store: Store,
    org: String,
    /// Site slug from the board graph, when the board acts on one site. `None`
    /// for an org-level board → only org-level rules resolve.
    site: Option<String>,
}

impl TableRuleStore {
    pub fn new(store: Store, org: impl Into<String>, site: Option<String>) -> Self {
        Self {
            store,
            org: org.into(),
            site,
        }
    }
}

impl RuleStore for TableRuleStore {
    fn load(&self, name: &str) -> Result<StoredRule, RuleError> {
        // Resolve the board's site slug to a site id (best-effort): a board on
        // `org/site/…` resolves that site's rule first, then the org-level one.
        // An unresolvable site falls through to org-level resolution.
        let site_id = self.site.as_ref().and_then(|site| {
            self.store
                .site_id_by_prefix(&format!("{}/{}", self.org, site))
                .ok()
        });
        match self.store.load_rule(&self.org, site_id, name) {
            Ok(rule) => Ok(rule.into_stored()),
            // Fail closed: a missing name is a resolve error the engine surfaces
            // as a clean composition failure, not a hang or a silent skip.
            Err(StoreError::NotFound(_)) => Err(RuleError::Resolve(format!(
                "stored rule `{name}` not found in org `{}`",
                self.org
            ))),
            Err(e) => Err(RuleError::Resolve(format!("load rule `{name}`: {e}"))),
        }
    }
}
