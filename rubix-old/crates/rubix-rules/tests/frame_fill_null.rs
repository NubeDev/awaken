//! Golden-frame tests for `frame/fill_null.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, kw};

#[test]
fn fill_null_zero_replaces_nulls() {
    let f = kw(&[Some(1.0), None, Some(3.0)]);
    let out = f.fill_null("zero").unwrap();
    assert_eq!(col_f64(&out, "kw"), vec![Some(1.0), Some(0.0), Some(3.0)]);
    assert_eq!(out.row_count(), 3);
}

#[test]
fn fill_null_mean_uses_column_mean() {
    // present values 2 and 4 -> mean 3 fills the gap
    let f = kw(&[Some(2.0), None, Some(4.0)]);
    let out = f.fill_null("mean").unwrap();
    assert_eq!(col_f64(&out, "kw"), vec![Some(2.0), Some(3.0), Some(4.0)]);
}

#[test]
fn fill_null_unknown_strategy_errors() {
    assert!(kw(&[Some(1.0)]).fill_null("forward").is_err());
}
