//! The cycle and composition-depth guard, enforced at call time.

use std::sync::{Arc, Mutex};

use crate::error::RuleError;

/// Default cap on composition nesting (rules calling rules).
///
/// Independent of `set_max_call_levels` (which counts *all* Rhai function calls,
/// including primitive chaining): this counts only `rule()` hops, the depth an
/// author reasons about. Kept well below the call-level cap so over-depth
/// surfaces as a clear resolve error, not an opaque recursion-limit error.
pub const DEFAULT_MAX_DEPTH: usize = 8;

/// Tracks the active `rule()` call stack to reject cycles and over-depth.
///
/// Shared (`Arc<Mutex<…>>`) because the guard is captured by the `rule()` closure
/// registered into a sandbox engine, and composition may build sub-engines that
/// share the same guard. The mutex is uncontended in the single-threaded run
/// path; it exists to satisfy the `Send + Sync` the Rhai `sync` feature requires.
#[derive(Debug, Clone)]
pub struct Guard {
    stack: Arc<Mutex<Vec<String>>>,
    max_depth: usize,
}

impl Guard {
    /// A guard with the given composition-depth cap.
    pub fn new(max_depth: usize) -> Self {
        Self {
            stack: Arc::new(Mutex::new(Vec::new())),
            max_depth,
        }
    }

    /// Enter `name`, or return a [`RuleError::Resolve`] on a cycle / over-depth.
    ///
    /// A cycle is `name` already present on the active stack (direct or
    /// transitive self-call). Over-depth is the stack already at the cap. Both
    /// are distinct resolve errors — never a hang or panic.
    pub fn enter(&self, name: &str) -> Result<(), RuleError> {
        let mut stack = self.lock();
        if stack.iter().any(|n| n == name) {
            let mut chain = stack.clone();
            chain.push(name.to_string());
            return Err(RuleError::Resolve(format!(
                "composition cycle: {}",
                chain.join(" -> ")
            )));
        }
        if stack.len() >= self.max_depth {
            return Err(RuleError::Resolve(format!(
                "composition depth cap {} exceeded at `{name}`",
                self.max_depth
            )));
        }
        stack.push(name.to_string());
        Ok(())
    }

    /// Leave the most recently entered rule.
    pub fn leave(&self) {
        self.lock().pop();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        // A poisoned mutex means a prior panic in the run path; recover the inner
        // stack rather than propagate a panic across the Rhai edge.
        self.stack.lock().unwrap_or_else(|e| e.into_inner())
    }
}
