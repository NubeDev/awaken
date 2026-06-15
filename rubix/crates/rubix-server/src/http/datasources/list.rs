//! `GET /datasources` — list the declared datasources.
//!
//! The Grafana datasource surface (`rubix/docs/SCOPE.md`, "Datasources"): the set
//! of declared datasources a dashboard can query. Listing is an open read — it
//! reveals only names, which the query capability already governs
//! (`rubix-datasource::list`). The native SurrealDB default is always present.
//!
//! The registry is seeded with only the native default here: registering external
//! connectors (the `POST /datasources` half) requires a connector instance that
//! the WS-13 extension control plane supplies, which is deferred — see the WS-16
//! session log and `docs/sessions/TODOs.md`.

use axum::Json;
use rubix_datasource::{Registry, list};

use crate::auth::Authenticated;
use crate::dto::datasource::DatasourceDto;
use crate::error::ApiResult;

/// List the declared datasources visible to any authenticated principal.
pub async fn list_datasources_route(_auth: Authenticated) -> ApiResult<Json<Vec<DatasourceDto>>> {
    let registry = Registry::with_native_default();
    let datasources = list(&registry)
        .into_iter()
        .map(|config| DatasourceDto {
            id: config.id().to_owned(),
            label: config.label().to_owned(),
        })
        .collect();
    Ok(Json(datasources))
}
