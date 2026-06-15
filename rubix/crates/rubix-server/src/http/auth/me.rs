//! `GET /auth/me` — the current principal and the capabilities it holds.
//!
//! The UI reflects what a principal may do from this: identity plus its capability
//! grants (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Auth"; ADMIN-UI open
//! question 4). Authentication runs through the shared [`Authenticated`] extractor,
//! so this works with either a bearer token or the credential headers; the grants
//! are read by the gate on the store handle (the capability layer is not a scoped
//! read).

use axum::Json;
use axum::extract::State;
use rubix_gate::list_grants;

use crate::auth::Authenticated;
use crate::dto::auth::MeResponse;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Return the authenticated principal and its granted capabilities.
pub async fn me_route(
    State(state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<MeResponse>> {
    let grants = list_grants(state.store.raw(), &auth.principal)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let capabilities = grants
        .iter()
        .map(|grant| grant.capability.as_str().to_owned())
        .collect();
    Ok(Json(MeResponse::new(&auth.principal, capabilities)))
}
