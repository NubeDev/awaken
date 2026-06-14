//! Drop samples that fail a predicate, in flight.
//!
//! A filter node removes samples the platform should not persist at all
//! (`rubix/docs/SCOPE.md`, "Ingestion and pre-processing": pre-process in flight
//! before persistence) — out-of-range readings, samples missing a required
//! field, or any other predicate over the decoded content. Filtering is a pure
//! drop decision: a sample either passes through unchanged or is discarded, never
//! rewritten (that is [`enrich`](crate::process::enrich)'s job).

use crate::subscribe::Sample;

/// A predicate-driven drop node over a sample stream.
///
/// The predicate is any `Fn(&Sample) -> bool`; a sample is kept when it returns
/// `true` and dropped otherwise. Holding the predicate by boxed closure lets a
/// caller express domain checks (range, presence, key match) without this node
/// baking any one of them in — the platform has no fixed ontology to validate
/// against (principle 4).
pub struct Filter {
    predicate: Box<dyn Fn(&Sample) -> bool + Send + Sync>,
}

impl Filter {
    /// Build a filter that keeps samples for which `predicate` returns `true`.
    pub fn new(predicate: impl Fn(&Sample) -> bool + Send + Sync + 'static) -> Self {
        Self {
            predicate: Box::new(predicate),
        }
    }

    /// Keep `sample` if it passes the predicate, else drop it.
    ///
    /// Returns `Some(sample)` (moved through unchanged) when kept, `None` when the
    /// predicate rejects it.
    #[must_use]
    pub fn admit(&self, sample: Sample) -> Option<Sample> {
        if (self.predicate)(&sample) {
            Some(sample)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Filter;
    use crate::subscribe::Sample;

    fn reading(temp: f64) -> Sample {
        Sample::new("rubix/ingest/edge/temp", serde_json::json!({ "temp": temp }))
    }

    #[test]
    fn predicate_true_keeps_the_sample() {
        let filter = Filter::new(|s| s.content.get("temp").is_some());
        assert!(filter.admit(reading(21.0)).is_some());
    }

    #[test]
    fn predicate_false_drops_the_sample() {
        let filter = Filter::new(|s| {
            s.content.get("temp").and_then(serde_json::Value::as_f64).is_some_and(|t| t < 100.0)
        });
        assert!(filter.admit(reading(150.0)).is_none());
        assert!(filter.admit(reading(20.0)).is_some());
    }

    #[test]
    fn a_missing_required_field_is_dropped() {
        let filter = Filter::new(|s| s.content.get("temp").is_some());
        let bad = Sample::new("rubix/ingest/edge/temp", serde_json::json!({ "humidity": 40 }));
        assert!(filter.admit(bad).is_none());
    }
}
