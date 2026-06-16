//! Extension admin routes — drive and observe the extension runtime.
//!
//! The HTTP face of `rubix-ext`'s runtime half (`rubix/docs/design/
//! EXTENSION-RUNTIME.md`, "Admin HTTP surface"), mounted into the existing
//! `rubix-server` rather than a parallel Axum app. Route shapes are ported from
//! `starter-ext-server`; the auth and persistence are re-wired through rubix:
//!
//! - **Reads** (`GET /extensions*`) run on the caller's WS-03 scoped session, so
//!   SurrealDB row-level permissions confine them to the caller's namespace —
//!   per-tenant by construction, no parallel middleware. Process/metrics/event
//!   gauges are read off the in-memory supervisor + metrics registries (no DB).
//! - **The one mutation** (`POST /extensions/<id>/lifecycle`) crosses the WS-05
//!   gate inside `rubix-ext`, so the capability check ([`ExtensionManage`]) is
//!   the gate's, fail closed and audited — there is no starter `with_role` and no
//!   `EnablementStore` side row; the gated lifecycle record is the source of
//!   truth.
//!
//! One file per route (`rubix/docs/FILE-LAYOUT.md`); this barrel only merges them.
//!
//! [`ExtensionManage`]: rubix_gate::Capability::ExtensionManage

mod events;
mod get;
mod health;
mod lifecycle;
mod list;
mod metrics;
mod process;
mod shared;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

/// The extension admin routes mounted under `/extensions`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/extensions", get(list::list_extensions_route))
        .route("/extensions/:id", get(get::get_extension_route))
        .route("/extensions/:id/process", get(process::process_extension_route))
        .route("/extensions/:id/metrics", get(metrics::metrics_extension_route))
        .route("/extensions/:id/events", get(events::events_extension_route))
        .route(
            "/extensions/:id/lifecycle",
            post(lifecycle::lifecycle_extension_route),
        )
        .route("/extensions/:id/health", post(health::health_extension_route))
}
