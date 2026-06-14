//! Integration: a pre-processed sample lands append-only in the edge partition.
//!
//! Persistence is the last ingest stage (`rubix/docs/sessions/WS-12.md`): a
//! sample that survived the pipeline is written as a fresh record into the
//! partition keyed by the edge identity (the principal's namespace, contract #5).
//! This test runs a small stream through the pipeline and asserts every survivor
//! lands under the edge partition, each as a distinct append (never an overwrite).

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_ingest::{Decimator, Enricher, Filter, Pipeline, Sample, append_sample, partition_for};

use fixture::open::{NS, granted_principal, open_ingest_store};

#[tokio::test]
async fn surviving_samples_append_under_the_edge_partition() {
    let handle = open_ingest_store("append").await;
    let principal = granted_principal(&handle, "ingestor").await;

    let mut pipeline = Pipeline::new()
        .with_decimator(Decimator::new(2))
        .with_filter(Filter::new(|s| {
            s.content.get("temp").and_then(serde_json::Value::as_f64).is_some_and(|t| t < 100.0)
        }))
        .with_enricher(Enricher::new(|_| {
            let mut fields = serde_json::Map::new();
            fields.insert("unit".to_owned(), serde_json::json!("celsius"));
            fields
        }));

    // A six-sample stream: decimation keeps indices 0, 2, 4; index 4 is out of
    // range and dropped by the filter, so two records should land.
    let stream = [20.0, 21.0, 22.0, 23.0, 150.0, 24.0];
    let mut persisted = Vec::new();
    for (n, temp) in stream.into_iter().enumerate() {
        let sample = Sample::new(
            "rubix/ingest/edge-7/temp",
            serde_json::json!({ "n": n, "temp": temp }),
        );
        if let Some(survivor) = pipeline.admit(sample) {
            let record = append_sample(handle.raw(), &principal, &survivor)
                .await
                .expect("append sample");
            persisted.push(record);
        }
    }

    assert_eq!(persisted.len(), 2, "decimate+filter should leave two survivors");

    // Every record landed in the edge partition (the principal's namespace) and
    // carries the enrichment.
    for record in &persisted {
        assert_eq!(record.namespace, partition_for(&principal));
        assert_eq!(record.namespace, NS);
        assert_eq!(record.content.get("unit"), Some(&serde_json::json!("celsius")));
    }

    // Appends are distinct rows, never an overwrite of one another.
    assert_ne!(persisted[0].id, persisted[1].id);

    // The append is durable: reading the record back returns it.
    let first = &persisted[0];
    let read_back = rubix_core::read_record(handle.raw(), &first.id)
        .await
        .expect("read back");
    assert_eq!(read_back.as_ref().map(|r| &r.namespace), Some(&first.namespace));
}
