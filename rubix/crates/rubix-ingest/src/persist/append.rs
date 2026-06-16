//! Append a pre-processed sample as a reading into the edge partition.
//!
//! Persistence is the last stage of ingest: a sample that survived
//! pre-processing is written into the partition keyed by the edge identity
//! (`rubix/STACK-DEISGN.md`, contract #5: append-only, edge-partitioned — two
//! edges never write the same rows, so reconciliation is ordering + dedup, not
//! merge). Samples are time-series, so they land in the **`reading` data plane**,
//! not the generic `record` table (`rubix/docs/design/READINGS-TIMESERIES.md`):
//! the decoded [`Sample`] maps to a lean `{ series, at, value }` reading in the
//! principal's namespace, extras preserved in `content`.
//!
//! The write does **not** re-cross the command gate per sample: the capability
//! decision was taken once at subscribe (`authorize`), so taxing every high-rate
//! message again would defeat the streaming design (contract #2). The edge
//! partition comes from the principal, never from the sample, so a publisher
//! cannot write into another edge's partition by spoofing a field. The row id is
//! derived from `(series, at)`, so a duplicated delivery is an idempotent no-op.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::Datetime;

use rubix_core::{Principal, Reading, append_readings};

use crate::error::{IngestError, Result};
use crate::subscribe::Sample;

use super::partition::partition_for;

/// Append `sample` as a reading in `principal`'s edge partition.
///
/// Maps the decoded sample to a [`Reading`] (see [`reading_from_sample`]) in the
/// partition namespace and appends it. Returns the persisted reading; the
/// deterministic `(series, at)` id makes a re-delivery of the same sample a no-op
/// rather than a duplicate row.
///
/// # Errors
/// Returns [`IngestError::Persist`] if the sample carries no numeric value or the
/// append write fails.
pub async fn append_sample(
    db: &Surreal<Db>,
    principal: &Principal,
    sample: &Sample,
) -> Result<Reading> {
    let partition = partition_for(principal);
    let reading = reading_from_sample(partition, sample)?;
    append_readings(db, std::slice::from_ref(&reading))
        .await
        .map_err(|e| IngestError::Persist(e.to_string()))?;
    Ok(reading)
}

/// Map a decoded [`Sample`] into the [`Reading`] it persists as.
///
/// `series` is the series-defining record id, taken from an explicit
/// `content.series` when the source supplies one, else resolved from the
/// key-space — the last segment of the Zenoh key (`rubix/ingest/<ns>/<series>`).
/// `at` is **measurement** time: the sample's own `content.at`/`content.ts`
/// timestamp when the source stamps it, else arrival time (now) — never the write
/// time. `value` is the numeric `content.value`; a sample without one is not a
/// well-formed reading and is rejected. The promoted keys are stripped from the
/// retained `content` so the row stays lean while real extras (quality flags,
/// enrichment) are preserved (`READINGS-TIMESERIES.md`, "Rows stay lean").
///
/// # Errors
/// Returns [`IngestError::Persist`] if the sample carries no numeric `value`.
fn reading_from_sample(partition: &str, sample: &Sample) -> Result<Reading> {
    let value = sample
        .content
        .get("value")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| {
            IngestError::Persist("ingested sample carries no numeric `value`".to_owned())
        })?;
    let series = sample
        .content
        .get("series")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| series_from_key(&sample.key));
    let at = sample_instant(&sample.content).unwrap_or_else(Datetime::now);
    let content = extras(&sample.content);
    Ok(Reading::new(partition, series, at, value, content))
}

/// The series id encoded in a Zenoh key — its last path segment.
fn series_from_key(key: &str) -> String {
    key.rsplit('/').next().unwrap_or(key).to_owned()
}

/// The sample's own measurement instant from `content.at`/`content.ts`, if it
/// carries a valid RFC3339 one; `None` falls the caller back to arrival time.
fn sample_instant(content: &serde_json::Value) -> Option<Datetime> {
    content
        .get("at")
        .or_else(|| content.get("ts"))
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.parse::<Datetime>().ok())
}

/// The content to retain on the reading: the sample's object minus the keys
/// promoted to typed columns, so the row does not duplicate them. A non-object
/// content yields an empty object.
fn extras(content: &serde_json::Value) -> serde_json::Value {
    let mut object = content.as_object().cloned().unwrap_or_default();
    for promoted in ["series", "value", "at", "ts"] {
        object.remove(promoted);
    }
    serde_json::Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::reading_from_sample;
    use crate::subscribe::Sample;

    #[test]
    fn maps_value_series_and_at_into_typed_columns() {
        let sample = Sample::new(
            "rubix/ingest/edge-7/reg-1",
            serde_json::json!({ "value": 21.5, "at": "2026-06-14T10:00:00Z", "unit": "c" }),
        );
        let reading = reading_from_sample("edge-7", &sample).expect("maps to a reading");
        assert_eq!(reading.namespace, "edge-7");
        assert_eq!(reading.series, "reg-1", "series from the key's last segment");
        assert!((reading.value - 21.5).abs() < f64::EPSILON);
        // `at` is the sample's stamped instant, not arrival time.
        assert_eq!(reading.at, "2026-06-14T10:00:00Z".parse().unwrap());
        // The promoted keys are stripped; real extras stay in content.
        assert_eq!(reading.content.get("unit"), Some(&serde_json::json!("c")));
        assert!(reading.content.get("value").is_none());
        assert!(reading.content.get("at").is_none());
    }

    #[test]
    fn an_explicit_content_series_overrides_the_key() {
        let sample = Sample::new(
            "rubix/ingest/edge-7/temp",
            serde_json::json!({ "value": 1.0, "series": "reg-explicit" }),
        );
        let reading = reading_from_sample("edge-7", &sample).expect("maps");
        assert_eq!(reading.series, "reg-explicit");
    }

    #[test]
    fn a_sample_without_a_numeric_value_is_rejected() {
        let sample = Sample::new(
            "rubix/ingest/edge-7/reg-1",
            serde_json::json!({ "temp": 21.5 }),
        );
        assert!(reading_from_sample("edge-7", &sample).is_err());
    }

    #[test]
    fn a_sample_without_a_stamp_falls_back_to_arrival_time() {
        let sample = Sample::new(
            "rubix/ingest/edge-7/reg-1",
            serde_json::json!({ "value": 1.0 }),
        );
        let before = surrealdb::types::Datetime::now();
        let reading = reading_from_sample("edge-7", &sample).expect("maps");
        // Arrival-time `at` is stamped around now, not left unset.
        assert!(reading.at >= before);
    }
}
