//! Golden-frame tests for `frame/describe.rs` and `frame/any_true.rs`.

#[path = "support/frame.rs"]
mod frame;

use datafusion::arrow::array::{Array, BooleanArray, Float64Array};
use frame::{kw, ts_kw};

#[test]
fn describe_summarizes_each_numeric_column() {
    let f = ts_kw(&[(0, 10.0), (60, 20.0), (120, 30.0)]);
    let out = f.describe().unwrap();
    // one row per numeric column: `kw` only (ts is a timestamp, non-numeric).
    assert_eq!(out.row_count(), 1);
    let batch = &out.batches()[0];
    let mean_idx = out.schema().index_of("mean").unwrap();
    let mean = batch
        .column(mean_idx)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    assert_eq!(mean.value(0), 20.0);
}

#[test]
fn describe_no_numeric_columns_errors() {
    // timestamp-only frame after dropping kw.
    let f = ts_kw(&[(0, 1.0)]).select(&["ts".into()]).unwrap();
    assert!(f.describe().is_err());
}

#[test]
fn any_true_reduces_a_boolean_flag_column() {
    let f = kw(&[Some(0.0), Some(0.0), Some(50.0)]);
    let flagged = f.anomalies("kw", 1.0).unwrap();
    assert!(flagged.any_true("kw_anomaly").unwrap());
    let _ = BooleanArray::from(vec![true]); // keep import meaningful
}

#[test]
fn any_true_false_on_empty_frame() {
    let f = kw(&[Some(1.0)]).filter_gt("kw", 100.0).unwrap();
    let flagged = f.anomalies("kw", 1.0).unwrap();
    assert!(!flagged.any_true("kw_anomaly").unwrap());
}
