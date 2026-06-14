//! Integration: the enrich node attaches the expected derived fields.
//!
//! Enrichment adds fields a downstream rule or query needs but the source did not
//! send (`rubix/docs/sessions/WS-12.md`), exercising the public `Enricher` API
//! and confirming derived fields land alongside the original content.

use rubix_ingest::{Enricher, Sample};

#[test]
fn enrich_attaches_the_edge_partition_derived_from_the_key() {
    let enricher = Enricher::new(|s| {
        let edge = s.key.split('/').nth(2).unwrap_or_default().to_owned();
        let mut fields = serde_json::Map::new();
        fields.insert("edge".to_owned(), serde_json::json!(edge));
        fields
    });

    let sample = Sample::new("rubix/ingest/edge-7/temp", serde_json::json!({ "temp": 21.5 }));
    let enriched = enricher.enrich(sample);

    assert_eq!(enriched.content.get("edge"), Some(&serde_json::json!("edge-7")));
    assert_eq!(enriched.content.get("temp"), Some(&serde_json::json!(21.5)));
}

#[test]
fn enrich_can_attach_multiple_fields() {
    let enricher = Enricher::new(|_| {
        let mut fields = serde_json::Map::new();
        fields.insert("unit".to_owned(), serde_json::json!("celsius"));
        fields.insert("kind".to_owned(), serde_json::json!("temperature"));
        fields
    });
    let enriched = enricher.enrich(Sample::new("rubix/ingest/edge-7/temp", serde_json::json!({ "temp": 1 })));
    assert_eq!(enriched.content.get("unit"), Some(&serde_json::json!("celsius")));
    assert_eq!(enriched.content.get("kind"), Some(&serde_json::json!("temperature")));
}
