//! Supervisor errors: surfaced at launch, before any process is spawned.

use rubix_driver::DriverError;

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("driver `{0}` manifest invalid: {1}")]
    Manifest(String, #[source] DriverError),
}
