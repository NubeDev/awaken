//! Per-principal preferences routes (§2).
//!
//! `GET /prefs` returns the requesting principal's display preferences (units,
//! datetime pattern, timezone), defaulting to the canonical display when none are
//! set. `PATCH /prefs` updates them. Preferences are stored as a principal-scoped
//! `kind:"prefs"` record so they ride the WS-05 gate and its audit like every
//! other write (`rubix/docs/design/DASHBOARDS-SCOPE.md` §2). One record per
//! principal, keyed deterministically by the principal's subject.

pub(crate) mod read;
mod update;

use axum::Router;
use axum::routing::{get, patch};
use rubix_core::Id;

use crate::state::AppState;

use read::get_prefs_route;
use update::patch_prefs_route;

/// The preferences routes mounted at `/prefs`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/prefs", get(get_prefs_route))
        .route("/prefs", patch(patch_prefs_route))
}

/// The deterministic record id holding `subject`'s preferences.
///
/// One record per principal, derived from the subject, so a read/write addresses
/// the same row without a lookup. Kept colon-free so it is a plain record key.
pub(crate) fn prefs_id(subject: &str) -> Id {
    Id::from_raw(format!("prefs-{subject}"))
}
