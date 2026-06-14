//! GET /api/v1/tokens — list issued tokens (secret hashes never echoed). Only
//! tokens within the caller's own scope are returned, so a team admin sees its
//! team's tokens but not another's.

use axum::extract::State;
use axum::Json;

use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, TokenRecord};
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/tokens", tag = "tokens",
    security(("bearer" = [])),
    responses((status = 200, body = [TokenRecord]), (status = 401, body = ErrorBody)))]
pub(crate) async fn list_tokens(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<TokenRecord>>, ApiError> {
    let store = state.store.clone();
    let all = blocking(move || Ok(store.list_tokens()?)).await?;
    let visible = all
        .into_iter()
        .filter(|t| principal.authorize_read(&t.scope).is_ok())
        .collect();
    Ok(Json(visible))
}
