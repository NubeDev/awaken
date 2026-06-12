//! Rubix driver extension contract.
//!
//! Pure types shared by the supervisor and by driver authors: the
//! [`DriverManifest`] a driver declares, the [`CapabilitySet`] that confines
//! its zenoh session to granted keyexpr prefixes, and the ack protocol for
//! writes. No IO — the supervisor (in `rubix-server`) owns spawning and health.

mod error;
mod manifest;

pub use error::DriverError;
pub use manifest::{
    Access, Capability, CapabilitySet, DriverManifest, Identity, Launch, PinDirection, PointType,
};
