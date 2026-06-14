//! The parameter map a caller hands to [`crate::run_rule`].
//!
//! [`run_rule`] takes a `rhai::Map` (the `params` variable a script reads). An
//! integrating crate holds parameters as JSON (a board node's config, an HTTP
//! request body), so this module is the bridge: [`params_from_json`] converts a
//! JSON object into the `Map` `run_rule` expects, and [`Params`] re-exports the
//! map type so the caller need not depend on `rhai` directly. This keeps the
//! crate standalone while giving integrators a dependency-free way to build the
//! one non-`std` argument the entry point requires.
//!
//! [`run_rule`]: crate::run_rule

use rhai::{Dynamic, Map};
use serde_json::Value;

/// The parameter map exposed to a rule script as its `params` variable.
pub type Params = Map;

/// Build a [`Params`] map from a JSON object.
///
/// Object keys become map keys; values convert by JSON kind — numbers to
/// integer or float (matching how a script reads `params.limit`), strings,
/// bools, null to `()`, and nested arrays/objects recursively. A non-object
/// JSON value yields an empty map, so a board node with no `params` config runs
/// with an empty parameter set rather than failing.
pub fn params_from_json(value: &Value) -> Params {
    match value {
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| (k.as_str().into(), json_to_dynamic(v)))
            .collect(),
        _ => Map::new(),
    }
}

fn json_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else {
                Dynamic::from(n.as_f64().unwrap_or(f64::NAN))
            }
        }
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Array(items) => {
            let arr: rhai::Array = items.iter().map(json_to_dynamic).collect();
            Dynamic::from(arr)
        }
        Value::Object(_) => Dynamic::from_map(params_from_json(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_keys_become_params() {
        let json = serde_json::json!({ "limit": 25.0, "name": "ahu" });
        let p = params_from_json(&json);
        assert_eq!(p.len(), 2);
        assert!(p.contains_key("limit"));
        assert!(p.contains_key("name"));
    }

    #[test]
    fn integer_stays_integer() {
        let p = params_from_json(&serde_json::json!({ "n": 3 }));
        assert!(p.get("n").unwrap().is_int());
    }

    #[test]
    fn float_stays_float() {
        let p = params_from_json(&serde_json::json!({ "n": 3.5 }));
        assert!(p.get("n").unwrap().is_float());
    }

    #[test]
    fn non_object_yields_empty_map() {
        assert!(params_from_json(&serde_json::json!([1, 2, 3])).is_empty());
        assert!(params_from_json(&serde_json::Value::Null).is_empty());
    }

    #[test]
    fn nested_object_and_array_convert() {
        let json = serde_json::json!({ "tags": ["a", "b"], "win": { "every": "1h" } });
        let p = params_from_json(&json);
        assert!(p.get("tags").unwrap().is_array());
        assert!(p.get("win").unwrap().is_map());
    }
}
