//! Integration: the rolling retention bound evicts the oldest spans.
//!
//! Proves contract #4 (`rubix/STACK-DEISGN.md`): traces are bounded — not kept
//! forever. Persist more spans than the bound, enforce retention, and assert the
//! count drops to the bound with the most-recent spans surviving (kv-mem).

#[path = "open.rs"]
mod open;

use rubix_core::CorrelationId;
use rubix_trace::{SampleRate, Span, assemble_trace, enforce_retention, persist_span};

use open::{NS, open_trace_store};

/// Count the spans stored under `trace` by assembling and flattening its forest.
async fn stored_span_count(handle: &rubix_store::StoreHandle, trace: &CorrelationId) -> usize {
    let forest = assemble_trace(handle.raw(), trace).await.expect("assemble");
    fn count(nodes: &[rubix_trace::SpanNode]) -> usize {
        nodes.iter().map(|n| 1 + count(&n.children)).sum()
    }
    count(&forest)
}

#[tokio::test]
async fn retention_evicts_the_oldest_spans_past_the_bound() {
    let handle = open_trace_store("retain_bound").await;
    let trace = CorrelationId::carry("corr-retain");
    let rate = SampleRate::new(0.0);

    // Append ten root spans in order; the `appended` clock orders them.
    let mut spans = Vec::new();
    for i in 0..10 {
        let span = Span::root(trace.clone(), format!("s{i}"), serde_json::json!({ "i": i }), i, i + 1);
        persist_span(handle.raw(), NS, &span, rate)
            .await
            .expect("persist span");
        spans.push(span);
    }
    assert_eq!(stored_span_count(&handle, &trace).await, 10);

    // Cap the namespace at three spans; seven oldest should be evicted.
    let evicted = enforce_retention(handle.raw(), NS, 3)
        .await
        .expect("enforce retention");
    assert_eq!(evicted, 7);
    assert_eq!(stored_span_count(&handle, &trace).await, 3, "bound holds at three");

    // The survivors are a subset of the spans appended, and the count is exactly
    // the bound — the rolling cut left the namespace at its ceiling.
    let forest = assemble_trace(handle.raw(), &trace).await.expect("assemble");
    let originals: Vec<&str> = spans.iter().map(|s| s.name.as_str()).collect();
    for node in &forest {
        assert!(
            originals.contains(&node.span.name.as_str()),
            "survivor {} was one of the appended spans",
            node.span.name
        );
    }
}

#[tokio::test]
async fn retention_under_the_bound_is_a_no_op() {
    let handle = open_trace_store("retain_noop").await;
    let trace = CorrelationId::carry("corr-noop");
    let rate = SampleRate::new(0.0);

    for i in 0..3 {
        persist_span(handle.raw(), NS, &Span::root(trace.clone(), format!("s{i}"), serde_json::json!({}), i, i + 1), rate)
            .await
            .expect("persist span");
    }

    let evicted = enforce_retention(handle.raw(), NS, 10)
        .await
        .expect("enforce retention");
    assert_eq!(evicted, 0);
    assert_eq!(stored_span_count(&handle, &trace).await, 3);
}

#[tokio::test]
async fn a_zero_bound_evicts_everything() {
    let handle = open_trace_store("retain_zero").await;
    let trace = CorrelationId::carry("corr-zero");
    let rate = SampleRate::new(0.0);

    for i in 0..5 {
        persist_span(handle.raw(), NS, &Span::root(trace.clone(), format!("s{i}"), serde_json::json!({}), i, i + 1), rate)
            .await
            .expect("persist span");
    }

    let evicted = enforce_retention(handle.raw(), NS, 0)
        .await
        .expect("enforce retention");
    assert_eq!(evicted, 5);
    assert_eq!(stored_span_count(&handle, &trace).await, 0);
}
