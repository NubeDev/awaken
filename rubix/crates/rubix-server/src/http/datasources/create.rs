//! `POST /datasources` — register an external datasource connector.
//!
//! Adding a datasource is a cross-plane action SurrealDB's permission engine does
//! not see, so it is gated on the WS-04 `datasource-register` capability
//! (`rubix/docs/SCOPE.md`, "Datasources"; contract #2), checked fail-closed inside
//! `rubix_datasource::register`. The handler builds the live connector from the
//! posted declaration, registers it into the shared registry (materialising its
//! providers — the connect reaches the backend), then persists the declaration so
//! it survives a restart. A duplicate id is `409`; an unsupported kind `400`; a
//! denied principal `403`; an unreachable backend `502`.

use axum::Json;
use axum::extract::State;
use rubix_datasource::DatasourceError;

use crate::auth::Authenticated;
use crate::datasources::{ControlError, StoredDatasource, build_and_register, save};
use crate::dto::{DatasourceDto, RegisterDatasourceRequest};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Register the posted datasource, persist it, and return its declared identity.
pub async fn create_datasource_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<RegisterDatasourceRequest>,
) -> ApiResult<Json<DatasourceDto>> {
    let decl = StoredDatasource {
        id: body.id,
        label: body.label,
        kind: body.kind,
        connection_string: body.connection_string,
        tables: body.tables,
    };

    // Register into the shared registry under a write lock — the connect and the
    // duplicate check run while no query reads the registry.
    {
        let mut registry = state.datasources.write().await;
        build_and_register(&mut registry, state.store.raw(), &auth.principal, &decl)
            .await
            .map_err(map_control_error)?;
    }

    // The connector is live; persist the declaration so a restart rehydrates it.
    // A persistence failure after a successful register would leave the running
    // registry ahead of the store, so surface it rather than swallowing.
    save(state.store.raw(), &decl)
        .await
        .map_err(ApiError::Internal)?;

    Ok(Json(DatasourceDto {
        id: decl.id,
        label: decl.label,
        kind: decl.kind,
    }))
}

/// Map a control-plane failure to its transport status.
///
/// A denied capability is `403`, a duplicate id `409`, an unknown/unbuilt kind
/// `400`, a failed connect to the backend `502` (the request was well-formed; the
/// upstream datasource is the problem), and anything else `500`.
pub(crate) fn map_control_error(error: ControlError) -> ApiError {
    match error {
        ControlError::UnsupportedKind(kind) => {
            ApiError::BadRequest(format!("unsupported datasource kind `{kind}`"))
        }
        ControlError::Datasource(DatasourceError::Denied) => {
            ApiError::Forbidden("principal lacks the datasource-register capability".to_owned())
        }
        ControlError::Datasource(DatasourceError::Duplicate(id)) => {
            ApiError::Conflict(format!("a datasource is already registered under id `{id}`"))
        }
        ControlError::Datasource(DatasourceError::Connect { id, reason }) => {
            ApiError::BadGateway(format!("connector `{id}` could not reach its backend: {reason}"))
        }
        ControlError::Datasource(DatasourceError::Unknown(_)) => ApiError::NotFound,
        other => ApiError::Internal(other.to_string()),
    }
}
