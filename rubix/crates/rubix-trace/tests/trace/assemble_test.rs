//! Integration: a multi-span operation assembles into a tree by trace id.
//!
//! Proves the read path (`rubix/docs/SCOPE.md`, "Tracing"): persist a root span
//! with two children and a grandchild, then assemble by trace id and assert the
//! parent/child shape and start-ordering — the hook WS-11's Rhai span tree plugs
//! into. Verified on kv-mem.

#[path = "open.rs"]
mod open;

use rubix_core::CorrelationId;
use rubix_trace::{SampleRate, Span, assemble_trace, persist_span};

use open::{NS, open_trace_store};

#[tokio::test]
async fn a_multi_span_operation_assembles_into_its_tree() {
    let handle = open_trace_store("assemble_tree").await;
    let trace = CorrelationId::carry("corr-assemble");
    let rate = SampleRate::new(0.0);

    let root = Span::root(trace.clone(), "ingest", serde_json::json!({}), 0, 100);
    let pre = Span::child(&root, "preprocess", serde_json::json!({}), 5, 40);
    let rule = Span::child(&root, "rule.eval", serde_json::json!({}), 45, 90);
    let subrule = Span::child(&rule, "rule.subcall", serde_json::json!({}), 50, 80);

    for span in [&root, &pre, &rule, &subrule] {
        persist_span(handle.raw(), NS, span, rate)
            .await
            .expect("persist span");
    }

    let forest = assemble_trace(handle.raw(), &trace).await.expect("assemble");
    assert_eq!(forest.len(), 1, "single root");
    let root_node = &forest[0];
    assert_eq!(root_node.span, root);
    assert_eq!(root_node.children.len(), 2, "preprocess + rule.eval");
    // Start-ordered: preprocess (start 5) before rule.eval (start 45).
    assert_eq!(root_node.children[0].span, pre);
    assert_eq!(root_node.children[1].span, rule);
    // The grandchild hangs off rule.eval.
    assert_eq!(root_node.children[1].children.len(), 1);
    assert_eq!(root_node.children[1].children[0].span, subrule);
}

#[tokio::test]
async fn an_unknown_trace_id_assembles_to_an_empty_forest() {
    let handle = open_trace_store("assemble_unknown").await;
    let forest = assemble_trace(handle.raw(), &CorrelationId::carry("corr-missing"))
        .await
        .expect("assemble");
    assert!(forest.is_empty());
}
