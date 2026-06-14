//! Kill-switch tests for `sandbox/build.rs` and `sandbox/limits.rs`.
//!
//! These prove the sandbox stops a pathological script: an infinite loop trips
//! the operation cap, an oversized string/array trips the size cap, the
//! wall-clock deadline fires, and `import` is blocked by the dummy resolver
//! (the documented Rhai footgun — absence of registration is not enough).

use std::time::Duration;

#[path = "support/frame.rs"]
mod frame;
#[path = "support/run.rs"]
mod run;

use frame::ts_kw;
use run::run_limited;
use rubix_rules::{RuleError, SandboxLimits};

fn small_ops(max_operations: u64) -> SandboxLimits {
    SandboxLimits {
        max_operations,
        ..SandboxLimits::default()
    }
}

#[test]
fn infinite_loop_trips_max_operations() {
    let limits = small_ops(10_000);
    let err = run_limited("let i = 0; loop { i += 1; }", ts_kw(&[(0, 1.0)]), limits)
        .unwrap_err();
    assert!(matches!(err, RuleError::LimitExceeded(_)), "{err:?}");
}

#[test]
fn oversized_string_trips_size_cap() {
    let limits = SandboxLimits {
        max_string_size: 64,
        ..SandboxLimits::default()
    };
    // Repeated concatenation past the cap is rejected as a limit error.
    let script = r#" let s = "x"; loop { s += s; } "#;
    let err = run_limited(script, ts_kw(&[(0, 1.0)]), limits).unwrap_err();
    assert!(matches!(err, RuleError::LimitExceeded(_)), "{err:?}");
}

#[test]
fn oversized_array_trips_size_cap() {
    let limits = SandboxLimits {
        max_array_size: 8,
        ..SandboxLimits::default()
    };
    let script = r#" let a = []; loop { a.push(1); } "#;
    let err = run_limited(script, ts_kw(&[(0, 1.0)]), limits).unwrap_err();
    assert!(matches!(err, RuleError::LimitExceeded(_)), "{err:?}");
}

#[test]
fn deadline_fires_on_a_slow_script() {
    // A tiny timeout with a large op budget: the wall clock, not the op cap,
    // must be what stops the loop.
    let limits = SandboxLimits {
        max_operations: u64::MAX,
        timeout: Duration::from_millis(50),
        ..SandboxLimits::default()
    };
    let err = run_limited("let i = 0; loop { i += 1; }", ts_kw(&[(0, 1.0)]), limits)
        .unwrap_err();
    assert!(matches!(err, RuleError::LimitExceeded(_)), "{err:?}");
}

#[test]
fn import_is_blocked_by_the_dummy_resolver() {
    // Absence of a registered module is NOT enough to block `import`; the dummy
    // resolver must fail it. A blocked import surfaces as a (non-limit) error,
    // never a successful load.
    let script = r#" import "anything" as m; clear() "#;
    let err = run_limited(script, ts_kw(&[(0, 1.0)]), SandboxLimits::default()).unwrap_err();
    assert!(
        matches!(err, RuleError::Compile(_) | RuleError::Runtime(_)),
        "import must fail closed, got {err:?}"
    );
}
