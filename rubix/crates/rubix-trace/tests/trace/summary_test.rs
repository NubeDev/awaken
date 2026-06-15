//! Integration: the `trace_summary` rollup folds and survives the store boundary.
//!
//! Proves the §5b surface end to end on a live engine (kv-mem): upserting spans
//! of one correlation id folds a single summary row (status/tokens/cost/count/top
//! span), and a late span keeps folding into — never resetting — that row. Also
//! checks tenant isolation, mirroring the `trace` table's row-level scope.

#[path = "open.rs"]
mod open;

use rubix_core::CorrelationId;
use rubix_trace::{MetricsBuilder, Span, SpanStatus, read_summary, upsert_summary};

use open::{NS, open_trace_store};

fn span(trace: &CorrelationId, name: &str, status: SpanStatus, tokens: i64, cost: f64, dur: i64) -> Span {
    let mut attrs = serde_json::json!({});
    MetricsBuilder::new()
        .kind("rule")
        .status(status)
        .tokens(tokens)
        .cost(cost)
        .apply(&mut attrs);
    Span::root(trace.clone(), name, attrs, 0, dur)
}

#[tokio::test]
async fn upserting_spans_folds_one_summary_row() {
    let handle = open_trace_store("summary_fold").await;
    let trace = CorrelationId::carry("corr-summary");

    upsert_summary(handle.raw(), NS, &span(&trace, "short", SpanStatus::Ok, 10, 1.0, 5))
        .await
        .expect("fold 1");
    upsert_summary(handle.raw(), NS, &span(&trace, "long", SpanStatus::Ok, 20, 2.5, 100))
        .await
        .expect("fold 2");
    upsert_summary(handle.raw(), NS, &span(&trace, "mid", SpanStatus::Ok, 5, 0.5, 50))
        .await
        .expect("fold 3");

    let summary = read_summary(handle.raw(), NS, &trace.to_string())
        .await
        .expect("read")
        .expect("summary exists");

    assert_eq!(summary.num_spans, 3);
    assert_eq!(summary.total_tokens, 35);
    assert_eq!(summary.total_cost, 4.0);
    assert_eq!(summary.status, SpanStatus::Ok);
    assert_eq!(summary.top_span_name, "long");
    assert_eq!(summary.top_span_kind.as_deref(), Some("rule"));
}

#[tokio::test]
async fn a_late_errored_span_taints_status_and_grows_the_version() {
    let handle = open_trace_store("summary_late").await;
    let trace = CorrelationId::carry("corr-late");

    // A "complete" ok summary persists first.
    upsert_summary(handle.raw(), NS, &span(&trace, "a", SpanStatus::Ok, 0, 0.0, 1))
        .await
        .expect("fold a");
    upsert_summary(handle.raw(), NS, &span(&trace, "b", SpanStatus::Ok, 0, 0.0, 1))
        .await
        .expect("fold b");
    let before = read_summary(handle.raw(), NS, &trace.to_string())
        .await
        .expect("read before")
        .expect("exists");
    assert_eq!(before.status, SpanStatus::Ok);
    assert_eq!(before.num_spans, 2);

    // A late errored span folds in: status flips to error, version (num_spans)
    // grows — the late arrival updates rather than clobbering the rollup.
    let after = upsert_summary(handle.raw(), NS, &span(&trace, "c", SpanStatus::Error, 0, 0.0, 1))
        .await
        .expect("fold late c");
    assert_eq!(after.status, SpanStatus::Error);
    assert_eq!(after.num_spans, 3);
    assert!(after.supersedes(&before), "the folded summary supersedes the earlier version");
}

#[tokio::test]
async fn summaries_of_different_tenants_do_not_collide() {
    let handle = open_trace_store("summary_isolation").await;
    // Same correlation id, two namespaces — keyed per tenant, so independent.
    let trace = CorrelationId::carry("corr-shared");

    upsert_summary(handle.raw(), "tenant-a", &span(&trace, "a", SpanStatus::Ok, 1, 0.0, 1))
        .await
        .expect("fold a");
    upsert_summary(handle.raw(), "tenant-b", &span(&trace, "b", SpanStatus::Error, 2, 0.0, 1))
        .await
        .expect("fold b");

    let a = read_summary(handle.raw(), "tenant-a", &trace.to_string())
        .await
        .expect("read a")
        .expect("a exists");
    let b = read_summary(handle.raw(), "tenant-b", &trace.to_string())
        .await
        .expect("read b")
        .expect("b exists");

    assert_eq!(a.num_spans, 1);
    assert_eq!(a.status, SpanStatus::Ok);
    assert_eq!(b.num_spans, 1);
    assert_eq!(b.status, SpanStatus::Error);
}
