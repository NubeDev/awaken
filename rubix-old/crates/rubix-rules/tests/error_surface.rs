//! Error-surface tests: the four categories are distinct and nothing panics.
//!
//! A spark must tell "the rule is broken" (compile / runtime / limit) from a
//! composition failure (resolve), and from "ran, found nothing" (a non-error).
//! Every failure crosses the Rhai edge as a typed `RuleError`, never a panic.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/run.rs"]
mod run;

use frame::ts_kw;
use run::run_with;
use rubix_rules::{MemoryRuleStore, RuleError};

#[test]
fn malformed_script_is_a_compile_error() {
    let err = run_with(MemoryRuleStore::new(), "let x = ;", ts_kw(&[(0, 1.0)])).unwrap_err();
    assert!(matches!(err, RuleError::Compile(_)), "{err:?}");
}

#[test]
fn bad_primitive_argument_is_a_runtime_error() {
    // Unknown column reaches the primitive as a runtime error, not a panic.
    let err = run_with(
        MemoryRuleStore::new(),
        r#" df.filter_gt("ghost", 1.0); clear() "#,
        ts_kw(&[(0, 1.0)]),
    )
    .unwrap_err();
    assert!(matches!(err, RuleError::Runtime(_)), "{err:?}");
}

#[test]
fn thrown_error_is_a_runtime_error() {
    let err = run_with(MemoryRuleStore::new(), r#" throw "boom" "#, ts_kw(&[(0, 1.0)])).unwrap_err();
    assert!(matches!(err, RuleError::Runtime(_)), "{err:?}");
}

#[test]
fn returning_a_non_result_value_is_a_runtime_error() {
    // A rule whose final value is neither a RuleResult nor () is rejected.
    let err = run_with(MemoryRuleStore::new(), "42", ts_kw(&[(0, 1.0)])).unwrap_err();
    assert!(matches!(err, RuleError::Runtime(_)), "{err:?}");
}

#[test]
fn unknown_severity_string_is_a_runtime_error() {
    let err = run_with(
        MemoryRuleStore::new(),
        r#" finding("critical", "x") "#,
        ts_kw(&[(0, 1.0)]),
    )
    .unwrap_err();
    assert!(matches!(err, RuleError::Runtime(_)), "{err:?}");
}

#[test]
fn error_categories_render_to_distinct_prefixes() {
    // The surfaced strings are stable enough for a spark to log/route by.
    assert!(RuleError::Compile("x".into()).to_string().starts_with("compile rule"));
    assert!(RuleError::Runtime("x".into()).to_string().starts_with("run rule"));
    assert!(RuleError::LimitExceeded("x".into())
        .to_string()
        .starts_with("rule exceeded"));
    assert!(RuleError::Resolve("x".into())
        .to_string()
        .starts_with("resolve composed"));
}
