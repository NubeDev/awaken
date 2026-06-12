//! DELETE /api/v1/sites/{id} — cascades to equips, points, history, sparks.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/sites/{id}", params(("id" = Uuid, Path)), tag = "sites",
    security(("bearer" = [])),
    responses((status = 204), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_site(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let store = state.store.clone();
    let site = blocking(move || Ok(store.get_site(id)?)).await?;
    principal.authorize_site_write(&site.org, &site.slug)?;
    blocking(move || Ok(state.store.delete_site(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
