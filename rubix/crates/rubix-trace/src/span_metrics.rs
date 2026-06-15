//! Reserved span-attribute keys and the typed metrics folded out of them.
//!
//! Spans stay generic by construction — a [`Span`](crate::Span) is just a name
//! plus free-form `attributes` (`rubix/docs/design/LAMINAR-BORROW.md` §5a). Rather
//! than promote a closed `span_kind` enum (a breaking contract change that fights
//! the generic model), we reserve a small set of **well-known attribute keys** —
//! [`SPAN_KIND`], [`SPAN_STATUS`], [`SPAN_TOKENS`], [`SPAN_COST`] — that emitters
//! set consistently and the trace rollup (§5b) folds typed metrics out of at
//! persist time. No schema break, no enum to grow to regret.
//!
//! This module owns three things: the reserved key constants, a [`SpanStatus`]
//! for the one value with a fixed vocabulary, and the read/write helpers
//! ([`SpanMetrics::read`], [`MetricsBuilder`]) that keep the keys' shapes
//! consistent across every emitter.

use serde_json::Value;

/// Reserved attribute key for a span's kind — a free-form category label
/// (`llm`, `tool`, `rule`, …). Deliberately a string, not a closed enum, so the
/// span model stays generic; readers filter on whatever vocabulary emitters use.
pub const SPAN_KIND: &str = "span.kind";

/// Reserved attribute key for a span's status — see [`SpanStatus`].
pub const SPAN_STATUS: &str = "span.status";

/// Reserved attribute key for a span's token count (LLM / model work).
pub const SPAN_TOKENS: &str = "span.tokens";

/// Reserved attribute key for a span's monetary cost.
pub const SPAN_COST: &str = "span.cost";

/// The status of the work a span recorded.
///
/// This is the one reserved key with a fixed vocabulary, because the rollup
/// (§5b) folds it deterministically: a trace summary is `Error` if *any* child
/// span errored. [`SpanStatus::Unset`] is the default when an emitter does not
/// declare a status — it is neither ok nor error and never poisons a rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpanStatus {
    /// No status declared — the default; does not affect a rollup.
    #[default]
    Unset,
    /// The step completed successfully.
    Ok,
    /// The step errored — taints the enclosing trace summary's status.
    Error,
}

impl SpanStatus {
    /// The wire string written under [`SPAN_STATUS`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SpanStatus::Unset => "unset",
            SpanStatus::Ok => "ok",
            SpanStatus::Error => "error",
        }
    }

    /// Parse a status from its wire string; anything unrecognized (including a
    /// missing key) folds to [`SpanStatus::Unset`] so a malformed attribute can
    /// never be read as an error.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "ok" => SpanStatus::Ok,
            "error" => SpanStatus::Error,
            _ => SpanStatus::Unset,
        }
    }

    /// Whether this status is [`SpanStatus::Error`] — the rollup poison test.
    #[must_use]
    pub fn is_error(self) -> bool {
        matches!(self, SpanStatus::Error)
    }
}

/// The typed metrics folded out of a span's reserved attribute keys.
///
/// Every field defaults harmlessly: an absent or malformed key yields `None`
/// (or [`SpanStatus::Unset`]), never an error, so reading metrics off an
/// arbitrary span — including one written before this schema existed — is always
/// safe and lossless about what it cannot find.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpanMetrics {
    /// The span's kind label, if set (free-form; see [`SPAN_KIND`]).
    pub kind_present: bool,
    /// The span's status; [`SpanStatus::Unset`] if absent.
    pub status: SpanStatus,
    /// The span's token count, if set.
    pub tokens: Option<i64>,
    /// The span's monetary cost, if set.
    pub cost: Option<f64>,
}

impl SpanMetrics {
    /// Fold the reserved metric keys out of an `attributes` value.
    ///
    /// Tolerant by design: non-object attributes, missing keys, and wrong-typed
    /// values all fold to the harmless default rather than failing — the rollup
    /// must never break on a span that simply did not set a key.
    #[must_use]
    pub fn read(attributes: &Value) -> Self {
        let kind = read_kind(attributes);
        Self {
            kind_present: kind.is_some(),
            status: attributes
                .get(SPAN_STATUS)
                .and_then(Value::as_str)
                .map(SpanStatus::parse)
                .unwrap_or_default(),
            tokens: attributes.get(SPAN_TOKENS).and_then(Value::as_i64),
            cost: attributes.get(SPAN_COST).and_then(Value::as_f64),
        }
    }
}

