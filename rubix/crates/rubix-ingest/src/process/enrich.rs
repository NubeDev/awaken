//! Attach derived or contextual fields to a sample, in flight.
//!
//! Enrichment adds fields a downstream rule or query needs but the source did not
//! send (`rubix/docs/SCOPE.md`, "Ingestion and pre-processing": enrich in flight
//! before persistence) — the edge identity the sample arrived under, a derived
//! value, a unit tag. Unlike [`filter`](crate::process::filter) (a drop) and
//! [`decimate`](crate::process::decimate) (a rate cut), enrichment is the one
//! node that *rewrites* content, so it is kept distinct and additive: it merges
//! new fields into the existing object and never discards a sample.

use crate::subscribe::Sample;

/// The fields an enrichment derives from a sample, ready to merge into content.
type DerivedFields = serde_json::Map<String, serde_json::Value>;

/// The boxed closure that derives the fields to merge for one sample.
type DeriveFn = Box<dyn Fn(&Sample) -> DerivedFields + Send + Sync>;

/// A field-attaching node over a sample stream.
///
/// The enrichment fn maps a sample to the JSON object of fields to merge into its
/// content. Holding it as a boxed closure lets a caller derive context (the edge
/// partition, a computed value, a constant tag) without this node fixing any one
/// enrichment — the platform bakes in no ontology (principle 4).
pub struct Enricher {
    derive: DeriveFn,
}

impl Enricher {
    /// Build an enricher whose `derive` fn yields the fields to merge per sample.
    pub fn new(
        derive: impl Fn(&Sample) -> DerivedFields + Send + Sync + 'static,
    ) -> Self {
        Self {
            derive: Box::new(derive),
        }
    }

    /// Merge the derived fields into `sample`'s content and return it.
    ///
    /// Derived fields overwrite same-named existing fields (the enrichment is the
    /// authority on what it sets); content that is not a JSON object is left
    /// untouched, since there is no object to merge into.
    #[must_use]
    pub fn enrich(&self, mut sample: Sample) -> Sample {
        let derived = (self.derive)(&sample);
        if let Some(object) = sample.content.as_object_mut() {
            for (key, value) in derived {
                object.insert(key, value);
            }
        }
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::Enricher;
    use crate::subscribe::Sample;

    #[test]
    fn enrich_attaches_a_derived_field() {
        let enricher = Enricher::new(|_| {
            let mut fields = serde_json::Map::new();
            fields.insert("source".to_owned(), serde_json::json!("edge-7"));
            fields
        });
        let sample = Sample::new("rubix/ingest/edge-7/temp", serde_json::json!({ "temp": 21.0 }));
        let enriched = enricher.enrich(sample);
        assert_eq!(enriched.content.get("source"), Some(&serde_json::json!("edge-7")));
        assert_eq!(enriched.content.get("temp"), Some(&serde_json::json!(21.0)));
    }

    #[test]
    fn enrich_can_derive_from_the_key() {
        let enricher = Enricher::new(|s| {
            let mut fields = serde_json::Map::new();
            fields.insert("key".to_owned(), serde_json::json!(s.key.clone()));
            fields
        });
        let sample = Sample::new("rubix/ingest/edge-7/temp", serde_json::json!({ "temp": 1 }));
        let enriched = enricher.enrich(sample);
        assert_eq!(enriched.content.get("key"), Some(&serde_json::json!("rubix/ingest/edge-7/temp")));
    }

    #[test]
    fn non_object_content_is_left_untouched() {
        let enricher = Enricher::new(|_| {
            let mut fields = serde_json::Map::new();
            fields.insert("x".to_owned(), serde_json::json!(1));
            fields
        });
        let sample = Sample::new("rubix/ingest/edge/raw", serde_json::json!(42));
        let enriched = enricher.enrich(sample.clone());
        assert_eq!(enriched.content, serde_json::json!(42));
    }
}
