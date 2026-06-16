//! Build the embedded Rhai engine and register the host surface a rule sees.
//!
//! The rule runtime is embedded and deterministic (`rubix/docs/SCOPE.md`,
//! principle 3: rules fire offline). This builds a Rhai [`Engine`] hardened for
//! that role — bounded operations and recursion so a script cannot run away — and
//! registers the one host function a script needs for composition: `invoke(id)`,
//! which returns the decision value of an already-evaluated sub-rule. Composition
//! is resolved depth-first in Rust *before* the parent script runs (sub-rules pull
//! their own window values asynchronously, which Rhai cannot), so `invoke` is a
//! synchronous lookup into the precomputed child values — keeping the engine free
//! of any I/O and the evaluation deterministic.

use std::collections::HashMap;
use std::sync::Arc;

use rhai::{Engine, EvalAltResult};

use crate::error::RuleError;

/// The host name a script calls to read a sub-rule's decision value.
pub const INVOKE_FN: &str = "invoke";

/// Cap on the operations one script evaluation may run.
///
/// A rule is a small decision, not a compute job (heavy aggregation lives in
/// DataFusion, `rubix/STACK-DEISGN.md`). The cap makes a runaway or adversarial
/// script fail deterministically rather than hang the runtime.
const MAX_OPERATIONS: u64 = 100_000;

/// Cap on script call depth, bounding accidental recursion within one script.
const MAX_CALL_LEVELS: usize = 32;

/// Build an engine whose `invoke(id)` resolves against `child_values`.
///
/// `child_values` maps a sub-rule id to the decision value it already produced.
/// `invoke` returns that value; an id not present is an evaluation error (fail
/// closed — a script cannot invoke a rule the orchestrator did not pre-evaluate),
/// surfaced to the caller as a Rhai runtime error and mapped to a
/// [`RuleError`](crate::error::RuleError) at the run boundary.
#[must_use]
pub fn build_engine(child_values: HashMap<String, f64>) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(MAX_CALL_LEVELS);

    let values = Arc::new(child_values);
    engine.register_fn(
        INVOKE_FN,
        move |id: &str| -> Result<f64, Box<EvalAltResult>> {
            values.get(id).copied().ok_or_else(|| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("invoke: sub-rule '{id}' was not evaluated").into(),
                    rhai::Position::NONE,
                ))
            })
        },
    );

    engine
}

/// Check that `script` compiles under the rule engine, without running it.
///
/// The cheap, side-effect-free validation a write path runs before storing a
/// rule: a script that does not compile is a [`RuleError::Compile`], caught at
/// author time rather than on the next tick. Compilation does not bind inputs or
/// resolve `invoke`, so it validates syntax and shape only — a runtime error
/// (a missing binding, an unevaluated sub-rule) still surfaces at evaluation.
///
/// # Errors
/// Returns [`RuleError::Compile`] if the script is not valid Rhai.
pub fn compile_check(script: &str) -> crate::error::Result<()> {
    build_engine(HashMap::new())
        .compile(script)
        .map(|_| ())
        .map_err(|e| RuleError::Compile(e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::build_engine;

    #[test]
    fn invoke_returns_a_precomputed_child_value() {
        let mut children = HashMap::new();
        children.insert("child".to_owned(), 1.0_f64);
        let engine = build_engine(children);
        let out: f64 = engine.eval(r#"invoke("child")"#).unwrap();
        assert_eq!(out, 1.0);
    }

    #[test]
    fn invoke_on_an_unevaluated_child_errors() {
        let engine = build_engine(HashMap::new());
        let out: Result<f64, _> = engine.eval(r#"invoke("missing")"#);
        assert!(out.is_err());
    }

    #[test]
    fn the_operation_cap_stops_a_runaway_script() {
        let engine = build_engine(HashMap::new());
        let out: Result<i64, _> = engine.eval("let x = 0; loop { x += 1; }");
        assert!(out.is_err(), "an unbounded loop must hit the operation cap");
    }
}
