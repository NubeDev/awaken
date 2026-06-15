//! Tier-B `trace_summary` rollup: one cheap row per correlation id.
//!
//! `rubix/docs/design/LAMINAR-BORROW.md` §5b: as child spans land, fold a
//! per-correlation-id summary so the trace list is **one read, not a tree walk**.
//! The summary is a *derived, high-churn rollup surface* (Tier B, §7) — a table,
//! not a record — and carries the §5a metrics folded out of each span:
//! `status = error` if any child errored, summed tokens/cost, a span count, and
//! the dominant ("top") span's name/kind.
//!
//! **Late arrivals.** Spans of one trace may persist out of order, and the
//! edge↔cloud sync (`rubix-sync`) may re-deliver a whole summary. `num_spans`
//! doubles as a **version column**: a summary built from more spans supersedes one
//! built from fewer ([`TraceSummary::supersedes`]), so a late or partial summary
//! can never clobber a more-complete one. The fold itself
//! ([`TraceSummary::fold`]) is monotonic — count and totals only grow, and an
//! `error` status, once set, sticks.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{Datetime, RecordId, SurrealValue};

use crate::error::{Result, TraceError};
use crate::span::Span;
use crate::span_metrics::{SpanMetrics, SpanStatus, read_kind};

/// The table trace-summary rows live in.
pub(crate) const SUMMARY_TABLE: &str = "trace_summary";

/// A per-correlation-id rollup of a trace's spans.
///
/// Every field is a fold over the trace's spans: [`status`](Self::status) is
/// `Error` if any folded span errored (else `Ok` if any was `Ok`, else `Unset`),
/// tokens/cost are sums, [`num_spans`](Self::num_spans) is the count (and the
/// late-arrival version), and the top span is the longest-running span folded so
/// far — the dominant step, a deterministic, well-defined "top".
#[derive(Debug, Clone, PartialEq)]
pub struct TraceSummary {
    /// The correlation id this summary rolls up.
    pub trace_id: String,
    /// The trace's status: error if any span errored.
    pub status: SpanStatus,
    /// Number of spans folded — also the late-arrival version (§6).
    pub num_spans: u64,
    /// Summed `span.tokens` across folded spans.
    pub total_tokens: i64,
    /// Summed `span.cost` across folded spans.
    pub total_cost: f64,
    /// Name of the longest-running span folded so far.
    pub top_span_name: String,
    /// Kind of the longest-running span folded so far, if it declared one.
    pub top_span_kind: Option<String>,
    /// Duration (ns) of the current top span — the comparison key, not persisted
    /// for query but kept so an incremental fold can compare a new span against it.
    top_span_duration_ns: i64,
}

impl TraceSummary {
    /// Seed a fresh summary from a trace's first folded span.
    #[must_use]
    pub fn seed(span: &Span) -> Self {
        let metrics = SpanMetrics::read(&span.attributes);
        Self {
            trace_id: span.trace_id.to_string(),
            status: metrics.status,
            num_spans: 1,
            total_tokens: metrics.tokens.unwrap_or(0),
            total_cost: metrics.cost.unwrap_or(0.0),
            top_span_name: span.name.clone(),
            top_span_kind: read_kind(&span.attributes),
            top_span_duration_ns: duration_ns(span),
        }
    }

    /// Fold one more span into this summary (monotonic).
    ///
    /// Count and totals grow; an `error` status, once set, sticks; the top span is
    /// replaced only by a strictly longer-running one, so the fold is stable under
    /// reordering (the same set of spans yields the same top regardless of arrival
    /// order, ties going to the first seen).
    pub fn fold(&mut self, span: &Span) {
        let metrics = SpanMetrics::read(&span.attributes);
        self.num_spans += 1;
        self.total_tokens += metrics.tokens.unwrap_or(0);
        self.total_cost += metrics.cost.unwrap_or(0.0);
        self.status = combine_status(self.status, metrics.status);
        let dur = duration_ns(span);
        if dur > self.top_span_duration_ns {
            self.top_span_duration_ns = dur;
            self.top_span_name = span.name.clone();
            self.top_span_kind = read_kind(&span.attributes);
        }
    }

