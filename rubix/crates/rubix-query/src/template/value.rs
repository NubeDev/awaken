//! A variable's scalar value and its lowering to a safe SQL literal.
//!
//! A dashboard selection is a closed set of scalars — text, number, or boolean.
//! The lowering is the injection boundary (see the module docs): text is single-
//! quoted with every `'` doubled, while a number/boolean is a closed character set
//! emitted bare. A composite JSON value (object or nested array) is **rejected** at
//! construction rather than coerced, so nothing untyped reaches the SQL.

/// One scalar variable value, normalised for lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    /// A string value — lowered as a single-quoted, escaped SQL literal.
    Text(String),
    /// A numeric value, carried as its canonical decimal text (digits/sign/`.`/`e`
    /// only) so it lowers bare without a float round-trip.
    Num(String),
    /// A boolean value — lowered bare as `TRUE`/`FALSE`.
    Bool(bool),
}

impl Scalar {
    /// Normalise one JSON scalar, or `None` for null / a composite value.
    ///
    /// A JSON number's own string form is reused verbatim (it is already a closed
    /// character set), so an integer stays an integer and a decimal keeps its
    /// digits without a lossy `f64` hop.
    #[must_use]
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::String(text) => Some(Self::Text(text.clone())),
            serde_json::Value::Number(number) => Some(Self::Num(number.to_string())),
            serde_json::Value::Bool(flag) => Some(Self::Bool(*flag)),
            serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                None
            }
        }
    }

    /// The value as a typed SQL literal: text quoted, number/bool bare.
    #[must_use]
    pub fn to_literal(&self) -> String {
        match self {
            Self::Text(text) => quote(text),
            Self::Num(number) => number.clone(),
            Self::Bool(true) => "TRUE".to_owned(),
            Self::Bool(false) => "FALSE".to_owned(),
        }
    }

    /// The value force-quoted as a string literal, whatever its kind — backs the
    /// `${name:singlequote}` modifier where every member is wanted as a string.
    #[must_use]
    pub fn to_quoted(&self) -> String {
        match self {
            Self::Text(text) => quote(text),
            Self::Num(number) => quote(number),
            Self::Bool(true) => quote("true"),
            Self::Bool(false) => quote("false"),
        }
    }
}

/// Render `text` as a single-quoted SQL literal, doubling every embedded quote.
///
/// Standard-SQL string literals (DataFusion's dialect) have no backslash escape,
/// so doubling `'` is the complete and only escape needed: the value can never
/// terminate its own literal.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::Scalar;
    use serde_json::json;

    #[test]
    fn json_scalars_normalise() {
        assert_eq!(Scalar::from_json(&json!("hi")), Some(Scalar::Text("hi".to_owned())));
        assert_eq!(Scalar::from_json(&json!(42)), Some(Scalar::Num("42".to_owned())));
        assert_eq!(Scalar::from_json(&json!(1.5)), Some(Scalar::Num("1.5".to_owned())));
        assert_eq!(Scalar::from_json(&json!(true)), Some(Scalar::Bool(true)));
    }

    #[test]
    fn composite_and_null_json_is_rejected() {
        assert_eq!(Scalar::from_json(&json!(null)), None);
        assert_eq!(Scalar::from_json(&json!([1, 2])), None);
        assert_eq!(Scalar::from_json(&json!({"a": 1})), None);
    }

    #[test]
    fn literals_are_typed() {
        assert_eq!(Scalar::Text("hq".to_owned()).to_literal(), "'hq'");
        assert_eq!(Scalar::Num("42".to_owned()).to_literal(), "42");
        assert_eq!(Scalar::Bool(true).to_literal(), "TRUE");
    }

    #[test]
    fn singlequote_forces_strings() {
        assert_eq!(Scalar::Num("42".to_owned()).to_quoted(), "'42'");
        assert_eq!(Scalar::Bool(false).to_quoted(), "'false'");
    }

    #[test]
    fn a_quote_is_doubled() {
        assert_eq!(Scalar::Text("o'brien".to_owned()).to_literal(), "'o''brien'");
    }
}
