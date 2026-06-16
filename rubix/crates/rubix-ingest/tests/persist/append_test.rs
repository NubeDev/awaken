//! Integration: a pre-processed sample lands append-only as a reading.
//!
//! Persistence is the last ingest stage (`rubix/docs/sessions/WS-12.md`): a
//! sample that survived the pipeline is written as a [`Reading`] into the
//! `reading` data plane, in the partition keyed by the edge identity (the
//! principal's namespace, contract #5; `rubix/docs/design/READINGS-TIMESERIES.md`).
//! This test runs a small stream through the pipeline and asserts every survivor
//! lands under the edge partition with its `series`/`value`/`at` mapped to typed
//! columns, each a distinct reading.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_ingest::{Decimator, Enricher, Filter, Pipeline, Sample, append_sample, partition_for};

use fixture::open::{NS, granted_principal, open_ingest_store};

#[tokio::test]
async fn surviving_samples_append_as_readings_under_the_edge_partition() {
    let handle = open_ingest_store("append").await;
    let principal = granted_principal(&handle, "ingestor").await;

    let mut pipeline = Pipeline::new()
        .with_decimator(Decimator::new(2))
        .with_filter(Filter::new(|s| {
            s.content.get("value").and_then(serde_json::Value::as_f64).is_some_and(|t| t < 100.0)
        }))
        .with_enricher(Enricher::new(|_| {
            let mut fields = serde_json::Map::new();
            fields.insert("unit".to_owned(), serde_json::json!("celsius"));
            fields
        }));

    // A six-sample stream: decimation keeps indices 0, 2, 4; index 4 is out of
    // range and dropped by the filter, so two readings should land. Each sample
    // is back-dated to a distinct `at` so the deterministic `(series, at)` ids
    // differ (a shared `at` would dedupe to one row).
    let stream = [20.0, 21.0, 22.0, 23.0, 150.0, 24.0];
    let mut persisted = Vec::new();
    for (n, value) in stream.into_iter().enumerate() {
        let sample = Sample::new(
            "rubix/ingest/edge-7/reg-temp",
            serde_json::json!({ "n": n, "value": value, "at": format!("2026-06-14T10:0{n}:00Z") }),
        );
        if let Some(survivor) = pipeline.admit(sample) {
            let reading = append_sample(handle.raw(), &principal, &survivor)
                .await
                .expect("append sample");
            persisted.push(reading);
        }
    }

    assert_eq!(persisted.len(), 2, "decimate+filter should leave two survivors");

    // Every reading landed in the edge partition with its series and enrichment.
    for reading in &persisted {
        assert_eq!(reading.namespace, partition_for(&principal));
        assert_eq!(reading.namespace, NS);
        assert_eq!(reading.series, "reg-temp", "series resolved from the key");
        assert_eq!(reading.content.get("unit"), Some(&serde_json::json!("celsius")));
    }

    // Distinct measurement instants → distinct deterministic ids.
    assert_ne!(persisted[0].id, persisted[1].id);

    // The append is durable: reading it back returns it.
    let first = &persisted[0];
    let read_back = rubix_core::read_reading(handle.raw(), &first.id)
        .await
        .expect("read back");
    assert_eq!(read_back.as_ref().map(|r| &r.value), Some(&first.value));
}
