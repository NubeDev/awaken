//! Environment-variable names the supervisor injects into a driver process and
//! a driver reads back. Part of the driver contract, so both the spawner
//! (`rubix-server`) and any driver binary share one definition.

/// Carries the driver's own name — also its liveliness-token key.
pub const ENV_DRIVER_NAME: &str = "RUBIX_DRIVER_NAME";
/// Carries the JSON-encoded [`crate::CapabilitySet`] the driver must confine
/// its session to.
pub const ENV_DRIVER_CAPS: &str = "RUBIX_DRIVER_CAPS";
/// Carries the driver's JSON config blob.
pub const ENV_DRIVER_CONFIG: &str = "RUBIX_DRIVER_CONFIG";
