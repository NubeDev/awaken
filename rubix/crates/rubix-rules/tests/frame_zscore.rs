//! Golden-frame tests for `frame/zscore.rs` and `frame/anomalies.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, ts_kw};

#[test]
fn zscore_centers_and_scales() {
    // values 10,20,30 -> mean 20, sample stddev 10 -> z = -1, 0, 1
    let f = ts_kw(&[(0, 10.0), (60, 20.0), (120, 30.0)]);
    let out = f.zscore("kw").unwrap();
    let z = col_f64(&out, "kw_z");
    assert_eq!(z, vec![Some(-1.0), Some(0.0), Some(1.0)]);
}

#[test]
fn zscore_zero_spread_is_zero_not_nan() {
    let f = ts_kw(&[(0, 5.0), (60, 5.0)]);
    let out = f.zscore("kw").unwrap();
    assert_eq!(col_f64(&out, "kw_z"), vec![Some(0.0), Some(0.0)]);
}

#[test]
fn anomalies_flags_outliers_beyond_threshold() {
    // 0,0,0,0,10 -> the last point is the clear outlier.
    let f = ts_kw(&[(0, 0.0), (60, 0.0), (120, 0.0), (180, 0.0), (240, 10.0)]);
    let out = f.anomalies("kw", 1.5).unwrap();
    assert!(out.any_true("kw_anomaly").unwrap());
    assert_eq!(out.row_count(), 5); // one flag per row — no growth
}

#[test]
fn anomalies_none_when_flat() {
    let f = ts_kw(&[(0, 5.0), (60, 5.0), (120, 5.0)]);
    let out = f.anomalies("kw", 3.0).unwrap();
    assert!(!out.any_true("kw_anomaly").unwrap());
}

#[test]
fn anomalies_rejects_bad_threshold() {
    let f = ts_kw(&[(0, 1.0)]);
    assert!(f.anomalies("kw", -1.0).is_err());
}
