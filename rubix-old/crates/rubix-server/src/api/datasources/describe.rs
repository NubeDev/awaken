//! GET /api/v1/datasources/{id}/describe — the tables and columns a datasource
//! exposes, for a human authoring a widget or the AI choosing a named query.
//!
//! Returns the operator-declared schema blob from `datasources.json` when
//! present, else introspects `information_schema` under the read-only role
//! (docs/design/datasources.md "Schema discovery"). Without it the authoring and
//! AI surfaces have nothing to show.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::Value;

use super::registry_or_unavailable;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/datasources/{id}/describe", tag = "datasources",
    params(("id" = String, Path, description = "Registered datasource id")),
    responses(
        (status = 200, description = "Declared or introspected schema: { tables: [...] }"),
        (status = 404, body = ErrorBody),
        (status = 500, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn describe_datasource(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let registry = registry_or_unavailable(&state)?;
    let schema = registry.describe(&id).await?;
    Ok(Json(serde_json::to_value(schema).unwrap_or(Value::Null)))
}
