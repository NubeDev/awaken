//! The table-backed [`rubix_rules::RuleStore`]: resolves stored rules by name
//! within one org for a board's rule node and its `rule(name, …)` composition.
//!
//! `rubix-rules` is standalone — it loads rules through this trait rather than
//! touching the database. This adapter binds the abstract `load(name)` to the
//! org-scoped `rules` table via [`Store::load_rule`]. Resolution is fail-closed:
//! a missing name is a [`rubix_rules::RuleError::Resolve`], never a silent skip,
//! so a composed rule that does not exist fails the rule.

use rubix_rules::{RuleError, RuleStore, StoredRule};

use crate::store::{Store, StoreError};

/// A [`RuleStore`] scoped to one org, backed by the relational store.
pub struct TableRuleStore {
    store: Store,
    org: String,
}

impl TableRuleStore {
    pub fn new(store: Store, org: impl Into<String>) -> Self {
        Self {
            store,
            org: org.into(),
        }
    }
}

impl RuleStore for TableRuleStore {
    fn load(&self, name: &str) -> Result<StoredRule, RuleError> {
        match self.store.load_rule(&self.org, name) {
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
