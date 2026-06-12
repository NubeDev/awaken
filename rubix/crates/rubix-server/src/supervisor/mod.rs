//! Driver supervisor: spawns manifest-described driver processes, confirms bus
//! attachment via zenoh liveliness, and restarts them with jittered backoff.
//! The only multi-process part of the platform (crash isolation from native
//! protocol stacks), per STACK-DEISGN.md.

mod backoff;
mod error;
mod health;
mod reap;
mod spawn;
mod supervise;

pub use backoff::Backoff;
pub use error::SupervisorError;
pub use health::liveliness_key;
pub use spawn::{ENV_DRIVER_CAPS, ENV_DRIVER_CONFIG, ENV_DRIVER_NAME};

use rubix_driver::DriverManifest;
use zenoh::Session;

/// Owns the supervision tasks and the shutdown signal that stops them.
pub struct Supervisor {
    shutdown: tokio::sync::watch::Sender<bool>,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl Supervisor {
    /// Validate every manifest and launch a detached supervision loop per
    /// driver on the given zenoh session. Fails closed if any manifest is
    /// invalid — a bad manifest never reaches spawn.
    pub fn launch(
        session: Session,
        manifests: Vec<DriverManifest>,
    ) -> Result<Self, SupervisorError> {
        for m in &manifests {
            m.validate()
                .map_err(|e| SupervisorError::Manifest(m.identity.name.clone(), e))?;
        }
        let (shutdown, rx) = tokio::sync::watch::channel(false);
        let handles = manifests
            .into_iter()
            .map(|m| {
                tokio::spawn(supervise::supervise(
                    session.clone(),
                    m,
                    Backoff::default(),
                    rx.clone(),
                ))
            })
            .collect();
        Ok(Self { shutdown, handles })
    }

    /// Signal all supervised drivers to stop and await their loops.
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(true);
        for h in self.handles {
            let _ = h.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubix_driver::{CapabilitySet, Identity, Launch};

    fn bad_manifest() -> DriverManifest {
        DriverManifest {
            identity: Identity {
                name: "bacnet".into(),
                protocol: "bacnet-ip".into(),
                version: "0.1.0".into(),
                launch: Launch {
                    command: "true".into(),
                    args: vec![],
                },
            },
            point_types: vec![],
            // No capabilities → invalid; launch must fail closed.
            capabilities: CapabilitySet::default(),
            config: serde_json::Value::Null,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn launch_fails_closed_on_invalid_manifest() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("session");
        let result = Supervisor::launch(session, vec![bad_manifest()]);
        match result {
            Err(SupervisorError::Manifest(name, _)) => assert_eq!(name, "bacnet"),
            Ok(_) => panic!("expected invalid manifest to fail closed"),
        }
    }
}
