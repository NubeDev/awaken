//! Integration: a persisted span round-trips through the `trace` table.
//!
//! Proves the durable append path: persist a span on the root handle, then read
//! it back via `assemble_trace` and assert every field — trace id, name, parent
//! link, attributes, timing — survives the store boundary unchanged (kv-mem).

#[path = "open.rs"]
mod open;

use rubix_core::CorrelationId;
use rubix_trace::{Persisted, SampleRate, Span, assemble_trace, persist_span};

use open::{NS, open_trace_store};

#[tokio::test]
async fn a_persisted_span_reads_back_with_all_fields_intact() {
    let handle = open_trace_store("persist_roundtrip").await;
    let trace = CorrelationId::carry("corr-persist");
    let span = Span::root(
        trace.clone(),
        "rule.eval",
        serde_json::json!({ "input": 21, "fired": true }),
        100,
        200,
    );

    let outcome = persist_span(handle.raw(), NS, &span, SampleRate::new(0.0))
        .await
        .expect("persist span");
    assert_eq!(outcome, Persisted::Written);

    let forest = assemble_trace(handle.raw(), &trace).await.expect("assemble");
    assert_eq!(forest.len(), 1);
    let read = &forest[0].span;
    assert_eq!(read, &span);
    assert_eq!(read.attributes, serde_json::json!({ "input": 21, "fired": true }));
    assert_eq!(read.start_ns, 100);
    assert_eq!(read.end_ns, 200);
}

#[tokio::test]
async fn spans_of_different_traces_do_not_bleed_together() {
    let handle = open_trace_store("persist_isolation").await;
    let trace_a = CorrelationId::carry("corr-a");
    let trace_b = CorrelationId::carry("corr-b");
    let rate = SampleRate::new(0.0);

    persist_span(handle.raw(), NS, &Span::root(trace_a.clone(), "a", serde_json::json!({}), 0, 1), rate)
        .await
        .expect("persist a");
    persist_span(handle.raw(), NS, &Span::root(trace_b.clone(), "b", serde_json::json!({}), 0, 1), rate)
        .await
        .expect("persist b");

    let forest_a = assemble_trace(handle.raw(), &trace_a).await.expect("assemble a");
    assert_eq!(forest_a.len(), 1);
    assert_eq!(forest_a[0].span.name, "a");
}
