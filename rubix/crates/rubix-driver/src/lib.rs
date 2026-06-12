//! Rubix driver extension contract.
//!
//! Pure types shared by the supervisor and by driver authors: the
//! [`DriverManifest`] a driver declares, the [`CapabilitySet`] that confines
//! its zenoh session to granted keyexpr prefixes, and the ack protocol for
//! writes. No IO — the supervisor (in `rubix-server`) owns spawning and health.

mod buffer;
mod error;
mod manifest;

pub use buffer::{
    ChannelClass, CurBuffer, OverflowPolicy, PendingWrite, ReliableQueue, WriteAck, WriteAction,
};
pub use error::DriverError;
pub use manifest::{
    Access, Capability, CapabilitySet, DriverManifest, Identity, Launch, PinDirection, PointType,
    ENV_DRIVER_CAPS, ENV_DRIVER_CONFIG, ENV_DRIVER_NAME,
};
