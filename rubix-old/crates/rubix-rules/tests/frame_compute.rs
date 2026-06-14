//! Tests for `frame/compute.rs`: the no-row-explosion invariant.
//!
//! Every curated primitive routes through the guarded compute path, and the
//! surface offers no join, so the invariant is structurally enforced. These
//! tests confirm the guard holds across the row-preserving and row-shrinking
//! primitives and that no primitive ever grows a frame.

#[path = "support/frame.rs"]
mod frame;

use frame::ts_kw;

#[test]
fn row_preserving_primitives_never_grow() {
    let f = ts_kw(&[(0, 1.0), (60, 2.0), (120, 3.0)]);
    let n = f.row_count();
    assert_eq!(f.zscore("kw").unwrap().row_count(), n);
    assert_eq!(f.rolling_mean("ts", "kw", "1h").unwrap().row_count(), n);
    assert_eq!(f.lag("ts", "kw").unwrap().row_count(), n);
    assert_eq!(f.anomalies("kw", 2.0).unwrap().row_count(), n);
    assert_eq!(f.sort("kw", true).unwrap().row_count(), n);
}

#[test]
fn shrinking_primitives_only_shrink() {
    let f = ts_kw(&[(0, 1.0), (60, 2.0), (120, 3.0), (180, 4.0)]);
    let n = f.row_count();
    assert!(f.filter_gt("kw", 2.0).unwrap().row_count() <= n);
    assert!(f.head(2).unwrap().row_count() <= n);
    assert!(f
        .resample("ts", "1h", &[("kw".into(), "avg".into())])
        .unwrap()
        .row_count()
        <= n);
}

#[test]
fn chained_primitives_stay_bounded() {
    let f = ts_kw(&[(0, 10.0), (60, 20.0), (120, 30.0), (180, 40.0)]);
    let out = f
        .resample("ts", "120s", &[("kw".into(), "avg".into())])
        .unwrap()
        .zscore("kw")
        .unwrap()
        .anomalies("kw", 1.0)
        .unwrap();
    assert!(out.row_count() <= f.row_count());
}
