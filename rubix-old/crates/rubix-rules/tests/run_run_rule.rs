//! End-to-end tests for `run/run_rule.rs`: the entry point and result coercion.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/run.rs"]
mod run;

use frame::ts_kw;
use run::run_with;
use rubix_rules::{MemoryRuleStore, Severity};

#[test]
fn script_orchestrates_engine_computes_into_a_finding() {
    // The canonical shape: chain primitives, then decide. No row iteration.
    let script = r#"
        let hot = df.resample("ts", "120s", #{ "kw": "avg" }).filter_gt("kw", 25.0);
        if hot.row_count() > 0 {
            finding("fault", "avg kw exceeded 25")
        } else {
            clear()
        }
    "#;
    let f = ts_kw(&[(0, 30.0), (60, 40.0), (120, 10.0), (180, 5.0)]);
    let r = run_with(MemoryRuleStore::new(), script, f).unwrap();
    assert!(r.flagged);
    assert_eq!(r.severity, Severity::Fault);
    assert_eq!(r.message, "avg kw exceeded 25");
}

#[test]
fn empty_result_is_ok_not_error() {
    let script = r#" if df.row_count() > 100 { finding("warning", "x") } else { clear() } "#;
    let r = run_with(MemoryRuleStore::new(), script, ts_kw(&[(0, 1.0)])).unwrap();
    assert!(!r.flagged); // ran and found nothing — a normal Ok
}

#[test]
fn rule_returning_unit_is_a_clear_result() {
    // An `if` with no else evaluates to () — treated as a non-flagged result.
    let r = run_with(MemoryRuleStore::new(), "if false { finding(\"info\", \"x\") }", ts_kw(&[(0, 1.0)]))
        .unwrap();
    assert!(!r.flagged);
}

#[test]
fn anomalies_to_decision_via_any_true() {
    let script = r#"
        let flagged = df.anomalies("kw", 1.5);
        if any_true(flagged, "kw_anomaly") {
            finding("warning", "anomaly detected")
        } else {
            clear()
        }
    "#;
    let f = ts_kw(&[(0, 0.0), (60, 0.0), (120, 0.0), (180, 50.0)]);
    let r = run_with(MemoryRuleStore::new(), script, f).unwrap();
    assert!(r.flagged);
    assert_eq!(r.severity, Severity::Warning);
}

#[test]
fn with_value_carries_a_score() {
    let r = run_with(
        MemoryRuleStore::new(),
        r#" finding("info", "scored").with_value(3.5) "#,
        ts_kw(&[(0, 1.0)]),
    )
    .unwrap();
    assert_eq!(r.value, Some(3.5));
}
