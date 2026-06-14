//! The rule registry — the set a composing rule resolves sub-rules from.
//!
//! Rules are composable: a script invokes another rule by id
//! (`rubix/docs/SCOPE.md`, "Rhai — rules and insights"). Composition therefore
//! needs a lookup from rule id to [`Rule`]; this registry is that lookup. It is a
//! plain in-memory map owned by the caller (an edge node loads its rule pack into
//! one), keyed by [`Rule::id`]. Resolution **fails closed**: an unknown sub-rule
//! id is a [`RuleError::NotFound`], never a silent skip, so a composed decision
//! can never quietly omit a missing child (CLAUDE.md "Core Rules").

use std::collections::HashMap;

use crate::error::{Result, RuleError};
use crate::rule::define::Rule;

/// An in-memory set of rules addressable by id for composition.
#[derive(Debug, Clone, Default)]
pub struct RuleRegistry {
    rules: HashMap<String, Rule>,
}

impl RuleRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `rule`, replacing any rule already registered under its id.
    pub fn insert(&mut self, rule: Rule) {
        self.rules.insert(rule.id.as_str().to_owned(), rule);
    }

    /// Resolve `id` to its rule, or fail closed if it is not registered.
    ///
    /// # Errors
    /// Returns [`RuleError::NotFound`] if no rule is registered under `id`.
    pub fn resolve(&self, id: &str) -> Result<&Rule> {
        self.rules
            .get(id)
            .ok_or_else(|| RuleError::NotFound(id.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::Id;

    use crate::rule::define::Rule;

    use super::RuleRegistry;

    fn rule(id: &str) -> Rule {
        Rule::new(Id::from_raw(id), "true", Vec::new(), "out")
    }

    #[test]
    fn resolve_returns_a_registered_rule() {
        let mut registry = RuleRegistry::new();
        registry.insert(rule("child"));
        assert_eq!(registry.resolve("child").unwrap().id.as_str(), "child");
    }

    #[test]
    fn an_unknown_id_fails_closed() {
        let registry = RuleRegistry::new();
        let err = registry.resolve("missing").unwrap_err();
        assert!(matches!(err, crate::error::RuleError::NotFound(_)));
    }

    #[test]
    fn insert_replaces_a_prior_rule_of_the_same_id() {
        let mut registry = RuleRegistry::new();
        registry.insert(Rule::new(Id::from_raw("r"), "1", Vec::new(), "a"));
        registry.insert(Rule::new(Id::from_raw("r"), "2", Vec::new(), "b"));
        assert_eq!(registry.resolve("r").unwrap().output, "b");
    }
}
