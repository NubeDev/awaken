//! Golden-frame tests for `frame/filter.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, ts_kw};

#[test]
fn filter_gt_keeps_strictly_greater() {
    let f = ts_kw(&[(0, 1.0), (60, 25.0), (120, 30.0)]);
    let out = f.filter_gt("kw", 25.0).unwrap();
    assert_eq!(col_f64(&out, "kw"), vec![Some(30.0)]);
}

#[test]
fn filter_lt_keeps_strictly_less() {
    let f = ts_kw(&[(0, 1.0), (60, 25.0)]);
    let out = f.filter_lt("kw", 25.0).unwrap();
    assert_eq!(col_f64(&out, "kw"), vec![Some(1.0)]);
}

#[test]
fn filter_eq_keeps_equal() {
    let f = ts_kw(&[(0, 1.0), (60, 25.0)]);
    let out = f.filter_eq("kw", 25.0).unwrap();
    assert_eq!(col_f64(&out, "kw"), vec![Some(25.0)]);
}

#[test]
fn filter_never_grows_and_can_empty() {
    let f = ts_kw(&[(0, 1.0)]);
    let out = f.filter_gt("kw", 100.0).unwrap();
    assert_eq!(out.row_count(), 0);
}

#[test]
fn filter_rejects_non_finite_and_quotes_safely() {
    let f = ts_kw(&[(0, 1.0)]);
    assert!(f.filter_gt("kw", f64::NAN).is_err());
    // An injection-flavored column name resolves to "no such column", not SQL.
    assert!(f.filter_gt("kw\"; DROP TABLE f; --", 1.0).is_err());
}
