//! Register the `rule(name, frame, params)` composition primitive into Rhai.
//!
//! A rule composes another stored rule by name, over a caller-supplied frame
//! (callees never query). The call is bounded by the shared budget and the
//! cycle/depth guard, and memoized within the tick. The returned value is the
//! callee's [`RuleResult`], which the caller reads via the result accessors.

use std::sync::Arc;

use rhai::{Engine, EvalAltResult, Map};

use super::bridge;
use crate::error::RuleError;
use crate::frame::Frame;
use crate::result::RuleResult;
use crate::run::{eval_under, Execution};

/// Register the `rule(name, frame, params)` primitive for `exec`.
pub(crate) fn register_compose(engine: &mut Engine, exec: Arc<Execution>) {
    let e = exec;
    engine.register_fn(
        "rule",
        move |name: &str, frame: Frame, params: Map| -> Result<RuleResult, Box<EvalAltResult>> {
            bridge(&e, compose(&e, name, frame, params))
        },
    );
}

/// Resolve, bound, run, and memoize a composed rule.
fn compose(
    exec: &Arc<Execution>,
    name: &str,
    frame: Frame,
    params: Map,
) -> Result<RuleResult, RuleError> {
    // Memo within the tick keyed by (name, frame identity, canonical params), so
    // a shared rule invoked by several callers runs once.
    let key = (name.to_string(), frame.identity(), canonical_params(&params));
    if let Some(hit) = exec.memo_get(&key) {
        return Ok(hit);
    }

    // Cycle / over-depth guard, enforced at call time (catches a computed name).
    exec.guard.enter(name)?;
    let loaded = exec.store.load(name);
    let result = loaded.and_then(|stored| {
        check_required_params(&stored.params, &params)?;
        eval_under(exec, &stored.script, frame, params)
    });
    exec.guard.leave();

    let result = result?;
    exec.memo_put(key, result.clone());
    Ok(result)
}

/// Reject a composition that omits a declared-required parameter.
///
/// Makes a mismatch fail clearly (a `resolve`-adjacent runtime error) rather
/// than opaquely deep inside the callee script.
fn check_required_params(
    schema: &crate::store::ParamSchema,
    params: &Map,
) -> Result<(), RuleError> {
    for required in schema.required_names() {
        if !params.contains_key(required) {
            return Err(RuleError::Runtime(format!(
                "composed rule missing required param `{required}`"
            )));
        }
    }
    Ok(())
}

/// Canonicalize a params map to a stable string for memo keying.
///
/// Rhai `Map` is a `BTreeMap`, so iteration is already key-ordered; rendering
/// each value via its `Debug` form is enough for equal maps to collide and
/// differing maps to separate within a tick.
fn canonical_params(params: &Map) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{k}={v:?}"))
        .collect::<Vec<_>>()
        .join(";")
}
