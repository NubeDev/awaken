//! The backend-neutral bound parameter the engine emits.
//!
//! Lowering produces SQL text with `$N` positional placeholders plus an ordered
//! list of [`BoundParam`]. This type is intentionally backend-agnostic: the
//! DataFusion `/query` path maps each to a `ScalarValue`, the datasource
//! `/query` path maps each to a `rubix_datasource::Param`. The engine never
//! knows (or needs to know) which backend will bind them — its single
//! responsibility is to guarantee every value leaves as a bound parameter,
//! never as spliced SQL text (docs/design/variables-and-templating.md §2, the
//! injection boundary).

use super::var::Scalar;

/// One value bound positionally against the `$N` placeholder the engine emitted
/// for it. The set mirrors [`Scalar`]; the two query backends each translate it
/// into their native parameter type.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundParam {
    /// SQL NULL.
    Null,
    /// A boolean.
    Bool(bool),
    /// A signed integer.
    Int(i64),
    /// A double-precision float.
    Float(f64),
    /// A text value.
    Text(String),
    /// An RFC 3339 timestamp, bound as a temporal value rather than plain text.
    ///
    /// Emitted only by the time macros (`$__from`/`$__to`/`$__timeFilter`/
    /// `$__timeGroup`), never by a [`Scalar`] variable — the `From<&Scalar>`
    /// impl below stays total without it (docs/design/time-range-and-refresh.md
    /// §4). The two query backends bind it to their temporal parameter type so a
    /// range bound compares as an instant, not a string.
    Timestamp(String),
}

impl From<&Scalar> for BoundParam {
    fn from(scalar: &Scalar) -> Self {
        match scalar {
            Scalar::Null => BoundParam::Null,
            Scalar::Bool(b) => BoundParam::Bool(*b),
            Scalar::Int(i) => BoundParam::Int(*i),
            Scalar::Float(f) => BoundParam::Float(*f),
            Scalar::Text(s) => BoundParam::Text(s.clone()),
        }
    }
}

/// SQL text and its ordered bound parameters, the lowering result.
///
/// `sql` carries `$1..$N` placeholders; `params[i]` binds against `$（i+1)`.
/// Handed verbatim to whichever backend executes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Lowered {
    /// The rewritten SQL with `$N` placeholders in place of every variable token.
    pub sql: String,
    /// The bound parameters, positional against `$1..$N`.
    pub params: Vec<BoundParam>,
}
