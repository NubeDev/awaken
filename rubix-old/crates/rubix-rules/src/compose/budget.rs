//! The shared operation allowance for a whole composition tree.
//!
//! Rhai's `max_operations` is per-`eval`. To make operations *one* budget across
//! every nested `rule()` call, each sub-eval is given a `max_operations` equal to
//! the budget still remaining, and on completion the operations it actually
//! consumed are subtracted. The caller reads consumption via Rhai's engine
//! progress count; we approximate by charging the sub-eval's reported cost. If
//! the remaining budget reaches zero, the next nested call is refused as a
//! limit error rather than silently granting a fresh allowance.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A shared, decrementing operation allowance for one execution tree.
#[derive(Debug, Clone)]
pub struct Budget {
    remaining: Arc<AtomicU64>,
}

impl Budget {
    /// A budget of `total` operations shared across the tree.
    pub fn new(total: u64) -> Self {
        Self {
            remaining: Arc::new(AtomicU64::new(total)),
        }
    }

    /// Operations still available to spend.
    pub fn remaining(&self) -> u64 {
        self.remaining.load(Ordering::Relaxed)
    }

    /// Charge `spent` operations against the budget, saturating at zero.
    pub fn charge(&self, spent: u64) {
        let mut cur = self.remaining.load(Ordering::Relaxed);
        loop {
            let next = cur.saturating_sub(spent);
            match self.remaining.compare_exchange_weak(
                cur,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => cur = actual,
            }
        }
    }

    /// Whether the allowance is exhausted.
    pub fn exhausted(&self) -> bool {
        self.remaining() == 0
    }
}
