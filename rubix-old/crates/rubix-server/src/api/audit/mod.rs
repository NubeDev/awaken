//! Audit + undo/redo routes over the change ledger (docs/design/audit-and-undo.md
//! "Audit read surface", "Undo/Redo"). Wiring only; one file per verb. The audit
//! reads are admin-gated and org-scoped (a cross-org read is impossible by
//! construction — the store query always filters by `org`); undo/redo are per-actor
//! and CAS-guarded.
//!
//! [`record`] is the handler-facing recorder mutation handlers call to append a
//! ledger row next to the mutation they describe.

pub(crate) mod dispatch;
pub(crate) mod list;
pub(crate) mod record;
pub(crate) mod redo;
pub(crate) mod timeline;
pub(crate) mod undo;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/audit", get(list::list_audit))
        .route("/api/v1/audit/{kind}/{id}", get(timeline::resource_timeline))
        .route("/api/v1/undo", post(undo::undo))
        .route("/api/v1/redo", post(redo::redo))
}
