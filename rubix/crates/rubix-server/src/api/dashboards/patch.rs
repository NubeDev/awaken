//! PATCH /api/v1/dashboards/{id} — edit the title. `org`/`site_id`/`slug` are
//! immutable identity (a rescope is delete-and-recreate).

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::{validate_variables, Dashboard, Variable};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::authorize_dashboard_write_existing;
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchDashboard {
    #[serde(default)]
    pub title: Option<String>,
    /// Replace the dashboard's variables wholesale (the editor owns the full
    /// list). Omit to leave them unchanged; send `[]` to clear them.
    #[serde(default)]
    pub variables: Option<Vec<Variable>>,
}

#[utoipa::path(patch, path = "/api/v1/dashboards/{id}", params(("id" = Uuid, Path)),
    request_body = PatchDashboard, tag = "dashboards", security(("bearer" = [])),
    responses((status = 200, body = Dashboard), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_dashboard(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchDashboard>,
) -> Result<Json<Dashboard>, ApiError> {
    if let Some(variables) = &req.variables {
        validate_variables(variables).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    }
    let store = state.store.clone();
    let current = blocking(move || Ok(store.get_dashboard(id)?)).await?;
    authorize_dashboard_write_existing(
        &principal,
        &state.store,
        &current.org,
        current.site_id,
        id,
    )?;
    let updated = blocking(move || {
        Ok(state
            .store
            .update_dashboard(id, req.title.as_deref(), req.variables.as_deref())?)
    })
    .await?;
    Ok(Json(updated))
}
