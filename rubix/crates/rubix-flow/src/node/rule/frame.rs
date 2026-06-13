//! Build a rule [`Frame`] from a `query_his` payload (a JSON array of history
//! samples) and enforce the input cap.
//!
//! The data path is `query → rule → emit_spark`: the query node returns rows,
//! the rule node folds them into a decision. A rule that folds **truncated**
//! input into a finding can silently reach a wrong conclusion, so a caps breach
//! on the input is an error that fails the node — never a finding emitted from
//! partial data (design: "Truncation and caps on the spark path"). The cap is
//! the node's `max_rows` config; an input larger than it means the upstream
//! query did not bound its read and the frame cannot be trusted.

use std::sync::Arc;

use rubix_core::HisSample;
use rubix_rules::arrow::array::{Float64Array, TimestampNanosecondArray};
use rubix_rules::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use rubix_rules::{Frame, RecordBatch, SchemaRef};

/// Column names the rule frame exposes to scripts (`df.rolling_mean("ts", …)`).
pub(super) const TS_COL: &str = "ts";
pub(super) const VALUE_COL: &str = "value";

/// Why a frame could not be built from a node input.
#[derive(Debug)]
pub(super) enum FrameError {
    /// The input row count exceeded the node's `max_rows` cap — a truncation
    /// breach the rule must not fold into a finding.
    CapsBreach { rows: usize, max_rows: usize },
    /// The payload was not a decodable history-sample array.
    Decode(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::CapsBreach { rows, max_rows } => write!(
                f,
                "input cap exceeded: {rows} rows > max_rows {max_rows} \
                 (truncated input must not be folded into a finding)"
            ),
            FrameError::Decode(e) => write!(f, "decode history samples: {e}"),
        }
    }
}

/// Decode `payload` as a `query_his` sample array and build a two-column frame
/// (`ts` timestamp, `value` float64), rejecting an input over `max_rows`.
///
/// A non-numeric sample value (a string point) becomes a null in `value`; the
/// timestamp is always present. The curated primitives operate on this frame.
pub(super) fn frame_from_samples(
    payload: &serde_json::Value,
    max_rows: usize,
) -> Result<Frame, FrameError> {
    let samples: Vec<HisSample> =
        serde_json::from_value(payload.clone()).map_err(|e| FrameError::Decode(e.to_string()))?;

    if samples.len() > max_rows {
        return Err(FrameError::CapsBreach {
            rows: samples.len(),
            max_rows,
        });
    }

    let schema: SchemaRef = Arc::new(Schema::new(vec![
        Field::new(
            TS_COL,
            DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into())),
            false,
        ),
        Field::new(VALUE_COL, DataType::Float64, true),
    ]));

    let ts: TimestampNanosecondArray = samples
        .iter()
        .map(|s| s.ts.timestamp_nanos_opt())
        .collect::<Option<Vec<_>>>()
        .map(|v| TimestampNanosecondArray::from(v).with_timezone("UTC"))
        .ok_or_else(|| FrameError::Decode("sample timestamp out of representable range".into()))?;

    let value = Float64Array::from(
        samples
            .iter()
            .map(|s| s.value.as_f64())
            .collect::<Vec<Option<f64>>>(),
    );

    let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(ts), Arc::new(value)])
        .map_err(|e| FrameError::Decode(e.to_string()))?;

    Ok(Frame::new(schema, vec![batch]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rubix_core::PointValue;

    fn sample(secs: i64, v: PointValue) -> HisSample {
        HisSample {
            ts: Utc.timestamp_opt(secs, 0).unwrap(),
            value: v,
        }
    }

    #[test]
    fn builds_frame_from_numeric_samples() {
        let payload = serde_json::to_value(vec![
            sample(0, PointValue::Number(20.0)),
            sample(60, PointValue::Number(25.0)),
        ])
        .unwrap();
        let frame = frame_from_samples(&payload, 100).unwrap();
        assert_eq!(frame.row_count(), 2);
        assert_eq!(frame.schema().fields().len(), 2);
    }

    #[test]
    fn string_value_becomes_null_not_an_error() {
        let payload = serde_json::to_value(vec![sample(0, PointValue::Str("on".into()))]).unwrap();
        let frame = frame_from_samples(&payload, 100).unwrap();
        assert_eq!(frame.row_count(), 1);
    }

    #[test]
    fn over_cap_input_is_a_breach() {
        let payload = serde_json::to_value(vec![
            sample(0, PointValue::Number(1.0)),
            sample(1, PointValue::Number(2.0)),
        ])
        .unwrap();
        let err = frame_from_samples(&payload, 1).unwrap_err();
        assert!(matches!(err, FrameError::CapsBreach { rows: 2, max_rows: 1 }));
    }

    #[test]
    fn non_array_payload_is_a_decode_error() {
        let payload = serde_json::json!({"not": "an array"});
        let err = frame_from_samples(&payload, 100).unwrap_err();
        assert!(matches!(err, FrameError::Decode(_)));
    }
}
