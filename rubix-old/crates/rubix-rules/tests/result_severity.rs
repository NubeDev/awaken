//! Tests for `severity.rs` and `result.rs`.

use rubix_rules::{RuleResult, Severity};

#[test]
fn severity_round_trips_its_wire_string() {
    for s in [Severity::Info, Severity::Warning, Severity::Fault] {
        assert_eq!(Severity::parse(s.as_str()).unwrap(), s);
    }
}

#[test]
fn unknown_severity_is_an_error_not_a_downgrade() {
    assert!(Severity::parse("CRITICAL").is_err());
    assert!(Severity::parse("").is_err());
}

#[test]
fn finding_is_flagged_clear_is_not() {
    let f = RuleResult::finding(Severity::Fault, "boom");
    assert!(f.flagged);
    assert_eq!(f.severity, Severity::Fault);
    assert_eq!(f.message, "boom");
    assert!(f.value.is_none());

    let c = RuleResult::clear();
    assert!(!c.flagged);
}

#[test]
fn with_value_attaches_a_score() {
    let r = RuleResult::finding(Severity::Info, "x").with_value(2.5);
    assert_eq!(r.value, Some(2.5));
}

#[test]
fn result_round_trips_through_json() {
    let r = RuleResult::finding(Severity::Warning, "msg").with_value(1.0);
    let back: RuleResult = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(r, back);
}
