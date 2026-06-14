//! Register the rule's return type and the `finding` constructor into Rhai.
//!
//! `finding(severity, message)` builds a flagged [`RuleResult`]; `clear()` is the
//! non-flagged "ran, found nothing" result. A script's last expression is its
//! return value, so a rule ends in `finding(...)` or `clear()` (or any
//! expression evaluating to a `RuleResult`). Accessors let a composing rule read
//! a callee's verdict: `r.flagged`, `r.severity`, `r.message`, `r.value`.

use rhai::{Dynamic, Engine, EvalAltResult};

use crate::result::RuleResult;
use crate::severity::Severity;

/// Register `RuleResult`, `finding`, `clear`, and the result accessors.
pub(crate) fn register_result(engine: &mut Engine) {
    engine.register_type_with_name::<RuleResult>("RuleResult");

    engine.register_fn(
        "finding",
        |severity: &str, message: &str| -> Result<RuleResult, Box<EvalAltResult>> {
            let sev = Severity::parse(severity).map_err(|e| e.to_string())?;
            Ok(RuleResult::finding(sev, message))
        },
    );
    engine.register_fn("clear", RuleResult::clear);

    // Attach a score to a result a composing rule can read.
    engine.register_fn("with_value", |r: RuleResult, value: f64| r.with_value(value));

    // Read-only accessors (also exposed as Rhai getters `r.flagged`, etc.).
    engine.register_get("flagged", |r: &mut RuleResult| r.flagged);
    engine.register_get("severity", |r: &mut RuleResult| r.severity.as_str().to_string());
    engine.register_get("message", |r: &mut RuleResult| r.message.clone());
    engine.register_get("value", |r: &mut RuleResult| {
        r.value.map(Dynamic::from_float).unwrap_or(Dynamic::UNIT)
    });
}
