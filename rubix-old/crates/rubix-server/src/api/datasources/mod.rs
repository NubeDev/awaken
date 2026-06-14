//! External datasource routes — read-only native SQL against a registered
//! external database (TimescaleDB/Postgres historian). Wiring only.
//!
//! These are the dashboard/authoring surfaces over the datasource executor: the
//! *lenient* path (a cap breach truncates and is reported via `breached`, not an
//! error — docs/design/datasources.md "Refresh cost"). The spark path's strict
//! breach handling lives in the `datasource` board node, not here.
//!
//! All three routes return 503 when no datasource manifest is loaded, so the
//! surface is simply absent on a deployment that declares none.

pub(crate) mod describe;
pub(crate) mod named;
pub(crate) mod run;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use serde_json::Value;

use rubix_datasource::{DatasourceRegistry, Param, Params, ResultSet};

use crate::error::ApiError;
use crate::AppState;

use run::DatasourceResultBody;

/// Borrow the loaded datasource registry, or 503 when none is configured. The
/// whole datasource surface is absent on a deployment that declares no
/// `datasources.json`, rather than 404-ing per id.
fn registry_or_unavailable(state: &AppState) -> Result<&Arc<DatasourceRegistry>, ApiError> {
    state
        .datasources
        .as_ref()
        .ok_or(ApiError::Unavailable("no datasources configured"))
}

/// Parse the wire parameter list (`[{type,value}, …]`) into typed [`Params`]. A
/// malformed parameter is a 400 the caller can correct, never a 500.
fn parse_params(raw: Vec<Value>) -> Result<Params, ApiError> {
    raw.into_iter()
        .map(|v| {
            serde_json::from_value::<Param>(v)
                .map_err(|e| ApiError::BadRequest(format!("invalid datasource parameter: {e}")))
        })
        .collect()
}

/// Project a [`ResultSet`] onto the JSON response body. `columns`/`rows` are
/// already `serde_json`-shaped, so this is a structural move, not a re-encode.
fn result_body(result: ResultSet) -> DatasourceResultBody {
    DatasourceResultBody {
        columns: serde_json::to_value(result.columns).unwrap_or(Value::Null),
        rows: serde_json::to_value(result.rows).unwrap_or(Value::Null),
        breached: result.breached,
        units: serde_json::Map::new(),
    }
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/datasources/{id}/query", post(run::run_query))
        .route(
            "/api/v1/datasources/{id}/named/{name}",
            post(named::invoke_named),
        )
        .route(
            "/api/v1/datasources/{id}/describe",
            get(describe::describe_datasource),
        )
}