/// Read the [`SPAN_KIND`] label out of `attributes`, if present and a string.
#[must_use]
pub fn read_kind(attributes: &Value) -> Option<String> {
    attributes
        .get(SPAN_KIND)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// A helper for setting reserved metric keys on a span's attributes consistently.
///
/// Emitters compose their domain attributes as a JSON object, then thread it
/// through this builder so the reserved keys always carry the same shapes
/// (`span.status` is the [`SpanStatus`] wire string, `span.tokens` an integer,
/// `span.cost` a number). Setting a key to its "unset" value (an `Unset` status,
/// a `None` token/cost) is a no-op, so a builder call is always safe even when an
/// emitter only knows some metrics.
#[derive(Debug, Default)]
pub struct MetricsBuilder {
    kind: Option<String>,
    status: SpanStatus,
    tokens: Option<i64>,
    cost: Option<f64>,
}

impl MetricsBuilder {
    /// Start an empty builder — nothing is written until a metric is set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the span kind label.
    #[must_use]
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Set the span status. An [`SpanStatus::Unset`] leaves the key unwritten.
    #[must_use]
    pub fn status(mut self, status: SpanStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the token count.
    #[must_use]
    pub fn tokens(mut self, tokens: i64) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Set the monetary cost.
    #[must_use]
    pub fn cost(mut self, cost: f64) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Write the set reserved keys onto `attributes`, which must be a JSON object.
    ///
    /// Only keys with a non-default value are written, so existing attributes are
    /// left untouched where a metric was not provided. If `attributes` is not an
    /// object it is replaced with one — emitters always pass an object, but this
    /// keeps the helper total.
    pub fn apply(self, attributes: &mut Value) {
        if !attributes.is_object() {
            *attributes = Value::Object(serde_json::Map::new());
        }
        let map = attributes
            .as_object_mut()
            .expect("attributes was just ensured to be an object");
        if let Some(kind) = self.kind {
            map.insert(SPAN_KIND.to_owned(), Value::String(kind));
        }
        if self.status != SpanStatus::Unset {
            map.insert(
                SPAN_STATUS.to_owned(),
                Value::String(self.status.as_str().to_owned()),
            );
        }
        if let Some(tokens) = self.tokens {
            map.insert(SPAN_TOKENS.to_owned(), Value::Number(tokens.into()));
        }
        if let Some(cost) = self.cost
            && let Some(num) = serde_json::Number::from_f64(cost)
        {
            map.insert(SPAN_COST.to_owned(), Value::Number(num));
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{MetricsBuilder, SpanMetrics, SpanStatus};

    #[test]
    fn reads_metrics_folded_out_of_reserved_keys() {
        let attrs = json!({
            "rule": "r",
            "span.kind": "rule",
            "span.status": "error",
            "span.tokens": 1234,
            "span.cost": 0.5,
        });
        let m = SpanMetrics::read(&attrs);
        assert!(m.kind_present);
        assert_eq!(m.status, SpanStatus::Error);
        assert!(m.status.is_error());
        assert_eq!(m.tokens, Some(1234));
        assert_eq!(m.cost, Some(0.5));
    }

    #[test]
    fn missing_and_malformed_keys_fold_to_harmless_defaults() {
        let none = SpanMetrics::read(&json!({ "rule": "r" }));
        assert_eq!(none, SpanMetrics::default());
        assert_eq!(none.status, SpanStatus::Unset);
        assert!(!none.status.is_error());

        // Wrong types must not be read as real values, nor as an error status.
        let bad = SpanMetrics::read(&json!({
            "span.status": 7,
            "span.tokens": "lots",
            "span.cost": "free",
        }));
        assert_eq!(bad, SpanMetrics::default());
    }

    #[test]
    fn non_object_attributes_read_as_default() {
        assert_eq!(SpanMetrics::read(&json!(null)), SpanMetrics::default());
        assert_eq!(SpanMetrics::read(&json!(42)), SpanMetrics::default());
    }

    #[test]
    fn builder_sets_only_provided_keys_and_round_trips() {
        let mut attrs = json!({ "rule": "r" });
        MetricsBuilder::new()
            .kind("rule")
            .status(SpanStatus::Ok)
            .tokens(10)
            .cost(1.25)
            .apply(&mut attrs);

        // Existing attributes are preserved.
        assert_eq!(attrs["rule"], "r");
        let m = SpanMetrics::read(&attrs);
        assert!(m.kind_present);
        assert_eq!(m.status, SpanStatus::Ok);
        assert_eq!(m.tokens, Some(10));
        assert_eq!(m.cost, Some(1.25));
    }

    #[test]
    fn builder_leaves_unset_status_and_none_metrics_unwritten() {
        let mut attrs = json!({ "rule": "r" });
        MetricsBuilder::new().status(SpanStatus::Unset).apply(&mut attrs);
        assert_eq!(attrs.get("span.status"), None);
        assert_eq!(attrs.get("span.tokens"), None);
        assert_eq!(SpanMetrics::read(&attrs), SpanMetrics::default());
    }

    #[test]
    fn status_parse_is_total_and_lossless_on_known_values() {
        assert_eq!(SpanStatus::parse("ok"), SpanStatus::Ok);
        assert_eq!(SpanStatus::parse("error"), SpanStatus::Error);
        assert_eq!(SpanStatus::parse("unset"), SpanStatus::Unset);
        assert_eq!(SpanStatus::parse("garbage"), SpanStatus::Unset);
        for s in [SpanStatus::Ok, SpanStatus::Error, SpanStatus::Unset] {
            assert_eq!(SpanStatus::parse(s.as_str()), s);
        }
    }
}
