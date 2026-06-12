//! Launch a driver child process from its manifest.

use rubix_driver::DriverManifest;
use tokio::process::{Child, Command};

/// Env var carrying the driver's own name (also its liveliness-token key).
pub const ENV_DRIVER_NAME: &str = "RUBIX_DRIVER_NAME";
/// Env var carrying the JSON-encoded [`rubix_driver::CapabilitySet`] the
/// driver must confine its session to.
pub const ENV_DRIVER_CAPS: &str = "RUBIX_DRIVER_CAPS";
/// Env var carrying the driver's JSON config blob.
pub const ENV_DRIVER_CONFIG: &str = "RUBIX_DRIVER_CONFIG";

/// Spawn the driver process described by `manifest`, passing its identity,
/// granted capabilities, and config through the environment. The child speaks
/// zenoh on its own; the supervisor only owns its lifecycle.
pub fn spawn(manifest: &DriverManifest) -> std::io::Result<Child> {
    let caps = serde_json::to_string(&manifest.capabilities).unwrap_or_else(|_| "{}".into());
    let config = serde_json::to_string(&manifest.config).unwrap_or_else(|_| "null".into());
    Command::new(&manifest.identity.launch.command)
        .args(&manifest.identity.launch.args)
        .env(ENV_DRIVER_NAME, &manifest.identity.name)
        .env(ENV_DRIVER_CAPS, caps)
        .env(ENV_DRIVER_CONFIG, config)
        .kill_on_drop(true)
        .spawn()
}
