//! Compose the pre-processing nodes into one in-flight pipeline.
//!
//! A stream is decimated, then filtered, then enriched before it reaches
//! persistence (`rubix/docs/SCOPE.md`, "Ingestion and pre-processing"). The order
//! is deliberate: decimate first so the cheaper rate cut runs before the
//! per-sample predicate and field derivation; filter before enrich so dropped
//! samples are never enriched needlessly. Each stage is optional — a pipeline
//! with no nodes is a pass-through — and any stage that drops a sample short-
//! circuits the rest, so a decimated-out or filtered-out sample never persists.

use crate::subscribe::Sample;

use super::decimate::Decimator;
use super::enrich::Enricher;
use super::filter::Filter;

/// The in-flight pre-processing pipeline: decimate → filter → enrich.
///
/// Built additively ([`with_decimator`](Pipeline::with_decimator),
/// [`with_filter`](Pipeline::with_filter), [`with_enricher`](Pipeline::with_enricher))
/// so a caller wires only the stages it needs. [`admit`](Pipeline::admit) runs one
/// sample through every configured stage and returns the survivor, or `None` if a
/// stage dropped it.
#[derive(Default)]
pub struct Pipeline {
    decimator: Option<Decimator>,
    filter: Option<Filter>,
    enricher: Option<Enricher>,
}

impl Pipeline {
    /// An empty pipeline — every sample passes through unchanged.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach the rate-cut stage.
    #[must_use]
    pub fn with_decimator(mut self, decimator: Decimator) -> Self {
        self.decimator = Some(decimator);
        self
    }

    /// Attach the predicate-drop stage.
    #[must_use]
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Attach the field-enrichment stage.
    #[must_use]
    pub fn with_enricher(mut self, enricher: Enricher) -> Self {
        self.enricher = Some(enricher);
        self
    }

    /// Run `sample` through every configured stage.
    ///
    /// Returns the enriched survivor, or `None` if the decimator or filter dropped
    /// it. Decimation advances its phase even on a kept sample, so the pipeline is
    /// stateful across a stream and must be threaded through it by `&mut self`.
    #[must_use]
    pub fn admit(&mut self, sample: Sample) -> Option<Sample> {
        let sample = match &mut self.decimator {
            Some(decimator) => decimator.admit(sample)?,
            None => sample,
        };
        let sample = match &self.filter {
            Some(filter) => filter.admit(sample)?,
            None => sample,
        };
        let sample = match &self.enricher {
            Some(enricher) => enricher.enrich(sample),
            None => sample,
        };
        Some(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::Pipeline;
    use crate::process::{Decimator, Enricher, Filter};
    use crate::subscribe::Sample;

    fn reading(n: i64, temp: f64) -> Sample {
        Sample::new("rubix/ingest/edge/temp", serde_json::json!({ "n": n, "temp": temp }))
    }

    #[test]
    fn empty_pipeline_passes_every_sample_through() {
        let mut pipeline = Pipeline::new();
        assert!(pipeline.admit(reading(0, 20.0)).is_some());
    }

    #[test]
    fn decimate_then_filter_then_enrich_composes() {
        let mut pipeline = Pipeline::new()
            .with_decimator(Decimator::new(2))
            .with_filter(Filter::new(|s| {
                s.content.get("temp").and_then(serde_json::Value::as_f64).is_some_and(|t| t < 100.0)
            }))
            .with_enricher(Enricher::new(|_| {
                let mut fields = serde_json::Map::new();
                fields.insert("edge".to_owned(), serde_json::json!("edge-7"));
                fields
            }));

        // Kept by decimation (index 0), passes the filter, gets enriched.
        let kept = pipeline.admit(reading(0, 20.0)).expect("kept");
        assert_eq!(kept.content.get("edge"), Some(&serde_json::json!("edge-7")));
        // Dropped by decimation (index 1).
        assert!(pipeline.admit(reading(1, 20.0)).is_none());
        // Kept by decimation (index 2) but dropped by the filter.
        assert!(pipeline.admit(reading(2, 150.0)).is_none());
    }
}
