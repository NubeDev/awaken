//! A `json_get` scalar UDF so queries can reach into the `content` JSON text.
//!
//! The canonical tables expose `content` as a JSON *string* (the free-form
//! document), deliberately un-flattened so no domain shape is baked into the
//! schema (`schema.rs`). Without a function to read it, only the structural
//! columns (`id`/`namespace`/`created`/`updated`) are queryable. This UDF adds
//! the missing reach: `json_get(json, key)` returns the value at a top-level key
//! as text.
//!
//! It is intentionally **composable rather than path-based**: an object or array
//! value is returned re-serialized as JSON, so nesting reaches deeper —
//! `json_get(json_get(content, 'content'), 'kind')` walks two levels. Scalars
//! come back as their text (`42`, `true`); an absent key or non-object input
//! yields NULL, never an error, so a query over mixed documents never fails on a
//! row that simply lacks the field.

use std::sync::Arc;

use datafusion::arrow::array::{Array, ArrayRef, StringArray};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ColumnarValue, ScalarUDF, Volatility, create_udf};

/// Build the `json_get(json, key) -> text` scalar UDF.
#[must_use]
pub(crate) fn json_get_udf() -> ScalarUDF {
    create_udf(
        "json_get",
        vec![DataType::Utf8, DataType::Utf8],
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(json_get_impl),
    )
}

/// Extract `key` from each JSON string in the first argument.
fn json_get_impl(args: &[ColumnarValue]) -> Result<ColumnarValue, DataFusionError> {
    // Broadcast any scalar argument (the key is typically a literal) to a full
    // array so both inputs are indexed row-by-row.
    let arrays = ColumnarValue::values_to_arrays(args)?;
    let json = as_strings(&arrays[0])?;
    let keys = as_strings(&arrays[1])?;

    let out: StringArray = (0..json.len())
        .map(|i| {
            if json.is_null(i) || keys.is_null(i) {
                return None;
            }
            extract(json.value(i), keys.value(i))
        })
        .collect();

    Ok(ColumnarValue::Array(Arc::new(out)))
}

/// Downcast an array to `StringArray`, erroring if it is not Utf8.
fn as_strings(array: &ArrayRef) -> Result<StringArray, DataFusionError> {
    array
        .as_any()
        .downcast_ref::<StringArray>()
        .cloned()
        .ok_or_else(|| DataFusionError::Execution("json_get expects Utf8 arguments".to_owned()))
}

/// Pull `key` out of one JSON document, rendering the value as text.
///
/// A string value is returned bare; a number or bool as its literal; an object
/// or array re-serialized as JSON (so a further `json_get` can descend). A
/// missing key, JSON null, or unparseable input is `None` (SQL NULL).
fn extract(json: &str, key: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    match value.get(key)? {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::extract;

    #[test]
    fn reads_a_top_level_string() {
        assert_eq!(extract(r#"{"action":"create"}"#, "action"), Some("create".to_owned()));
    }

    #[test]
    fn renders_scalars_as_their_literal_text() {
        assert_eq!(extract(r#"{"value":32.7}"#, "value"), Some("32.7".to_owned()));
        assert_eq!(extract(r#"{"fired":true}"#, "fired"), Some("true".to_owned()));
    }

    #[test]
    fn returns_objects_reserialized_so_they_nest() {
        // The record case: the document payload sits under `content`, and a
        // second json_get descends into it.
        let row = r#"{"content":{"kind":"board"},"namespace":"acme"}"#;
        let inner = extract(row, "content").expect("content present");
        assert_eq!(extract(&inner, "kind"), Some("board".to_owned()));
    }

    #[test]
    fn missing_key_null_and_bad_json_are_none() {
        assert_eq!(extract(r#"{"a":1}"#, "b"), None);
        assert_eq!(extract(r#"{"a":null}"#, "a"), None);
        assert_eq!(extract("not json", "a"), None);
    }
}
