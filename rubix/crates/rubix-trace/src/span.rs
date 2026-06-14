//! The span — one unit of "how data/decisions flowed".
//!
//! A span records one step as work flows ingest → pre-process → rule → insight →
//! sink (`rubix/docs/SCOPE.md`, "Tracing"). Its `trace_id` is the WS-05
//! correlation id minted at the gate or at ingest (contract #3,
//! `rubix/STACK-DEISGN.md`), so every span of one operation shares the same
//! trace id and a reader can pivot from an insight to the whole flow that
//! produced it. A `parent_span_id` links a span to the step that spawned it;
//! a root span (the operation's entry point) has none. This is the minimal
//! model WS-11's Rhai per-evaluation span tree plugs into.

use rubix_core::{CorrelationId, Id};

/// One span in a trace: a named step with a parent link, attributes, and a
/// start/end timestamp pair carried as caller-supplied epoch nanoseconds.
///
/// Timestamps are plain integers (epoch nanoseconds) rather than a clock type so
/// the span model stays free of an I/O dependency — the caller stamps them from
/// whatever clock it already holds, and the store records its own append time
/// separately for retention ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// This span's own id, unique within and across traces.
    pub span_id: Id,
    /// The correlation id this span belongs to — the WS-05 trace id (contract #3).
    pub trace_id: CorrelationId,
    /// The id of the span that spawned this one; `None` for a root span.
    pub parent_span_id: Option<Id>,
    /// The step name (e.g. `ingest.decimate`, `rule.eval`).
    pub name: String,
    /// Free-form attributes — the values seen at this step.
    pub attributes: serde_json::Value,
    /// Span start, epoch nanoseconds.
    pub start_ns: i64,
    /// Span end, epoch nanoseconds.
    pub end_ns: i64,
}

impl Span {
    /// Open a root span for `trace_id` — the entry point of a traced operation,
    /// with no parent.
    #[must_use]
    pub fn root(
        trace_id: CorrelationId,
        name: impl Into<String>,
        attributes: serde_json::Value,
        start_ns: i64,
        end_ns: i64,
    ) -> Self {
        Self {
            span_id: Id::new(),
            trace_id,
            parent_span_id: None,
            name: name.into(),
            attributes,
            start_ns,
            end_ns,
        }
    }

    /// Open a child span under `parent` — same trace id, linked to the parent's
    /// span id.
    ///
    /// The child inherits the parent's `trace_id` by construction, so a tree can
    /// never span two correlation ids (contract #3, `rubix/STACK-DEISGN.md`).
    #[must_use]
    pub fn child(
        parent: &Span,
        name: impl Into<String>,
        attributes: serde_json::Value,
        start_ns: i64,
        end_ns: i64,
    ) -> Self {
        Self {
            span_id: Id::new(),
            trace_id: parent.trace_id.clone(),
            parent_span_id: Some(parent.span_id.clone()),
            name: name.into(),
            attributes,
            start_ns,
            end_ns,
        }
    }

    /// Whether this span is a root (no parent).
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent_span_id.is_none()
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::CorrelationId;

    use super::Span;

    #[test]
    fn root_span_has_no_parent_and_carries_the_trace_id() {
        let trace = CorrelationId::carry("corr-1");
        let span = Span::root(trace.clone(), "ingest", serde_json::json!({ "n": 1 }), 0, 10);
        assert!(span.is_root());
        assert_eq!(span.trace_id, trace);
        assert_eq!(span.parent_span_id, None);
    }

    #[test]
    fn child_inherits_trace_id_and_links_to_parent() {
        let trace = CorrelationId::carry("corr-2");
        let parent = Span::root(trace.clone(), "rule.eval", serde_json::json!({}), 0, 20);
        let child = Span::child(&parent, "rule.subcall", serde_json::json!({ "v": 7 }), 1, 5);
        assert!(!child.is_root());
        assert_eq!(child.trace_id, trace);
        assert_eq!(child.parent_span_id.as_ref(), Some(&parent.span_id));
    }

    #[test]
    fn each_span_gets_a_distinct_id() {
        let trace = CorrelationId::carry("corr-3");
        let a = Span::root(trace.clone(), "a", serde_json::json!({}), 0, 1);
        let b = Span::root(trace, "b", serde_json::json!({}), 0, 1);
        assert_ne!(a.span_id, b.span_id);
    }
}
