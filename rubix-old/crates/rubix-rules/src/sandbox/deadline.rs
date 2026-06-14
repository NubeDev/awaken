//! A shared wall-clock deadline for one execution.
//!
//! `set_max_operations` bounds work but not time: a script can sit under the op
//! limit and still run long (e.g. an expensive primitive call per iteration), so
//! the design requires a wall-clock cap via `on_progress` + a deadline. The
//! deadline is cloned into the progress callback and checked on every operation
//! tick; the same `Deadline` is shared across a whole composition tree so the
//! time budget is one allowance, not one-per-callee.

use std::time::Instant;

/// A wall-clock cut-off shared by an execution (and its composed callees).
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    at: Instant,
}

impl Deadline {
    /// A deadline `timeout` from now.
    pub fn after(timeout: std::time::Duration) -> Self {
        Self {
            at: Instant::now() + timeout,
        }
    }

    /// Whether the wall-clock budget is spent.
    pub fn expired(&self) -> bool {
        Instant::now() >= self.at
    }
}
