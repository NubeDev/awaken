//! Authorize an extension to subscribe to the in-process control event bus.
//!
//! The seam is a single fail-closed capability decision followed by pure
//! delegation to [`rubix_bus::subscribe`]: an extension holding the
//! [`EventSubscribe`](rubix_gate::Capability::EventSubscribe) grant is handed a
//! live [`ControlSubscription`] on the requested event type; an out-of-grant
//! extension is denied before any receiver is opened. The grant check runs
//! through the gate's [`check_capability`](rubix_gate::check_capability) on the
//! same WS-04 grant table a user is authorized off — no extension-only authz
//! path (`rubix/docs/sessions/WS-13.md`, contract #2).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_bus::{ControlBus, ControlSubscription, subscribe};
use rubix_core::Principal;
use rubix_gate::Capability;

use crate::authz::authorize;
use crate::error::Result;

/// Authorize `extension` to subscribe to `event_type` on the control event bus.
///
/// Confirms the [`EventSubscribe`](Capability::EventSubscribe) grant fail closed,
/// then opens a [`ControlSubscription`] on `bus` for `event_type`. The scope
/// decision is taken **once, here** — the returned subscription is not re-checked
/// per event, the same shape the data plane's
/// [`authorize_data_scope`](crate::authorize_data_scope) uses. Events published
/// before this call are not replayed; the control plane is live coordination, not
/// a log.
///
/// # Errors
/// Returns [`ExtError::Denied`](crate::ExtError::Denied) if the extension lacks
/// the `event-subscribe` grant (before any receiver is opened) or the grant
/// lookup fails — never a silent allow.
pub async fn subscribe_events(
    db: &Surreal<Db>,
    extension: &Principal,
    bus: &ControlBus,
    event_type: &str,
) -> Result<ControlSubscription> {
    authorize(db, extension, Capability::EventSubscribe).await?;
    Ok(subscribe(bus, event_type))
}
