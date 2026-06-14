//! Integration: the filter node drops samples by predicate.
//!
//! Samples the platform should not persist are dropped in flight
//! (`rubix/docs/sessions/WS-12.md`), exercising the public `Filter` API over a
//! mixed stream of in-range and out-of-range readings.

use rubix_ingest::{Filter, Sample};

fn reading(temp: f64) -> Sample {
    Sample::new("rubix/ingest/edge-7/temp", serde_json::json!({ "temp": temp }))
}

#[test]
fn out_of_range_readings_are_dropped() {
    let filter = Filter::new(|s| {
        s.content
            .get("temp")
            .and_then(serde_json::Value::as_f64)
            .is_some_and(|t| (-40.0..=85.0).contains(&t))
    });

    let stream = [reading(20.0), reading(200.0), reading(-100.0), reading(70.0)];
    let kept: Vec<_> = stream.into_iter().filter_map(|s| filter.admit(s)).collect();

    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].content, serde_json::json!({ "temp": 20.0 }));
    assert_eq!(kept[1].content, serde_json::json!({ "temp": 70.0 }));
}

#[test]
fn a_sample_missing_the_required_field_is_dropped() {
    let filter = Filter::new(|s| s.content.get("temp").is_some());
    let bad = Sample::new("rubix/ingest/edge-7/temp", serde_json::json!({ "humidity": 50 }));
    assert!(filter.admit(bad).is_none());
}
