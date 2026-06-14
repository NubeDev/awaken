//! GET /api/v1/whoami — the resolved identity of the caller.
//!
//! The UI reads this once at boot so it can render permission-aware chrome
//! (show/hide admin nav, disable write controls a Viewer cannot use) instead of
//! holding an opaque token with no idea who it is. See
//! `docs/design/authz-rbac.md` increment A.
//!
//! When auth is **off** (the edge/dev default, no principal on the request) this
//! returns a synthetic global operator so the dev UI behaves as fully-authorized
//! — matching the gate convention where every `authorize_*` passes with no
//! principal. `auth_enabled: false` lets the UI tell "dev open" from a real
//! global admin if it wants to.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth::{RequestPrincipal, Role, Scope};
use crate::AppState;

/// The caller's resolved identity and the coarse capabilities the UI gates on.
#[derive(Debug, Serialize, ToSchema)]
pub struct Whoami {
    /// Stable subject (JWT `sub` or PAT id). `"dev"` when auth is off.
    pub subject: String,
    /// The org/team/site the caller is confined to (omitted levels are global).
    #[schema(value_type = Object)]
    pub scope: Scope,
    /// The caller's role.
    #[schema(value_type = String)]
    pub role: Role,
    /// True when the caller may mutate within scope (Admin/Operator/Service; not Viewer).
    pub can_write: bool,
    /// True when the caller may manage identity/authorization (users, teams,
    /// grants) at their scope — i.e. an org-admin or super-admin. The UI gates
    /// the Members/Teams/Access surfaces on this. `true` in auth-off dev so the
    /// surfaces render, though their mutations require a real admin server-side.
    pub can_admin: bool,
    /// True when auth is actually enforced. `false` in the edge/dev profile, so
    /// the UI can distinguish "open dev server" from a real global principal.
    pub auth_enabled: bool,
}

#[utoipa::path(get, path = "/api/v1/whoami", tag = "auth",
    security(("bearer" = [])),
    responses((status = 200, body = Whoami)))]
pub(crate) async fn whoami(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
) -> Json<Whoami> {
    match principal.0 {
        Some(p) => Json(Whoami {
            subject: p.subject,
            can_write: p.role.can_write(),
            can_admin: p.role.can_admin(),
            scope: p.scope,
            role: p.role,
            auth_enabled: true,
        }),
        // Auth off: synthesize a global operator so the dev UI is fully enabled.
        // `can_admin` is true so the admin surfaces render in dev (their writes
        // still demand a real admin server-side, per `require_admin`).
        None => Json(Whoami {
            subject: "dev".into(),
            scope: Scope::global(),
            role: Role::Operator,
            can_write: true,
            can_admin: true,
            auth_enabled: false,
        }),
    }
}
