//! The `run_rule` entry point and the recursive eval driver.

use std::sync::Arc;

use rhai::{Dynamic, Map, Scope};

use super::execution::Execution;
use crate::error::RuleError;
use crate::frame::Frame;
use crate::register::register_all;
use crate::result::RuleResult;
use crate::sandbox::{build_engine, SandboxLimits};
use crate::store::RuleStore;

/// What to run: an inline script, or a stored rule by id/name.
pub enum RuleSource<'a> {
    /// An ad-hoc script (the dry-run / inline-authoring path).
    Inline(&'a str),
    /// A stored rule, resolved by id-or-name through the store.
    Stored(&'a str),
}

/// Evaluate a rule over `frame` with `params`, returning its verdict.
///
/// The single entry point for inline scripts and stored-rule ids. It is *also*
/// the dry-run path: it returns the [`RuleResult`] without emitting a finding —
/// emission is the integrating session's job. A non-flagged result is a normal
/// `Ok`, not an error.
///
/// `params` is the parameter map exposed to the script as the `params` variable.
pub fn run_rule(
    store: Arc<dyn RuleStore>,
    source: RuleSource<'_>,
    frame: Frame,
    params: Map,
    limits: SandboxLimits,
) -> Result<RuleResult, RuleError> {
    use crate::compose::{Guard, DEFAULT_MAX_DEPTH};
    let exec = Arc::new(Execution::new(
        store.clone(),
        limits,
        Guard::new(DEFAULT_MAX_DEPTH),
    ));

    match source {
        RuleSource::Inline(script) => eval_under(&exec, script, frame, params),
        RuleSource::Stored(id) => {
            let rule = store.load(id)?;
            // The top-level stored rule occupies the first guard frame so a
            // self-referential `rule("self")` is caught as a cycle.
            exec.guard.enter(&rule.name)?;
            let out = eval_under(&exec, &rule.script, frame, params);
            exec.guard.leave();
            out
        }
    }
}

/// Compile and evaluate `script` over `frame` under the shared execution.
///
/// Shared by the top-level run and every composed `rule()` call, so the whole
/// tree runs under one budget, one deadline, and one guard. Operations are a
/// single allowance: each eval is capped at the *remaining* budget, and what it
/// consumed is charged back afterward.
pub(crate) fn eval_under(
    exec: &Arc<Execution>,
    script: &str,
    frame: Frame,
    params: Map,
) -> Result<RuleResult, RuleError> {
    let allowance = exec.next_op_allowance()?;

    let mut limits = exec.limits;
    limits.max_operations = allowance;
    let mut engine = build_engine(&limits, exec.deadline);
    register_all(&mut engine, exec.clone());

    let mut scope = Scope::new();
    scope.push("df", frame);
    scope.push("params", params);

    let ast = engine
        .compile(script)
        .map_err(|e| RuleError::Compile(e.to_string()))?;

    let outcome = engine.eval_ast_with_scope::<Dynamic>(&mut scope, &ast);

    // Charge consumed operations against the shared budget. Rhai does not expose
    // a post-run op count, so charge a coarse unit per call: composition depth is
    // already capped, and the deadline bounds wall-clock, so this only needs to
    // ensure the budget strictly decreases per nested eval to fail closed.
    exec.budget.charge(1);

    match outcome {
        Ok(value) => coerce_result(value),
        Err(err) => Err(classify(exec, *err)),
    }
}

/// Turn a script's final value into a [`RuleResult`].
///
/// A rule that ends in `finding(...)`/`clear()` yields a `RuleResult`. A rule
/// whose last expression is `()` (e.g. an `if` with no `else`) is treated as a
/// non-flagged result — "ran, found nothing", not an error.
fn coerce_result(value: Dynamic) -> Result<RuleResult, RuleError> {
    if value.is_unit() {
        return Ok(RuleResult::clear());
    }
    value
        .try_cast::<RuleResult>()
        .ok_or_else(|| {
            RuleError::Runtime(
                "rule must return a finding(...) / clear() result (or nothing)".into(),
            )
        })
}

/// Map a Rhai eval error to a categorized [`RuleError`].
///
/// A typed error recorded by a primitive on the execution sink takes precedence
/// (so a composition `resolve` error keeps its category). Otherwise the Rhai
/// error kind decides: an operation/time/size limit becomes `LimitExceeded`,
/// everything else `Runtime`. No path panics across the Rhai edge.
fn classify(exec: &Execution, err: rhai::EvalAltResult) -> RuleError {
    if let Some(typed) = exec.take_error() {
        return typed;
    }
    use rhai::EvalAltResult::*;
    match err {
        ErrorTooManyOperations(_)
        | ErrorTerminated(_, _)
        | ErrorDataTooLarge(_, _)
        | ErrorStackOverflow(_) => RuleError::LimitExceeded(err.to_string()),
        other => RuleError::Runtime(other.to_string()),
    }
}
