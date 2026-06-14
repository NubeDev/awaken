//! The abstract rule store the resolver loads from.

use super::record::StoredRule;
use crate::error::RuleError;

/// Where stored rules are loaded from for composition.
///
/// Abstracted so this crate is testable without a database; the integrating
/// session provides a real implementation backed by a tenant-scoped rules table
/// (and exposes the referencing-rules listing the design calls for, outside this
/// trait's hot path). Implementations are `Send + Sync` so one store can serve
/// concurrent ticks.
///
/// Resolution is **fail-closed**: a missing name is a [`RuleError::Resolve`],
/// never a silent skip — a composed rule that does not exist fails the rule.
pub trait RuleStore: Send + Sync {
    /// Load the rule named `name`, or a [`RuleError::Resolve`] if absent.
    fn load(&self, name: &str) -> Result<StoredRule, RuleError>;
}