    /// Whether `self` should win over `other` as the stored summary for a trace.
    ///
    /// The version tiebreak (§6): the summary folded from more spans is the more
    /// complete one and supersedes the other. Equal versions are treated as
    /// already-converged — `self` does not supersede an equal `other`, so a
    /// re-delivered identical summary is a no-op rather than a needless write.
    #[must_use]
    pub fn supersedes(&self, other: &TraceSummary) -> bool {
        self.num_spans > other.num_spans
    }
}

/// Span duration in ns, floored at zero so a malformed (end < start) span cannot
/// win the top-span comparison with a negative magnitude.
fn duration_ns(span: &Span) -> i64 {
    (span.end_ns - span.start_ns).max(0)
}

/// Combine two span statuses into a trace-level status.
///
/// `Error` dominates (any errored child makes the trace an error); else `Ok` if
/// either is `Ok`; else `Unset`. This is the lattice that makes the fold
/// order-independent.
fn combine_status(a: SpanStatus, b: SpanStatus) -> SpanStatus {
    if a.is_error() || b.is_error() {
        SpanStatus::Error
    } else if a == SpanStatus::Ok || b == SpanStatus::Ok {
        SpanStatus::Ok
    } else {
        SpanStatus::Unset
    }
}

/// SurrealDB-facing summary row: the reserved `id` thing, the owning namespace,
/// and the rolled-up fields. `status` is the [`SpanStatus`] wire string so the
/// surface is queryable without a custom type.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct SummaryRow {
    pub(crate) id: RecordId,
    pub(crate) namespace: String,
    pub(crate) trace_id: String,
    pub(crate) status: String,
    pub(crate) num_spans: i64,
    pub(crate) total_tokens: i64,
    pub(crate) total_cost: f64,
    pub(crate) top_span_name: String,
    pub(crate) top_span_kind: Option<String>,
    pub(crate) top_span_duration_ns: i64,
    pub(crate) updated: Datetime,
}

impl SummaryRow {
    /// The record-id key for a trace's summary, scoped per namespace so two
    /// tenants' summaries for the same correlation id never collide.
    fn key(namespace: &str, trace_id: &str) -> String {
        format!("{namespace}:{trace_id}")
    }

    fn from_summary(summary: &TraceSummary, namespace: &str) -> Self {
        Self {
            id: RecordId::new(SUMMARY_TABLE, Self::key(namespace, &summary.trace_id)),
            namespace: namespace.to_owned(),
            trace_id: summary.trace_id.clone(),
            status: summary.status.as_str().to_owned(),
            #[allow(clippy::cast_possible_wrap)]
            num_spans: summary.num_spans as i64,
            total_tokens: summary.total_tokens,
            total_cost: summary.total_cost,
            top_span_name: summary.top_span_name.clone(),
            top_span_kind: summary.top_span_kind.clone(),
            top_span_duration_ns: summary.top_span_duration_ns,
            updated: Datetime::now(),
        }
    }

    /// Rebuild the domain summary from a stored row. The top-span duration is
    /// persisted so the longest-span comparison holds across upserts — a later
    /// fold only replaces the top with a strictly longer span.
    fn into_summary(self) -> TraceSummary {
        TraceSummary {
            trace_id: self.trace_id,
            status: SpanStatus::parse(&self.status),
            #[allow(clippy::cast_sign_loss)]
            num_spans: self.num_spans.max(0) as u64,
            total_tokens: self.total_tokens,
            total_cost: self.total_cost,
            top_span_name: self.top_span_name,
            top_span_kind: self.top_span_kind,
            top_span_duration_ns: self.top_span_duration_ns,
        }
    }
}

/// Fold `span` into the stored `trace_summary` for its correlation id under
/// `namespace`, upserting the result.
///
/// Reads the current summary (if any), folds the span in, and writes it back —
/// the incremental per-span rollup of §5b. Runs on the root/owner handle, the
/// only session past the surface's write permission (see
/// [`define`](crate::define)). Returns the upserted summary.
///
/// The top span (longest-running, the dominant step) is chosen consistently
/// across upserts: its duration is persisted, so a later fold replaces the top
/// only with a strictly longer span.
///
/// # Errors
/// Returns [`TraceError::Persist`] if the read-back or upsert fails.
pub async fn upsert_summary(
    db: &Surreal<Db>,
    namespace: &str,
    span: &Span,
) -> Result<TraceSummary> {
    let key = SummaryRow::key(namespace, &span.trace_id.to_string());
    let existing: Option<SummaryRow> = db
        .select((SUMMARY_TABLE, key.as_str()))
        .await
        .map_err(TraceError::Persist)?;

    let summary = match existing {
        Some(row) => {
            let mut s = row.into_summary();
            s.fold(span);
            s
        }
        None => TraceSummary::seed(span),
    };

    let row = SummaryRow::from_summary(&summary, namespace);
    let _: Option<SummaryRow> = db
        .upsert((SUMMARY_TABLE, key.as_str()))
        .content(row)
        .await
        .map_err(TraceError::Persist)?;
    Ok(summary)
}

