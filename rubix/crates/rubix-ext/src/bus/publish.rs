//! Authorize an extension to publish onto the in-process control event bus.
//!
//! The effect counterpart to [`subscribe_events`](super::subscribe_events): a
//! single fail-closed [`EventPublish`](rubix_gate::Capability::EventPublish)
//! decision, then pure delegation to [`rubix_bus::publish`]. An out-of-grant
//! extension reaches no subscriber — the event is refused before it fans out, so
//! emitting onto the platform's coordination spine is never an accidental side
//! effect of merely holding a bus handle. The [`ControlEvent`] already carries a
//! gate-minted correlation id threading it to the action that produced it, so the
//! audit trail is preserved through that thread rather than by persisting each
//! ephemeral publish (the same once-checked, gate-bypassing shape as the
//! data-plane stream capabilities).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_bus::{ControlBus, ControlEvent, publish};
use rubix_core::Principal;
use rubix_gate::Capability;

use crate::authz::authorize;
use crate::error::Result;

/// Authorize `extension` to publish `event` onto the control event bus.
///
/// Confirms the [`EventPublish`](Capability::EventPublish) grant fail closed, then
/// fans `event` out to every current subscriber of its type via
/// [`rubix_bus::publish`]. Returns the number of subscribers reached — publishing
/// to a type with no subscribers is `0`, not an error, because the control plane
/// decouples components (a publisher never depends on a listener existing).
///
/// # Errors
/// Returns [`ExtError::Denied`](crate::ExtError::Denied) if the extension lacks
/// the `event-publish` grant (before the event fans out) or the grant lookup
/// fails — never a silent allow.
pub async fn publish_event(
    db: &Surreal<Db>,
    extension: &Principal,
    bus: &ControlBus,
    event: ControlEvent,
) -> Result<usize> {
    authorize(db, extension, Capability::EventPublish).await?;
    Ok(publish(bus, event))
}
