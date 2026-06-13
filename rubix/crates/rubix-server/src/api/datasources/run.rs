//! POST /api/v1/datasources/{id}/query — run operator-authored native SQL
//! against a registered external datasource and return `{ columns, rows }`.
//!
//! This is the lenient (dashboard/authoring) path: a result that breaches the
//! datasource's caps is truncated and flagged with `breached: true`, not turned
//! into an error. The strict (spark) path lives in the `datasource` board node.
//!
//! The SQL is operator-authored — the same trust tier as a widget definition
//! (docs/design/datasources.md "Query authoring tiers"). The executor still
//! refuses multi-statement input and binds every parameter positionally; values
//! are never spliced into the SQL text.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use rubix_core::SeriesField;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use super::registry_or_unavailable;
use crate::api::UnitsCtx;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct DatasourceQueryRequest {
    /// A single native SQL statement for the external engine (e.g. a TimescaleDB
    /// `time_bucket` aggregate). Multi-statement input is refused.
    pub sql: String,
    /// Positional bound parameters for `$1..$N`, typed and bound — never spliced
    /// into the SQL. Each is `{ "type": "text"|"int"|"float"|"bool"|"timestamp",
    /// "value": ... }` or `{ "type": "null" }`. Omit for a parameterless query.
    #[serde(default)]
    pub params: Vec<Value>,
    /// Optional per-column quantity declarations (WS-11). A column with a
    /// `quantity` is converted at the response edge into the caller's preferred
    /// unit (honouring the `Accept-Units` header); untagged columns pass through
    /// as bare numbers. The unit each tagged column ends up in is reported in
    /// the response `units` map.
    #[serde(default)]
    pub fields: Vec<SeriesField>,
}

/// `{ columns, rows, breached, units }` — the `rubix-query` shape plus a
/// `units` map (`{ column: unit_code }`) naming the unit each *converted* column
/// is now expressed in. `units` is empty when no field was tagged.
#[derive(Debug, Serialize, ToSchema)]
pub struct DatasourceResultBody {
    pub columns: Value,
    pub rows: Value,
    pub breached: bool,
    /// Per-column unit wire code for the columns that were unit-converted.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    #[schema(value_type = Object)]
    pub units: Map<String, Value>,
}

#[utoipa::path(post, path = "/api/v1/datasources/{id}/query", tag = "datasources",
    params(("id" = String, Path, description = "Registered datasource id")),
    request_body = DatasourceQueryRequest,
    responses(
        (status = 200, body = DatasourceResultBody),
        (status = 400, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn run_query(
    State(state): State<AppState>,
    Path(id): Path<String>,
    units: Option<Extension<UnitsCtx>>,
    Json(req): Json<DatasourceQueryRequest>,
) -> Result<Json<DatasourceResultBody>, ApiError> {
    let registry = registry_or_unavailable(&state)?;
    let params = super::parse_params(req.params)?;
    let executor = registry.executor(&id)?;
    let result = executor.execute(&req.sql, &params).await?;
    let mut body = super::result_body(result);
    // Convert any declared quantity columns at the edge. Untagged → passthrough.
    if let Some(Extension(ctx)) = units {
        body.units = convert_columns(&mut body, &req.fields, &ctx)?;
    }
    Ok(Json(body))
}

/// Convert each tagged column's cells in `body.rows` in place, returning the
/// `{ column: unit_code }` map for the columns that converted. A field with no
/// `quantity`, or naming a column the result doesn't have, is skipped (the
/// column passes through). A bad quantity/unit code is a 400.
fn convert_columns(
    body: &mut DatasourceResultBody,
    fields: &[SeriesField],
    ctx: &UnitsCtx,
) -> Result<Map<String, Value>, ApiError> {
    let mut units = Map::new();
    // Map column name → index from the result's `columns` array.
    let col_index = |name: &str| -> Option<usize> {
        body.columns
            .as_array()?
            .iter()
            .position(|c| c.get("name").and_then(Value::as_str) == Some(name))
    };
    let Some(rows) = body.rows.as_array_mut() else {
        return Ok(units);
    };
    for field in fields {
        let Some(quantity) = field.quantity.as_deref() else {
            continue; // untagged column: passthrough
        };
        let Some(idx) = col_index(&field.column) else {
            continue; // column not in this result: nothing to convert
        };
        let mut unit_code: Option<String> = None;
        for row in rows.iter_mut() {
            let Some(cell) = row.as_array_mut().and_then(|r| r.get_mut(idx)) else {
                continue;
            };
            // Only numeric cells convert; null / non-number cells pass through.
            if let Some(value) = cell.as_f64() {
                let (converted, unit) = ctx
                    .convert_field(quantity, field.stored_unit.as_deref(), value)
                    .map_err(ApiError::BadRequest)?;
                *cell = serde_json::json!(converted);
                unit_code = Some(unit);
            }
        }
        if let Some(unit) = unit_code {
            units.insert(field.column.clone(), Value::String(unit));
        }
    }
    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use rubix_prefs::resolver::{resolve, SystemDefaults};

    fn body() -> DatasourceResultBody {
        DatasourceResultBody {
            columns: serde_json::json!([{"name": "ts", "type_name": "int8"},
                                        {"name": "temp", "type_name": "float8"}]),
            rows: serde_json::json!([[1, 32.0], [2, 212.0]]),
            breached: false,
            units: Map::new(),
        }
    }

    fn ctx(mode: crate::api::UnitsMode) -> UnitsCtx {
        UnitsCtx::new(mode, Arc::new(resolve(None, None, &SystemDefaults::starter())))
    }

    #[test]
    fn converts_tagged_column_to_preferred_unit() {
        // Defaults are metric → celsius. A column stored in fahrenheit converts:
        // 32 °F → 0 °C, 212 °F → 100 °C.
        let mut b = body();
        let fields = vec![SeriesField {
            column: "temp".into(),
            quantity: Some("temperature".into()),
            stored_unit: Some("fahrenheit".into()),
        }];
        let units = convert_columns(&mut b, &fields, &ctx(crate::api::UnitsMode::Preferred)).unwrap();
        assert_eq!(units["temp"], "celsius");
        assert!((b.rows[0][1].as_f64().unwrap() - 0.0).abs() < 1e-9);
        assert!((b.rows[1][1].as_f64().unwrap() - 100.0).abs() < 1e-9);
        // The untagged ts column is untouched.
        assert_eq!(b.rows[0][0], 1);
    }

    #[test]
    fn untagged_columns_pass_through() {
        let mut b = body();
        let fields = vec![SeriesField {
            column: "temp".into(),
            quantity: None, // not convertible
            stored_unit: None,
        }];
        let units = convert_columns(&mut b, &fields, &ctx(crate::api::UnitsMode::Preferred)).unwrap();
        assert!(units.is_empty());
        assert_eq!(b.rows[1][1], 212.0); // unchanged
    }

    #[test]
    fn bad_quantity_is_rejected() {
        let mut b = body();
        let fields = vec![SeriesField {
            column: "temp".into(),
            quantity: Some("nonsense".into()),
            stored_unit: None,
        }];
        let err = convert_columns(&mut b, &fields, &ctx(crate::api::UnitsMode::Preferred));
        assert!(matches!(err, Err(ApiError::BadRequest(_))));
    }
}
