//! The persisted shape of a [`Span`] at the SurrealDB boundary.
//!
//! SurrealDB owns the reserved `id` as a `RecordId`; the domain [`Span`] carries
//! a plain string [`Id`]. The row adds the two store-only concerns the domain
//! span stays free of: the `namespace` the row-level read permission scopes on
//! (contract #5, edge-partitioned data) and `appended`, the store's own append
//! time used to order retention eviction independently of the caller-supplied
//! span clock.

use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue, ToSql};

use rubix_core::{CorrelationId, Id};

use crate::span::Span;

/// The table span rows live in.
pub(crate) const TRACE_TABLE: &str = "trace";

/// SurrealDB-facing span: the reserved `id` thing plus span fields, the owning
/// namespace, and the store append time.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct SpanRow {
    pub(crate) id: RecordId,
    pub(crate) namespace: String,
    pub(crate) trace_id: String,
    pub(crate) parent_span_id: Option<String>,
    pub(crate) name: String,
    pub(crate) attributes: serde_json::Value,
    pub(crate) start_ns: i64,
    pub(crate) end_ns: i64,
    pub(crate) appended: Datetime,
}

impl SpanRow {
    /// Project a domain [`Span`] into its persisted row under `namespace`,
    /// stamped with the current append time.
    pub(crate) fn from_span(span: &Span, namespace: &str) -> Self {
        Self {
            id: RecordId::new(TRACE_TABLE, span.span_id.as_str()),
            namespace: namespace.to_owned(),
            trace_id: span.trace_id.to_string(),
            parent_span_id: span.parent_span_id.as_ref().map(|p| p.to_string()),
            name: span.name.clone(),
            attributes: span.attributes.clone(),
            start_ns: span.start_ns,
            end_ns: span.end_ns,
            appended: Datetime::now(),
        }
    }

    /// Reconstruct the domain [`Span`] from a persisted row.
    pub(crate) fn into_span(self) -> Span {
        Span {
            span_id: Id::from_raw(span_key(&self.id)),
            trace_id: CorrelationId::carry(self.trace_id),
            parent_span_id: self.parent_span_id.map(Id::from_raw),
            name: self.name,
            attributes: self.attributes,
            start_ns: self.start_ns,
            end_ns: self.end_ns,
        }
    }
}

/// The raw string form of a span id's key (the part after `trace:`).
fn span_key(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => other.to_sql(),
    }
}
