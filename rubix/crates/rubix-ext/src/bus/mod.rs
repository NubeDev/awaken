//! The extension's event-bus plane — gated access to the in-process control bus.
//!
//! An extension reaches the platform's **in-process control event bus**
//! (`rubix-bus`) the same way it reaches every other plane: as a scoped
//! principal, authorized once against the same WS-04 grant table a user is
//! (`rubix/docs/SCOPE.md`, "Event bus"; `rubix/docs/sessions/WS-13.md`, contract
//! #2). The control bus itself imposes no authority — any holder of a
//! [`ControlBus`](rubix_bus::ControlBus) handle could publish or subscribe — so
//! this seam is where the capability decision lives for a principal that is not a
//! trusted in-binary component. It mirrors the data plane ([`crate::data`]): one
//! fail-closed capability check, then pure delegation to `rubix-bus`.
//!
//! Two distinct authorities, never collapsed:
//!
//! - [`subscribe_events`] requires [`EventSubscribe`](rubix_gate::Capability::EventSubscribe)
//!   — the right to *observe* the control stream.
//! - [`publish_event`] requires [`EventPublish`](rubix_gate::Capability::EventPublish)
//!   — the right to *emit* onto it, driving other components.
//!
//! Each is checked once at the seam, not re-taxed per event — the same shape as
//! [`ZenohSubscribe`](rubix_gate::Capability::ZenohSubscribe) on the data plane.
//!
//! The **data-change** (live-query) plane is deliberately *not* re-exposed here:
//! an extension subscribes to record changes through its own gate-issued scoped
//! session, where SurrealDB row-level permissions scope what it sees — reads are
//! SurrealDB-native (`rubix/docs/SCOPE.md`, principle 7), so wrapping them behind
//! a second capability would invent a path that could drift from the one a user
//! is held to.

mod publish;
mod subscribe;

pub use publish::publish_event;
pub use subscribe::subscribe_events;
