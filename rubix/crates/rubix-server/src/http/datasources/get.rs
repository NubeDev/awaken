//! `GET /datasources/:id` — read one declared datasource's identity.
//!
//! The single-datasource read behind the Grafana datasource surface
//! (`rubix/docs/SCOPE.md`, "Datasources"). Like the list route it is an open read
//! over the shared registry: it returns only id/label/kind, never the connection
//! secret. An id no connector is registered under is `404`.

use axum::Json;
use axum::extract::{Path, State};
use rubix_datasource::find;

use crate::auth::Authenticated;
use crate::dto::datasource::DatasourceDto;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Return the declared identity of the datasource registered under `id`.
pub async fn get_datasource_route(
    State(state): State<AppState>,
    _auth: Authenticated,
    Path(id): Path<String>,
) -> ApiResult<Json<DatasourceDto>> {
    let registry = state.datasources.read().await;
    let config = find(&registry, &id).map_err(|_| ApiError::NotFound)?;
    Ok(Json(DatasourceDto {
        id: config.id().to_owned(),
        label: config.label().to_owned(),
        kind: config.kind().to_owned(),
    }))
}
