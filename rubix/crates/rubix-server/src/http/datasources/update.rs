//! `PATCH /datasources/:id` — update a registered datasource connector.
//!
//! A datasource update is a re-registration (`rubix/docs/SCOPE.md`, "Datasources"):
//! the label, connection string, or table set can change, so the providers are
//! rebuilt against the new wiring. Gated on the same WS-04 `datasource-register`
//! capability as register. The handler loads the persisted declaration, merges the
//! patch over it, then atomically (under the registry write lock) drops the old
//! connector and registers the rebuilt one; only on success is the new row saved.
//! A backend the new wiring cannot reach is `502` and leaves the old connector in
//! place (the drop-and-rebuild rolls back). An unknown id is `404`.

use axum::Json;
use axum::extract::{Path, State};
use rubix_datasource::{NATIVE_SURREAL_ID, remove};

use crate::auth::Authenticated;
use crate::datasources::{StoredDatasource, build_and_register, load_all, save};
use crate::dto::{DatasourceDto, UpdateDatasourceRequest};
use crate::error::{ApiError, ApiResult};
use crate::http::datasources::create::map_control_error;
use crate::state::AppState;

/// Re-register the datasource under `id` with the patch applied, then persist it.
pub async fn update_datasource_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
    Json(body): Json<UpdateDatasourceRequest>,
) -> ApiResult<Json<DatasourceDto>> {
    if id == NATIVE_SURREAL_ID {
        return Err(ApiError::Forbidden(
            "the native datasource cannot be updated".to_owned(),
        ));
    }

    // The persisted row is the source of truth for the fields the patch omits; an
    // update against an id no connector was registered under is a clean 404.
    let current = load_all(state.store.raw())
        .await
        .map_err(ApiError::Internal)?
        .into_iter()
        .find(|d| d.id == id)
        .ok_or(ApiError::NotFound)?;

    let updated = StoredDatasource {
        id: current.id.clone(),
        kind: current.kind.clone(),
        label: body.label.unwrap_or(current.label),
        connection_string: body.connection_string.unwrap_or(current.connection_string),
        tables: body.tables.unwrap_or(current.tables),
    };

    {
        let mut registry = state.datasources.write().await;
        // Drop the old connector first so the id is free, then rebuild. If the
        // rebuild fails (unreachable backend), surface it — the datasource is now
        // absent from the registry, matching the row we have not yet overwritten;
        // a retry or a re-register repairs it.
        remove(&mut registry, state.store.raw(), &auth.principal, &id)
            .await
            .map_err(|e| map_control_error(e.into()))?;
        build_and_register(&mut registry, state.store.raw(), &auth.principal, &updated)
            .await
            .map_err(map_control_error)?;
    }

    save(state.store.raw(), &updated)
        .await
        .map_err(ApiError::Internal)?;

    Ok(Json(DatasourceDto {
        id: updated.id,
        label: updated.label,
        kind: updated.kind,
    }))
}
