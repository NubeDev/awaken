//! The rule-engine error domain.
//!
//! One `thiserror` enum spanning the four categories the design doc requires a
//! spark to distinguish: a broken rule (compile / runtime / limit) versus a
//! composition failure (resolve). An empty or non-flagged [`RuleResult`] is
//! never an error — "ran and found nothing" is a normal outcome, not a fault.
//!
//! These variants are safe to surface to a tenant, but a Rhai runtime error can
//! interpolate script strings and queried row values into its message. So the
//! categories here are treated as potentially data-bearing: a caller deciding
//! what to log versus show should treat the `String` payloads as tenant data,
//! not trusted engine text.
//!
//! [`RuleResult`]: crate::RuleResult

use thiserror::Error;

/// A failure evaluating a rule. The category is the discriminant a spark acts on.
#[derive(Debug, Error)]
pub enum RuleError {
    /// The script is malformed and never began executing.
    #[error("compile rule: {0}")]
    Compile(String),

    /// The script ran and failed (bad argument, type mismatch, thrown error).
    #[error("run rule: {0}")]
    Runtime(String),

    /// A sandbox limit tripped: operations, call levels, string/array size, or
    /// the wall-clock deadline. Distinct from [`Runtime`](Self::Runtime) so a
    /// spark can tell "the rule is too expensive" from "the rule errored".
    #[error("rule exceeded sandbox limit: {0}")]
    LimitExceeded(String),

    /// A composed `rule(name, …)` could not be resolved: the name is missing, a
    /// cycle was detected, or the composition-depth cap was exceeded. Fails
    /// closed — never a silent skip.
    #[error("resolve composed rule: {0}")]
    Resolve(String),

    /// The vectorized engine failed to compute a primitive.
    #[error("engine: {0}")]
    Engine(String),
}

impl From<datafusion::error::DataFusionError> for RuleError {
    fn from(e: datafusion::error::DataFusionError) -> Self {
        RuleError::Engine(e.to_string())
    }
}
