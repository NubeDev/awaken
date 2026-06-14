//! Integration: decimation reduces a sample stream to the target rate.
//!
//! High-rate streams are cut in flight before persistence
//! (`rubix/docs/sessions/WS-12.md`). A decimator with factor N keeps one sample
//! out of every N over a realistic run, exercising the public `Decimator` API.

use rubix_ingest::{Decimator, Sample};

fn sample(n: i64) -> Sample {
    Sample::new("rubix/ingest/edge-7/temp", serde_json::json!({ "n": n }))
}

#[test]
fn factor_four_keeps_one_quarter_of_a_long_stream() {
    let mut decimator = Decimator::new(4);
    let kept: Vec<_> = (0..40).filter_map(|n| decimator.admit(sample(n))).collect();
    assert_eq!(kept.len(), 10);
    // The first sample of each window of four is the one kept.
    for (index, kept_sample) in kept.iter().enumerate() {
        let expected = (index as i64) * 4;
        assert_eq!(kept_sample.content, serde_json::json!({ "n": expected }));
    }
}

#[test]
fn the_target_rate_is_one_in_factor() {
    let mut decimator = Decimator::new(10);
    let kept = (0..100).filter(|n| decimator.admit(sample(*n)).is_some()).count();
    assert_eq!(kept, 10);
}
