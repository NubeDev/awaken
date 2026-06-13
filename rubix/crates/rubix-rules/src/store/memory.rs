//! An in-memory [`RuleStore`] for tests and the integrating session's fixtures.

use std::collections::HashMap;

use super::load::RuleStore;
use super::record::StoredRule;
use crate::error::RuleError;

/// A [`RuleStore`] backed by an in-memory map keyed by rule name.
///
/// Not a production backend (it is unscoped and static); it exists so the engine
/// and composition path are exercisable without a database. The integrating
/// session swaps in the real, tenant-scoped store.
#[derive(Debug, Default, Clone)]
pub struct MemoryRuleStore {
    by_name: HashMap<String, StoredRule>,
}

impl MemoryRuleStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a rule, keyed by its name.
    pub fn insert(&mut self, rule: StoredRule) -> &mut Self {
        self.by_name.insert(rule.name.clone(), rule);
        self
    }

    /// Builder-style insert.
    pub fn with(mut self, rule: StoredRule) -> Self {
        self.insert(rule);
        self
    }
}

impl RuleStore for MemoryRuleStore {
    fn load(&self, name: &str) -> Result<StoredRule, RuleError> {
        self.by_name
            .get(name)
            .cloned()
            .ok_or_else(|| RuleError::Resolve(format!("no stored rule named `{name}`")))
    }
}
