//! Golden-frame tests for `frame/resample.rs`.

#[path = "support/frame.rs"]
mod frame;

use frame::{col_f64, ts_kw};

#[test]
fn resample_bins_and_aggregates_and_shrinks() {
    // Six points, 60s apart, into 180s (3-minute) bins -> 2 bins.
    let f = ts_kw(&[
        (0, 1.0),
        (60, 3.0),
        (120, 5.0),
        (180, 10.0),
        (240, 20.0),
        (300, 30.0),
    ]);
    let out = f
        .resample("ts", "180s", &[("kw".into(), "avg".into())])
        .unwrap();
    assert_eq!(out.row_count(), 2);
    let mut kw = col_f64(&out, "kw");
    kw.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(kw, vec![Some(3.0), Some(20.0)]); // bin means
}

#[test]
fn resample_rejects_unknown_aggregate() {
    let f = ts_kw(&[(0, 1.0)]);
    assert!(f
        .resample("ts", "1h", &[("kw".into(), "median".into())])
        .is_err());
}

#[test]
fn resample_requires_aggregates() {
    let f = ts_kw(&[(0, 1.0)]);
    assert!(f.resample("ts", "1h", &[]).is_err());
}
