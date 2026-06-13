//! Golden-frame tests for `frame/head.rs` and `frame/sort.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, ts_kw};

#[test]
fn head_keeps_first_n() {
    let f = ts_kw(&[(0, 1.0), (60, 2.0), (120, 3.0)]);
    let out = f.head(2).unwrap();
    assert_eq!(col_f64(&out, "kw"), vec![Some(1.0), Some(2.0)]);
}

#[test]
fn tail_keeps_last_n_in_order() {
    let f = ts_kw(&[(0, 1.0), (60, 2.0), (120, 3.0)]);
    let out = f.tail(2).unwrap();
    assert_eq!(col_f64(&out, "kw"), vec![Some(2.0), Some(3.0)]);
}

#[test]
fn head_more_than_rows_is_all_rows_no_growth() {
    let f = ts_kw(&[(0, 1.0)]);
    assert_eq!(f.head(99).unwrap().row_count(), 1);
}

#[test]
fn sort_orders_by_column() {
    let f = ts_kw(&[(0, 3.0), (60, 1.0), (120, 2.0)]);
    let asc = f.sort("kw", true).unwrap();
    assert_eq!(col_f64(&asc, "kw"), vec![Some(1.0), Some(2.0), Some(3.0)]);
    let desc = f.sort("kw", false).unwrap();
    assert_eq!(col_f64(&desc, "kw"), vec![Some(3.0), Some(2.0), Some(1.0)]);
}
