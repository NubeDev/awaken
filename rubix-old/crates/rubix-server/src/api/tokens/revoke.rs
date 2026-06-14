//! DELETE /api/v1/tokens/{id} — revoke a PAT. The caller must be able to write
//! the token's scope, so a token can only be revoked by someone who could have
//! issued it. Idempotent: revoking an already-revoked token succeeds.

use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/tokens/{id}", tag = "tokens",
    params(("id" = String, Path, description = "Token id")),
    security(("bearer" = [])),
    responses((status = 204), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn revoke_token(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let store = state.store.clone();
    // Authorize against the token's own scope before revoking it.
    let lookup_id = id.clone();
    let record = blocking(move || {
        store
            .list_tokens()?
            .into_iter()
            .find(|t| t.id == lookup_id)
            .ok_or(ApiError::NotFound("token"))
    })
    .await?;
    principal.authorize_write(&record.scope)?;

    let store = state.store.clone();
    blocking(move || {
        store.revoke_token(&id)?;
        Ok(())
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
