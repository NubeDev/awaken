//! Golden-frame tests for `frame/select.rs` and `frame/rename.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, ts_kw};

#[test]
fn select_projects_named_columns_in_order() {
    let f = ts_kw(&[(0, 1.0), (60, 2.0)]);
    let out = f.select(&["kw".into()]).unwrap();
    assert_eq!(out.schema().fields().len(), 1);
    assert_eq!(out.schema().field(0).name(), "kw");
    assert_eq!(col_f64(&out, "kw"), vec![Some(1.0), Some(2.0)]);
    assert_eq!(out.row_count(), 2);
}

#[test]
fn select_unknown_column_is_runtime_error() {
    let f = ts_kw(&[(0, 1.0)]);
    let err = f.select(&["nope".into()]).unwrap_err();
    assert!(matches!(err, rubix_rules::RuleError::Runtime(_)), "{err:?}");
}

#[test]
fn select_empty_is_error() {
    assert!(ts_kw(&[(0, 1.0)]).select(&[]).is_err());
}

#[test]
fn rename_changes_one_column_keeps_rest() {
    let f = ts_kw(&[(0, 5.0)]);
    let out = f.rename("kw", "power").unwrap();
    let names: Vec<&str> = out.schema().fields().iter().map(|f| f.name().as_str()).collect();
    assert_eq!(names, vec!["ts", "power"]);
    assert_eq!(col_f64(&out, "power"), vec![Some(5.0)]);
}

#[test]
fn rename_unknown_column_errors() {
    assert!(ts_kw(&[(0, 1.0)]).rename("nope", "x").is_err());
}
