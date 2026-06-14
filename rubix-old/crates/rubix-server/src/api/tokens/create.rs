//! POST /api/v1/tokens — issue a PAT / service-account token. The plaintext
//! bearer is returned once and never persisted (only its hash is stored). The
//! caller may not mint a token broader than its own scope, so a token cannot be
//! used to escalate privilege.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::blocking::blocking;
use crate::auth::{pat, RequestPrincipal, Role, Scope, TokenRecord};
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct IssueToken {
    /// Human label for the operator surface.
    pub name: String,
    /// The role a request bearing this token assumes.
    pub role: Role,
    /// The org/team/site the token is confined to. Must be within the issuer's
    /// own scope.
    #[serde(default)]
    pub scope: Scope,
}

/// The one-time issue response: the plaintext bearer plus the persisted record.
#[derive(Debug, Serialize, ToSchema)]
pub struct IssuedToken {
    /// The full bearer string. Shown once; store it now, it cannot be recovered.
    pub token: String,
    /// The persisted token metadata (without the secret).
    pub record: TokenRecord,
}

#[utoipa::path(post, path = "/api/v1/tokens", request_body = IssueToken, tag = "tokens",
    security(("bearer" = [])),
    responses((status = 201, body = IssuedToken), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody)))]
pub(crate) async fn create_token(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<IssueToken>,
) -> Result<(StatusCode, Json<IssuedToken>), ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("token name must not be empty".into()));
    }
    req.scope
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    // A caller may only mint a token within its own scope; auth-disabled (no
    // principal) issues unrestricted, matching the edge open posture.
    principal.authorize_write(&req.scope)?;

    let minted = pat::mint();
    let record = TokenRecord {
        id: minted.id,
        secret_hash: minted.secret_hash,
        name: req.name,
        role: req.role,
        scope: req.scope,
        created_at: Utc::now(),
        revoked_at: None,
    };
    let stored = record.clone();
    let store = state.store.clone();
    blocking(move || {
        store.create_token(&stored)?;
        Ok(())
    })
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(IssuedToken {
            token: minted.plaintext,
            record,
        }),
    ))
}
