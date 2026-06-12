use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A point value on the wire and in stores.
///
/// Untagged on the JSON side so writes read naturally:
/// `{"value": 21.5}`, `{"value": true}`, `{"value": "occupied"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum PointValue {
    Bool(bool),
    Number(f64),
    Str(String),
}

impl PointValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            PointValue::Number(n) => Some(*n),
            PointValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            PointValue::Str(_) => None,
        }
    }
}

impl std::fmt::Display for PointValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PointValue::Bool(b) => write!(f, "{b}"),
            PointValue::Number(n) => write!(f, "{n}"),
            PointValue::Str(s) => write!(f, "{s}"),
        }
    }
}