/// Read the stored `trace_summary` for `trace_id` under `namespace`, if any.
///
/// Returns `None` when no span of the trace has been rolled up (or the summary
/// was evicted). Reads on the passed handle, honoring the surface's row-level
/// read scope.
///
/// # Errors
/// Returns [`TraceError::Assemble`] if the read fails.
pub async fn read_summary(
    db: &Surreal<Db>,
    namespace: &str,
    trace_id: &str,
) -> Result<Option<TraceSummary>> {
    let key = SummaryRow::key(namespace, trace_id);
    let row: Option<SummaryRow> = db
        .select((SUMMARY_TABLE, key.as_str()))
        .await
        .map_err(TraceError::Assemble)?;
    Ok(row.map(SummaryRow::into_summary))
}

#[cfg(test)]
mod tests {
    use rubix_core::CorrelationId;

    use crate::span::Span;
    use crate::span_metrics::{MetricsBuilder, SpanStatus};

    use super::{combine_status, TraceSummary};

    fn span_with(name: &str, status: SpanStatus, tokens: i64, cost: f64, dur: i64) -> Span {
        let mut attrs = serde_json::json!({});
        MetricsBuilder::new()
            .kind("rule")
            .status(status)
            .tokens(tokens)
            .cost(cost)
            .apply(&mut attrs);
        Span::root(CorrelationId::carry("corr"), name, attrs, 0, dur)
    }

    #[test]
    fn fold_sums_metrics_counts_spans_and_picks_the_longest_as_top() {
        let mut s = TraceSummary::seed(&span_with("short", SpanStatus::Ok, 10, 1.0, 5));
        s.fold(&span_with("long", SpanStatus::Ok, 20, 2.5, 100));
        s.fold(&span_with("mid", SpanStatus::Ok, 5, 0.5, 50));

        assert_eq!(s.num_spans, 3);
        assert_eq!(s.total_tokens, 35);
        assert_eq!(s.total_cost, 4.0);
        assert_eq!(s.status, SpanStatus::Ok);
        assert_eq!(s.top_span_name, "long");
        assert_eq!(s.top_span_kind.as_deref(), Some("rule"));
    }

    #[test]
    fn any_errored_span_taints_the_trace_status() {
        let mut s = TraceSummary::seed(&span_with("a", SpanStatus::Ok, 0, 0.0, 1));
        s.fold(&span_with("b", SpanStatus::Error, 0, 0.0, 1));
        s.fold(&span_with("c", SpanStatus::Ok, 0, 0.0, 1));
        assert_eq!(s.status, SpanStatus::Error);
    }

    #[test]
    fn status_lattice_is_order_independent() {
        use SpanStatus::{Error, Ok, Unset};
        assert_eq!(combine_status(Unset, Unset), Unset);
        assert_eq!(combine_status(Unset, Ok), Ok);
        assert_eq!(combine_status(Ok, Unset), Ok);
        assert_eq!(combine_status(Ok, Error), Error);
        assert_eq!(combine_status(Error, Ok), Error);
    }

    #[test]
    fn a_more_complete_summary_supersedes_a_late_partial_one() {
        // Edge folded three spans; cloud only saw one before a late delivery.
        let mut complete = TraceSummary::seed(&span_with("a", SpanStatus::Error, 0, 0.0, 1));
        complete.fold(&span_with("b", SpanStatus::Ok, 0, 0.0, 1));
        complete.fold(&span_with("c", SpanStatus::Ok, 0, 0.0, 1));

        let partial = TraceSummary::seed(&span_with("a", SpanStatus::Ok, 0, 0.0, 1));

        assert!(complete.supersedes(&partial));
        // A late partial must NOT clobber the complete one...
        assert!(!partial.supersedes(&complete));
        // ...and an equal-version re-delivery is a no-op, not a needless overwrite.
        assert!(!complete.supersedes(&complete.clone()));
    }
}
