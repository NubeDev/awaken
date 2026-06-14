//! Tests for `compose/budget.rs` and per-tick memoization.

use std::sync::Arc;

#[path = "support/frame.rs"]
mod frame;

use frame::ts_kw;
use rhai::Map;
use rubix_rules::{run_rule, MemoryRuleStore, RuleSource, SandboxLimits, StoredRule};

#[test]
fn diamond_composition_resolves_shared_callee() {
    // D calls B and C; both call the shared rule S over the same `df`. With
    // per-tick memoization keyed by (name, frame, params), S is evaluated once
    // and both branches see the same verdict.
    let store = MemoryRuleStore::new()
        .with(StoredRule::new(
            "s",
            "shared",
            r#" if df.filter_gt("kw", 25.0).row_count() > 0 {
                    finding("warning", "hot")
                } else { clear() } "#,
        ))
        .with(StoredRule::new("b", "b", r#" rule("shared", df, #{}) "#))
        .with(StoredRule::new("c", "c", r#" rule("shared", df, #{}) "#))
        .with(StoredRule::new(
            "d",
            "d",
            r#" let x = rule("b", df, #{});
                let y = rule("c", df, #{});
                if x.flagged && y.flagged { finding("fault", "both") } else { clear() } "#,
        ));
    let r = run_rule(
        Arc::new(store),
        RuleSource::Stored("d"),
        ts_kw(&[(0, 30.0), (60, 40.0)]),
        Map::new(),
        SandboxLimits::default(),
    )
    .unwrap();
    assert!(r.flagged);
    assert_eq!(r.message, "both");
}

#[test]
fn exhausted_budget_fails_closed() {
    // A minuscule op budget cannot complete even a trivial rule -> a limit
    // error, never a fresh allowance handed to the script.
    let limits = SandboxLimits {
        max_operations: 1,
        ..SandboxLimits::default()
    };
    let out = run_rule(
        Arc::new(MemoryRuleStore::new()),
        RuleSource::Inline(r#" let i = 0; while i < 100 { i += 1; } clear() "#),
        ts_kw(&[(0, 1.0)]),
        Map::new(),
        limits,
    );
    assert!(matches!(out, Err(rubix_rules::RuleError::LimitExceeded(_))), "{out:?}");
}
