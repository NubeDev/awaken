//! The rule's return type and the `finding` constructor.
//!
//! This is the spark divergence from RW-06: a rule returns a *decision*, not a
//! reshaped frame. A [`RuleResult`] is `flagged` / `severity` / `message` plus
//! an optional `value` — a score a composing rule can read without re-deriving
//! it (the same reuse motivation that, in the full design, splits functions from
//! rules).
//!
//! The `finding(severity, message)` constructor is registered into Rhai; a
//! script that returns nothing flagged yields the non-flagged default, which is
//! a normal outcome and not an error.

use serde::{Deserialize, Serialize};

use crate::severity::Severity;

/// A rule's verdict over the caller-supplied frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleResult {
    /// Whether the rule decided this is a finding.
    pub flagged: bool,
    /// The finding severity. Meaningful when `flagged`; carried regardless so a
    /// composing rule can read an unflagged rule's intended severity.
    pub severity: Severity,
    /// Human-facing message for the finding.
    pub message: String,
    /// Optional score/number a composing rule can read without re-deriving it.
    pub value: Option<f64>,
}

impl RuleResult {
    /// A flagged finding at `severity` carrying `message`. Backs `finding(...)`.
    pub fn finding(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            flagged: true,
            severity,
            message: message.into(),
            value: None,
        }
    }

    /// The non-flagged result: the rule ran and found nothing. Not an error.
    pub fn clear() -> Self {
        Self {
            flagged: false,
            severity: Severity::Info,
            message: String::new(),
            value: None,
        }
    }

    /// Attach a score the result carries for a composing rule to read.
    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }
}

impl Default for RuleResult {
    fn default() -> Self {
        Self::clear()
    }
}
