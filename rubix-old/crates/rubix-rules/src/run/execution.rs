//! The shared per-tick execution context.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::compose::{Budget, Guard};
use crate::error::RuleError;
use crate::result::RuleResult;
use crate::sandbox::{Deadline, SandboxLimits};
use crate::store::RuleStore;

/// Memo key: `(rule name, frame identity, canonical params)`.
///
/// Per the design, `rule()` results are memoized within a single tick so a
/// popular shared rule invoked by several callers runs once, not once per
/// caller. Frame identity is the [`crate::Frame`]'s stable id; params are
/// canonicalized to a string so equal maps collide.
type MemoKey = (String, u64, String);

/// Everything a rule and its composed callees share for one execution (tick).
///
/// One `Execution` is built per top-level [`run_rule`](super::run_rule) call and
/// reused by every nested `rule()` — that is what makes the operation budget,
/// deadline, cycle guard, and memo a single allowance for the whole tree.
#[derive(Clone)]
pub struct Execution {
    pub(crate) store: Arc<dyn RuleStore>,
    pub(crate) guard: Guard,
    pub(crate) budget: Budget,
    pub(crate) deadline: Deadline,
    pub(crate) limits: SandboxLimits,
    memo: Arc<Mutex<HashMap<MemoKey, RuleResult>>>,
    /// Carries a typed [`RuleError`] from inside a registered primitive back to
    /// the run loop. A primitive that fails records its categorized error here
    /// and throws a sentinel into Rhai; the run loop recovers the category so a
    /// composition `resolve` error is never flattened into a generic runtime
    /// error by Rhai's stringly-typed error channel.
    error_sink: Arc<Mutex<Option<RuleError>>>,
}

impl Execution {
    /// Build an execution sharing one allowance across the composition tree.
    pub fn new(store: Arc<dyn RuleStore>, limits: SandboxLimits, guard: Guard) -> Self {
        Self {
            store,
            guard,
            budget: Budget::new(limits.max_operations),
            deadline: Deadline::after(limits.timeout),
            limits,
            memo: Arc::new(Mutex::new(HashMap::new())),
            error_sink: Arc::new(Mutex::new(None)),
        }
    }

    /// Record a typed error from inside a primitive for the run loop to recover.
    pub(crate) fn set_error(&self, err: RuleError) {
        *self.error_sink.lock().unwrap_or_else(|e| e.into_inner()) = Some(err);
    }

    /// Take any typed error recorded during this eval.
    pub(crate) fn take_error(&self) -> Option<RuleError> {
        self.error_sink.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// Look up a memoized composed result.
    pub(crate) fn memo_get(&self, key: &MemoKey) -> Option<RuleResult> {
        self.lock_memo().get(key).cloned()
    }

    /// Store a composed result for reuse within this tick.
    pub(crate) fn memo_put(&self, key: MemoKey, result: RuleResult) {
        self.lock_memo().insert(key, result);
    }

    /// Build the `max_operations` a nested eval may use: the remaining budget.
    ///
    /// Returns a [`RuleError::LimitExceeded`] when the shared allowance is spent,
    /// so a deep/wide tree fails closed instead of being granted a fresh budget.
    pub(crate) fn next_op_allowance(&self) -> Result<u64, RuleError> {
        if self.budget.exhausted() {
            return Err(RuleError::LimitExceeded(
                "composition operation budget exhausted".into(),
            ));
        }
        Ok(self.budget.remaining())
    }

    fn lock_memo(&self) -> std::sync::MutexGuard<'_, HashMap<MemoKey, RuleResult>> {
        self.memo.lock().unwrap_or_else(|e| e.into_inner())
    }
}
