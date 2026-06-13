//! Typed bound parameters for a datasource read.
//!
//! Bindings never textually splice values into SQL (docs/design/datasources.md
//! "Parameterization"): a binding declares native SQL with `$1`-style
//! placeholders and a typed parameter list, and the executor hands these values
//! to the driver as bound parameters separate from the SQL text. This enum is
//! the typed value carried to the backend; the backend is responsible for
//! binding each in positional order (see [`crate::backend`]).

use serde::{Deserialize, Serialize};

/// One bound parameter value. The set is intentionally small — the kinds a
/// dashboard time-range picker or site filter produce. A `Null` carries no type
/// and binds as SQL NULL; the external engine coerces against the column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Param {
    /// SQL NULL.
    Null,
    /// A boolean.
    Bool(bool),
    /// A 64-bit signed integer.
    Int(i64),
    /// A double-precision float.
    Float(f64),
    /// A text value, bound (never interpolated).
    Text(String),
    /// An RFC 3339 / ISO 8601 timestamp string, bound as text for the engine to
    /// cast (a dashboard time-range bound). Kept as a distinct variant so a
    /// future backend can bind it as a native timestamp without a schema change.
    Timestamp(String),
}

/// An ordered list of bound parameters, positional against `$1..$N`.
pub type Params = Vec<Param>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let params = vec![
            Param::Null,
            Param::Bool(true),
            Param::Int(-7),
            Param::Float(1.5),
            Param::Text("site-a".into()),
            Param::Timestamp("2026-06-13T00:00:00Z".into()),
        ];
        let json = serde_json::to_string(&params).unwrap();
        let back: Params = serde_json::from_str(&json).unwrap();
        assert_eq!(params, back);
    }

    #[test]
    fn text_param_carries_value_verbatim() {
        let p = Param::Text("'; DROP TABLE t; --".into());
        // The dangerous string is just data; it is bound, never spliced.
        assert!(matches!(p, Param::Text(s) if s.contains("DROP")));
    }
}
