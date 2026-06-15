//! `GET /datasources` — list the declared datasources.
//!
//! The Grafana datasource surface (`rubix/docs/SCOPE.md`, "Datasources"): the set
//! of declared datasources a dashboard can query. Listing is an open read — it
//! reveals only id/label/kind, which the query capability already governs
//! (`rubix-datasource::list`). The native SurrealDB default is always present.
//!
//! Reads the shared registry from [`AppState`], so it reflects every connector
//! registered through `POST /datasources` (and rehydrated at boot), not a
//! throwaway native-only registry.

use axum::Json;
use axum::extract::State;
use rubix_datasource::list;

use crate::auth::Authenticated;
use crate::dto::datasource::DatasourceDto;
use crate::error::ApiResult;
use crate::state::AppState;

/// List the declared datasources visible to any authenticated principal.
pub async fn list_datasources_route(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> ApiResult<Json<Vec<DatasourceDto>>> {
    let registry = state.datasources.read().await;
    let datasources = list(&registry)
        .into_iter()
        .map(|config| DatasourceDto {
            id: config.id().to_owned(),
            label: config.label().to_owned(),
            kind: config.kind().to_owned(),
        })
        .collect();
    Ok(Json(datasources))
}
