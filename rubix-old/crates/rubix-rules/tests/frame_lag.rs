//! Golden-frame tests for `frame/lag.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, ts_kw};

#[test]
fn lag_shifts_prior_value() {
    let f = ts_kw(&[(0, 10.0), (60, 20.0), (120, 30.0)]);
    let out = f.lag("ts", "kw").unwrap();
    assert_eq!(col_f64(&out, "kw_lag"), vec![None, Some(10.0), Some(20.0)]);
}

#[test]
fn diff_is_value_minus_prior() {
    let f = ts_kw(&[(0, 10.0), (60, 25.0)]);
    let out = f.diff("ts", "kw").unwrap();
    assert_eq!(col_f64(&out, "kw_diff"), vec![None, Some(15.0)]);
}

#[test]
fn pct_change_is_fractional_and_guards_zero() {
    let f = ts_kw(&[(0, 0.0), (60, 10.0), (120, 15.0)]);
    let out = f.pct_change("ts", "kw").unwrap();
    let pct = col_f64(&out, "kw_pct");
    assert_eq!(pct[0], None); // no prior
    assert_eq!(pct[1], None); // prior is 0 -> guarded to NULL
    assert_eq!(pct[2], Some(0.5));
    assert_eq!(out.row_count(), 3);
}
