//! DELETE /api/v1/dashboards/{id} — removes the board and cascades to its tiles.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use super::authorize_dashboard_write;
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/dashboards/{id}", params(("id" = Uuid, Path)),
    tag = "dashboards", security(("bearer" = [])),
    responses((status = 204), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_dashboard(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let store = state.store.clone();
    let current = blocking(move || Ok(store.get_dashboard(id)?)).await?;
    authorize_dashboard_write(&principal, &state.store, &current.org, current.site_id)?;
    blocking(move || Ok(state.store.delete_dashboard(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
