//! The execution context and `run_rule` entry point.
//!
//! [`run_rule`] is the single way in for both inline scripts and stored-rule
//! ids. It builds one [`Execution`] — the shared sandbox allowance (budget +
//! deadline), the cycle/depth guard, the rule store, and the per-tick memo — and
//! evaluates the script under it. Composed `rule(name, …)` calls reuse the same
//! `Execution`, so the whole tree runs under one budget with one guard.
//!
//! A [`RuleResult`] that is not flagged is the normal "ran and found nothing"
//! outcome and is returned as `Ok`, never an error. Errors are only raised for a
//! broken rule (compile / runtime / limit) or a composition failure (resolve).

mod execution;
mod run_rule;

pub(crate) use execution::Execution;
pub use run_rule::{run_rule, RuleSource};
pub(crate) use run_rule::eval_under;
