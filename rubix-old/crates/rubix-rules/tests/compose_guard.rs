//! Composition tests for `compose/*` and `register/compose_api.rs`.
//!
//! A rule calling another stored rule succeeds; a cycle (direct or transitive)
//! is a clean `resolve` error, not a hang or panic; exceeding the depth cap is a
//! `resolve` error; a missing composed name fails closed.

use std::sync::Arc;

#[path = "support/frame.rs"]
mod frame;

use frame::ts_kw;
use rhai::Map;
use rubix_rules::{
    run_rule, MemoryRuleStore, RuleError, RuleSource, SandboxLimits, StoredRule,
};

fn run_stored(store: MemoryRuleStore, entry: &str) -> Result<rubix_rules::RuleResult, RuleError> {
    run_rule(
        Arc::new(store),
        RuleSource::Stored(entry),
        ts_kw(&[(0, 30.0), (60, 40.0)]),
        Map::new(),
        SandboxLimits::default(),
    )
}

#[test]
fn a_rule_can_call_another_stored_rule() {
    let store = MemoryRuleStore::new()
        .with(StoredRule::new(
            "hot-id",
            "temp-high",
            r#" if df.filter_gt("kw", 25.0).row_count() > 0 {
                    finding("warning", "hot")
                } else { clear() } "#,
        ))
        .with(StoredRule::new(
            "top-id",
            "ahu-health",
            r#" let hi = rule("temp-high", df, #{});
                if hi.flagged { finding("fault", `AHU: ${hi.message}`) } else { clear() } "#,
        ));
    let r = run_stored(store, "ahu-health").unwrap();
    assert!(r.flagged);
    assert_eq!(r.message, "AHU: hot");
}

#[test]
fn direct_cycle_is_a_clean_resolve_error() {
    let store = MemoryRuleStore::new().with(StoredRule::new(
        "self-id",
        "loops",
        r#" rule("loops", df, #{}) "#,
    ));
    let err = run_stored(store, "loops").unwrap_err();
    assert!(matches!(err, RuleError::Resolve(_)), "{err:?}");
    assert!(err.to_string().contains("cycle"), "{err}");
}

#[test]
fn transitive_cycle_is_a_clean_resolve_error() {
    let store = MemoryRuleStore::new()
        .with(StoredRule::new("a", "a", r#" rule("b", df, #{}) "#))
        .with(StoredRule::new("b", "b", r#" rule("a", df, #{}) "#));
    let err = run_stored(store, "a").unwrap_err();
    assert!(matches!(err, RuleError::Resolve(_)), "{err:?}");
}

#[test]
fn missing_composed_name_fails_closed() {
    let store = MemoryRuleStore::new().with(StoredRule::new(
        "x",
        "calls-ghost",
        r#" rule("does-not-exist", df, #{}) "#,
    ));
    let err = run_stored(store, "calls-ghost").unwrap_err();
    assert!(matches!(err, RuleError::Resolve(_)), "{err:?}");
}

#[test]
fn over_depth_trips_the_cap() {
    // A chain longer than DEFAULT_MAX_DEPTH (8): r0 -> r1 -> ... -> r12.
    let mut store = MemoryRuleStore::new();
    for i in 0..12 {
        let script = format!(r#" rule("r{}", df, #{{}}) "#, i + 1);
        store.insert(StoredRule::new(format!("r{i}"), format!("r{i}"), script));
    }
    store.insert(StoredRule::new("r12", "r12", r#" clear() "#));
    let err = run_stored(store, "r0").unwrap_err();
    assert!(matches!(err, RuleError::Resolve(_)), "{err:?}");
    assert!(err.to_string().contains("depth"), "{err}");
}

#[test]
fn missing_required_param_in_composition_errors() {
    use rubix_rules::{ParamSchema, ParamSpec};
    let mut callee = StoredRule::new("c", "needs-limit", r#" clear() "#);
    let mut schema = ParamSchema::empty();
    schema.params.insert(
        "limit".into(),
        ParamSpec { required: true, description: None },
    );
    callee.params = schema;
    let store = MemoryRuleStore::new().with(callee).with(StoredRule::new(
        "caller",
        "caller",
        r#" rule("needs-limit", df, #{}) "#,
    ));
    let err = run_stored(store, "caller").unwrap_err();
    assert!(matches!(err, RuleError::Runtime(_)), "{err:?}");
}
