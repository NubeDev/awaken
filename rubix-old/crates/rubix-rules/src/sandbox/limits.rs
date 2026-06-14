//! Tunable sandbox bounds.
//!
//! Defaults are conservative for a scheduled rule over a bounded, caller-capped
//! frame. The operation budget and wall-clock deadline are a single allowance
//! for the whole composition tree (see [`crate::compose`]); these per-engine
//! limits bound any one script's structural shape.

use std::time::Duration;

/// Structural limits applied to every sandboxed engine.
#[derive(Debug, Clone, Copy)]
pub struct SandboxLimits {
    /// Max Rhai operations before [`RuleError::LimitExceeded`] — bounds runaway
    /// loops. This is the per-tree budget when composition shares one engine.
    ///
    /// [`RuleError::LimitExceeded`]: crate::RuleError::LimitExceeded
    pub max_operations: u64,
    /// Max nested function-call depth. Backstops composition recursion alongside
    /// the explicit cycle/depth guard.
    pub max_call_levels: usize,
    /// Max length of any string a script builds (characters).
    pub max_string_size: usize,
    /// Max length of any array a script builds (elements).
    pub max_array_size: usize,
    /// Wall-clock budget for the whole execution. Enforced via `on_progress`.
    pub timeout: Duration,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_operations: 5_000_000,
            max_call_levels: 32,
            max_string_size: 64 * 1024,
            max_array_size: 16 * 1024,
            timeout: Duration::from_secs(5),
        }
    }
}
