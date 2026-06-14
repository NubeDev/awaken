//! Launch a driver child process from its manifest.

use rubix_driver::DriverManifest;
// The env-var names are part of the driver contract; re-export the canonical
// definitions from `rubix-driver` so the spawner and driver binaries agree.
pub use rubix_driver::{ENV_DRIVER_CAPS, ENV_DRIVER_CONFIG, ENV_DRIVER_NAME};
use tokio::process::{Child, Command};

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
