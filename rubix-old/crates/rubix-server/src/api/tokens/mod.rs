//! PAT / service-account admin routes — wiring only. Issue, list, and revoke
//! bearer tokens that authenticate non-interactive callers (drivers, the
//! embedded agent's surface, CI). Gated by the caller's own scope so a token
//! cannot escalate privilege.

pub(crate) mod create;
pub(crate) mod list;
pub(crate) mod revoke;

use axum::routing::{delete, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/tokens",
            post(create::create_token).get(list::list_tokens),
        )
        .route("/api/v1/tokens/{id}", delete(revoke::revoke_token))
}
