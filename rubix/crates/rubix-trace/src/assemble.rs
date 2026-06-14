//! Read path: assemble a span tree by trace id.
//!
//! `rubix/docs/SCOPE.md`, "Tracing": the stored spans of one operation form a
//! tree (root entry point, child steps) that answers "how data/decisions flowed"
//! — and, once WS-11 lands the Rhai per-evaluation spans, "why did this fire".
//! This verb reads every span sharing a `trace_id` and links them by
//! `parent_span_id` into [`SpanNode`] roots. Reads run on whatever handle is
//! passed: the root handle sees all spans, a gate-issued scoped session sees only
//! its tenant's spans (the `trace` table's row-level read permission, see
//! [`define`](crate::define)).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use std::collections::HashMap;

use rubix_core::{CorrelationId, Id};

use crate::error::{Result, TraceError};
use crate::row::{SpanRow, TRACE_TABLE};
use crate::span::Span;

/// One node in an assembled span tree: a span plus its child spans, ordered by
/// start time.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanNode {
    /// The span at this node.
    pub span: Span,
    /// Child spans whose `parent_span_id` is this span's id, start-ordered.
    pub children: Vec<SpanNode>,
}

/// Assemble the span tree(s) for `trace_id` from stored spans.
///
/// Returns the root nodes (spans with no parent, or whose parent is absent from
/// the trace) start-ordered, each with its children linked recursively. An
/// unknown trace id yields an empty `Vec` rather than an error — a trace may have
/// been fully evicted by retention or never sampled.
///
/// # Errors
/// Returns [`TraceError::Assemble`] if the read fails.
pub async fn assemble_trace(db: &Surreal<Db>, trace_id: &CorrelationId) -> Result<Vec<SpanNode>> {
    let rows: Vec<SpanRow> = db
        .query(format!(
            "SELECT * FROM {TRACE_TABLE} WHERE trace_id = $trace ORDER BY start_ns ASC"
        ))
        .bind(("trace", trace_id.to_string()))
        .await
        .map_err(TraceError::Assemble)?
        .take(0)
        .map_err(TraceError::Assemble)?;
    let spans: Vec<Span> = rows.into_iter().map(SpanRow::into_span).collect();
    Ok(build_tree(spans))
}

/// Link a flat, start-ordered span list into parent/child trees.
///
/// Children of a parent are collected in input order, which is start-ordered by
/// the query — so each node's `children` stay start-ordered. A span whose parent
/// id is not among the supplied spans is treated as a root, so a partially
/// evicted trace still assembles into the surviving forest rather than vanishing.
fn build_tree(spans: Vec<Span>) -> Vec<SpanNode> {
    let present: std::collections::HashSet<String> = spans
        .iter()
        .map(|s| s.span_id.as_str().to_owned())
        .collect();

    let mut children: HashMap<String, Vec<Span>> = HashMap::new();
    let mut roots: Vec<Span> = Vec::new();
    for span in spans {
        match parent_present(&span, &present) {
            Some(parent_key) => children.entry(parent_key).or_default().push(span),
            None => roots.push(span),
        }
    }

    roots
        .into_iter()
        .map(|span| attach(span, &mut children))
        .collect()
}

/// The parent's id key when that parent is present in the trace, else `None`.
fn parent_present(span: &Span, present: &std::collections::HashSet<String>) -> Option<String> {
    span.parent_span_id
        .as_ref()
        .map(Id::as_str)
        .filter(|key| present.contains(*key))
        .map(str::to_owned)
}

/// Recursively attach a span's children, draining them from the pending map.
fn attach(span: Span, children: &mut HashMap<String, Vec<Span>>) -> SpanNode {
    let kids = children.remove(span.span_id.as_str()).unwrap_or_default();
    let nodes = kids
        .into_iter()
        .map(|child| attach(child, children))
        .collect();
    SpanNode {
        span,
        children: nodes,
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::CorrelationId;

    use crate::span::Span;

    use super::build_tree;

    #[test]
    fn a_root_with_two_children_assembles_into_one_tree() {
        let trace = CorrelationId::carry("corr-tree");
        let root = Span::root(trace, "root", serde_json::json!({}), 0, 100);
        let a = Span::child(&root, "a", serde_json::json!({}), 1, 10);
        let b = Span::child(&root, "b", serde_json::json!({}), 11, 20);
        let forest = build_tree(vec![root.clone(), a.clone(), b.clone()]);
        assert_eq!(forest.len(), 1);
        assert_eq!(forest[0].span, root);
        assert_eq!(forest[0].children.len(), 2);
        assert_eq!(forest[0].children[0].span, a);
        assert_eq!(forest[0].children[1].span, b);
    }

    #[test]
    fn an_orphan_whose_parent_is_absent_becomes_a_root() {
        let trace = CorrelationId::carry("corr-orphan");
        let root = Span::root(trace.clone(), "root", serde_json::json!({}), 0, 100);
        let orphan_parent = Span::root(trace, "evicted", serde_json::json!({}), 0, 5);
        let orphan = Span::child(&orphan_parent, "child", serde_json::json!({}), 1, 4);
        // Note: `orphan_parent` is deliberately omitted (modelling eviction).
        let forest = build_tree(vec![root, orphan.clone()]);
        assert_eq!(forest.len(), 2);
        assert!(forest.iter().any(|n| n.span == orphan));
    }

    #[test]
    fn an_empty_span_list_yields_an_empty_forest() {
        assert!(build_tree(Vec::new()).is_empty());
    }
}
