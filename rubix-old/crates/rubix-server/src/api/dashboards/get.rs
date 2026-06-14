//! GET /api/v1/dashboards/{id}

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::Dashboard;
use uuid::Uuid;

use super::may_read_dashboard;
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/dashboards/{id}", params(("id" = Uuid, Path)),
    tag = "dashboards", security(("bearer" = [])),
    responses((status = 200, body = Dashboard), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_dashboard(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Dashboard>, ApiError> {
    let store = state.store.clone();
    let dashboard = blocking(move || Ok(store.get_dashboard(id)?)).await?;
    if !may_read_dashboard(&principal, &state.store, &dashboard) {
        return Err(ApiError::NotFound("dashboard"));
    }
    Ok(Json(dashboard))
}
