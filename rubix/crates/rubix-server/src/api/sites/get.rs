//! GET /api/v1/sites/{id}

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::Site;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/sites/{id}", params(("id" = Uuid, Path)), tag = "sites",
    security(("bearer" = [])),
    responses((status = 200, body = Site), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_site(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Site>, ApiError> {
    let site = blocking(move || Ok(state.store.get_site(id)?)).await?;
    principal.authorize_site_read(&site.org, &site.slug)?;
    Ok(Json(site))
}
