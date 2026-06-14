//! The rules-crate error domain.
//!
//! `rubix-rules` owns the *decision*: it binds DataFusion window values into a
//! Rhai script, runs it, then records the insight through the WS-05 gate,
//! publishes the data-change event on the WS-07 bus, and emits the WS-08 span
//! tree (`rubix/STACK-DEISGN.md`, "Rhai owns the decision; DataFusion owns the
//! data"). Its failures are distinct from the gate/bus/query domains it composes:
//! a script that fails to compile or evaluate, a decision the script did not
//! produce in the expected shape, an input binding that could not be resolved,
//! and the wrapped failures of the gate (record), the query surface (window
//! values), and the trace plane (span persistence). Each converts into the
//! project [`Error`](rubix_core::Error) at the crate boundary so callers chain
//! with `.context()` (CLAUDE.md "Key Patterns").

use rubix_core::Error;

/// Convenience alias for the rules-crate result.
pub type Result<T> = std::result::Result<T, RuleError>;

/// A failure raised while binding, evaluating, recording, or tracing a rule.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RuleError {
    /// The Rhai script failed to compile into an executable AST.
    #[error("rule compile error: {0}")]
    Compile(String),

    /// The Rhai script failed at runtime, or produced no/invalid decision.
    #[error("rule evaluation error: {0}")]
    Evaluate(String),

    /// A named input binding could not be resolved to a window value.
    #[error("rule binding error: {0}")]
    Binding(String),

    /// A sub-rule named by a composing rule is not registered.
    #[error("rule not found: {0}")]
    NotFound(String),

    /// Recording the insight through the WS-05 gate failed.
    #[error("insight record error: {0}")]
    Record(String),

    /// Reading the DataFusion window values that feed the rule failed.
    #[error("window value error: {0}")]
    Window(String),

    /// Persisting the per-evaluation span tree on the WS-08 trace plane failed.
    #[error("span persist error: {0}")]
    Span(String),
}

impl From<RuleError> for Error {
    fn from(error: RuleError) -> Self {
        Error::Store(error.to_string())
    }
}
