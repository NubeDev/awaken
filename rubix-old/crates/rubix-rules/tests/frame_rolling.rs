//! Golden-frame tests for `frame/rolling.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, ts_kw};

#[test]
fn rolling_mean_is_a_time_duration_window() {
    // Points one minute apart; a 120s trailing window covers this row and the
    // prior two. Mean over [10,20,30] at the last row is 20.
    let f = ts_kw(&[(0, 10.0), (60, 20.0), (120, 30.0)]);
    let out = f.rolling_mean("ts", "kw", "120s").unwrap();
    let roll = col_f64(&out, "kw_roll");
    assert_eq!(roll[0], Some(10.0));
    assert_eq!(roll[1], Some(15.0));
    assert_eq!(roll[2], Some(20.0));
    assert_eq!(out.row_count(), 3); // window emits one value per row — no growth
}

#[test]
fn rolling_max_respects_the_window_bound() {
    // 30s window: each row sees only itself (points are 60s apart).
    let f = ts_kw(&[(0, 10.0), (60, 5.0), (120, 99.0)]);
    let out = f.rolling_max("ts", "kw", "30s").unwrap();
    assert_eq!(col_f64(&out, "kw_roll"), vec![Some(10.0), Some(5.0), Some(99.0)]);
}

#[test]
fn rolling_rejects_bad_duration_and_unknown_column() {
    let f = ts_kw(&[(0, 1.0)]);
    assert!(f.rolling_mean("ts", "kw", "1x").is_err());
    assert!(f.rolling_mean("ts", "nope", "1h").is_err());
}
