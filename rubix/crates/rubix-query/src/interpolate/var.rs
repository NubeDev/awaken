//! The caller-supplied variable input: a name and one or more values.
//!
//! This is the wire/contract type both query paths accept (a `variables` field
//! on their request DTOs, see docs/design/variables-and-templating.md §2). A
//! variable carries a name (referenced as `$name` / `${name}` in the SQL) and a
//! value that is either single or multi. Values arrive as JSON scalars and are
//! lowered into bound parameters — never spliced into the SQL text.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One caller-supplied variable: a `name` and its selected value(s).
///
/// `value` accepts either a single JSON scalar (`"Site-A"`, `7`, `true`) or an
/// array of scalars (a multi-select). A multi-value variable is what
/// `${name:csv}`, `${name:singlequote}`, and `$__sqlIn(name)` expand; a single
/// value is what `$name` / `${name}` substitute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryVariable {
    /// The variable name as referenced in SQL, without the leading `$`.
    pub name: String,
    /// The selected value(s). A bare scalar is one value; an array is many.
    pub value: VarValue,
}

/// A variable's value: a single scalar or a list of scalars.
///
/// Untagged so the wire shape stays ergonomic: `"Site-A"` deserialises to
/// [`VarValue::One`], `["Site-A", "Site-B"]` to [`VarValue::Many`]. Each scalar
/// is a [`Scalar`]; nested arrays/objects are rejected at parse time because a
/// bound parameter is a scalar, not a structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VarValue {
    /// A single scalar value.
    One(Scalar),
    /// An ordered list of scalar values (a multi-select).
    Many(Vec<Scalar>),
}

/// A JSON scalar a variable can carry. Mirrors the bound-parameter kinds the
/// backends accept; structured JSON (arrays/objects) is not a scalar and is
/// refused so it can never reach SQL as anything but a literal value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    /// SQL NULL.
    Null,
    /// A boolean.
    Bool(bool),
    /// A signed integer.
    Int(i64),
    /// A double-precision float.
    Float(f64),
    /// A text value, always bound, never interpolated.
    Text(String),
}

impl VarValue {
    /// The scalars this value contributes, in order. A single value yields one
    /// scalar; a multi value yields each in turn. Used by the multi-expansion
    /// tokens (`:csv`, `:singlequote`, `$__sqlIn`).
    pub fn scalars(&self) -> &[Scalar] {
        match self {
            VarValue::One(s) => std::slice::from_ref(s),
            VarValue::Many(values) => values,
        }
    }
}

impl Scalar {
    /// Build a scalar from an arbitrary JSON value, rejecting non-scalars.
    pub fn from_json(value: &Value) -> Option<Scalar> {
        match value {
            Value::Null => Some(Scalar::Null),
            Value::Bool(b) => Some(Scalar::Bool(*b)),
            Value::Number(n) => n
                .as_i64()
                .map(Scalar::Int)
                .or_else(|| n.as_f64().map(Scalar::Float)),
            Value::String(s) => Some(Scalar::Text(s.clone())),
            Value::Array(_) | Value::Object(_) => None,
        }
    }
}
