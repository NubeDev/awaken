//! Composition: running a rule that calls other stored rules, bounded.
//!
//! The `rule(name, frame, params)` primitive loads a stored rule and runs it
//! under the *same* sandbox allowance as its caller. Per the design, the
//! operation budget and wall-clock deadline are **one allowance for the whole
//! composition tree**, not a fresh budget per callee — otherwise composition
//! would multiply the very limits the sandbox sets.
//!
//! Two bounds keep a tree finite:
//!
//! - [`Budget`] — the shared operation/deadline allowance, decremented across
//!   every nested `rule()` call. `set_max_call_levels` backstops Rhai recursion;
//!   the budget backstops total work.
//! - [`Guard`] — an explicit cycle and composition-depth guard enforced at call
//!   time (not at save time): only call-time enforcement catches a dynamically
//!   named `rule(x)` where `x` is computed. A cycle (a rule calling itself
//!   directly or transitively) or exceeding the depth cap is a distinct
//!   [`RuleError::Resolve`], never a hang or panic.
//!
//! [`RuleError::Resolve`]: crate::RuleError::Resolve

mod budget;
mod guard;

pub use budget::Budget;
pub use guard::{Guard, DEFAULT_MAX_DEPTH};
