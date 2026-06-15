//! Post-read, per-caller unit conversion for query rows (§2).
//!
//! The query cache holds **raw canonical (metric) values** so it never has to key
//! on the unit system (`rubix/docs/design/DASHBOARDS-SCOPE.md` §2/§4a). Conversion
//! is therefore a strictly post-cache, per-caller layer: once a statement's rows
//! are read, each numeric column the chart declared as a physical [`Quantity`] is
//! converted to the **requesting principal's** unit system, leaving every other
//! column untouched. A metric preference is a pass-through, so the common case
//! pays nothing.

use std::collections::HashMap;

use rubix_prefs::{Quantity, UnitSystem, convert_json};
use serde_json::Value;

/// Convert the declared quantity columns of `rows` into `system`, in place.
///
/// `quantities` maps a column name to its physical quantity string (`temperature`
/// /`length`/`mass`/`speed`). For each row, a column present in the map and
/// holding a finite number is converted from canonical metric to `system`; a
/// column absent from a row, a non-numeric value, or an unknown quantity string is
/// left as-is (the conversion is best-effort display, never a hard failure on one
/// odd row). With a metric `system` this is a no-op.
pub fn convert_rows(rows: &mut [Value], quantities: &HashMap<String, String>, system: UnitSystem) {
    if system == UnitSystem::Metric || quantities.is_empty() {
        return;
    }
    // Resolve the quantity strings once, dropping any unknown names.
    let resolved: Vec<(&String, Quantity)> = quantities
        .iter()
        .filter_map(|(col, q)| Quantity::parse(q).map(|q| (col, q)))
        .collect();
    if resolved.is_empty() {
        return;
    }
    for row in rows {
        let Some(object) = row.as_object_mut() else {
            continue;
        };
        for (col, quantity) in &resolved {
            if let Some(value) = object.get(col.as_str()) {
                if let Ok(converted) = convert_json(value, *quantity, system) {
                    object.insert((*col).clone(), converted);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rubix_prefs::UnitSystem;
    use serde_json::json;

    use super::convert_rows;

    fn temp_map() -> HashMap<String, String> {
        HashMap::from([("temp".to_owned(), "temperature".to_owned())])
    }

    #[test]
    fn imperial_converts_the_declared_column_only() {
        let mut rows = vec![json!({ "temp": 100.0, "other": 100.0 })];
        convert_rows(&mut rows, &temp_map(), UnitSystem::Imperial);
        assert!((rows[0]["temp"].as_f64().unwrap() - 212.0).abs() < 1e-9);
        assert_eq!(rows[0]["other"], json!(100.0), "undeclared column untouched");
    }

    #[test]
    fn metric_is_a_passthrough() {
        let mut rows = vec![json!({ "temp": 100.0 })];
        convert_rows(&mut rows, &temp_map(), UnitSystem::Metric);
        assert_eq!(rows[0]["temp"], json!(100.0));
    }

    #[test]
    fn a_non_numeric_or_missing_column_is_left_alone() {
        let mut rows = vec![json!({ "temp": "warm" }), json!({ "label": "x" })];
        convert_rows(&mut rows, &temp_map(), UnitSystem::Imperial);
        assert_eq!(rows[0]["temp"], json!("warm"));
        assert_eq!(rows[1], json!({ "label": "x" }));
    }

    #[test]
    fn an_unknown_quantity_string_is_ignored() {
        let mut rows = vec![json!({ "temp": 100.0 })];
        let map = HashMap::from([("temp".to_owned(), "luminance".to_owned())]);
        convert_rows(&mut rows, &map, UnitSystem::Imperial);
        assert_eq!(rows[0]["temp"], json!(100.0), "unknown quantity left raw");
    }
}
