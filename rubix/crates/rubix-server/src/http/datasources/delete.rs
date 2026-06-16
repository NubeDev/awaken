//! `DELETE /datasources/:id` — deregister an external datasource connector.
//!
//! The inverse of register (`rubix/docs/SCOPE.md`, "Datasources"), gated on the
//! same WS-04 `datasource-register` capability checked fail-closed inside
//! `rubix_datasource::remove`. The handler drops the connector from the shared
//! registry (freeing its providers) and forgets its persisted row so a restart
//! does not resurrect it. The reserved native SurrealDB id cannot be removed
//! (`403`); an id no connector is registered under is `404`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use rubix_datasource::{DatasourceError, remove};

use crate::auth::Authenticated;
use crate::datasources::forget;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Deregister the datasource under `id`, then forget its persisted declaration.
pub async fn delete_datasource_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    {
        let mut registry = state.datasources.write().await;
        remove(&mut registry, state.store.raw(), &auth.principal, &id)
            .await
            .map_err(map_remove_error)?;
    }

    // The connector is gone from the live registry; forget its row so a restart
    // does not rehydrate it. A persistence miss here would resurrect it on boot.
    forget(state.store.raw(), &id)
        .await
        .map_err(ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Map a remove failure: a denied capability or a native-id removal is `403`, an
/// unknown id `404`, anything else `500`.
fn map_remove_error(error: DatasourceError) -> ApiError {
    match error {
        DatasourceError::Denied => ApiError::Forbidden("cannot remove this datasource".to_owned()),
        DatasourceError::Unknown(_) => ApiError::NotFound,
        other => ApiError::Internal(other.to_string()),
    }
}
