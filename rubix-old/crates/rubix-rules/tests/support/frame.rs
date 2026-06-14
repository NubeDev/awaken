//! Golden-frame fixtures shared across the primitive tests.
//!
//! Included via `#[path]` rather than a crate-level module so each integration
//! test file can build the same canonical frames without a `common.rs`-style
//! catch-all. One concept: constructing test frames.

use std::sync::Arc;

use rubix_rules::{Frame, RecordBatch};

use datafusion::arrow::array::{Array, Float64Array, TimestampSecondArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};

/// A frame with a `ts` (timestamp, seconds) column and a numeric `kw` column.
///
/// Rows are at the given `(epoch_secs, kw)` points, in the order supplied.
#[allow(dead_code)]
pub fn ts_kw(points: &[(i64, f64)]) -> Frame {
    let schema = Arc::new(Schema::new(vec![
        Field::new(
            "ts",
            DataType::Timestamp(TimeUnit::Second, None),
            false,
        ),
        Field::new("kw", DataType::Float64, true),
    ]));
    let ts: Vec<i64> = points.iter().map(|(t, _)| *t).collect();
    let kw: Vec<f64> = points.iter().map(|(_, v)| *v).collect();
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(TimestampSecondArray::from(ts)),
            Arc::new(Float64Array::from(kw)),
        ],
    )
    .expect("build batch");
    Frame::new(schema, vec![batch])
}

/// A single-column numeric frame named `kw`, one batch.
#[allow(dead_code)]
pub fn kw(values: &[Option<f64>]) -> Frame {
    let schema = Arc::new(Schema::new(vec![Field::new("kw", DataType::Float64, true)]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(Float64Array::from(values.to_vec()))],
    )
    .expect("build batch");
    Frame::new(schema, vec![batch])
}

/// Read column `col` across all batches of a frame as `Vec<Option<f64>>`.
#[allow(dead_code)]
pub fn col_f64(frame: &Frame, col: &str) -> Vec<Option<f64>> {
    let idx = frame
        .schema()
        .index_of(col)
        .unwrap_or_else(|_| panic!("no column {col}"));
    let mut out = Vec::new();
    for batch in frame.batches() {
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<Float64Array>()
            .expect("f64 column");
        for i in 0..arr.len() {
            out.push(if arr.is_null(i) { None } else { Some(arr.value(i)) });
        }
    }
    out
}
