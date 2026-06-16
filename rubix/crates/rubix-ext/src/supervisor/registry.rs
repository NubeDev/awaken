//! [`SupervisorRegistry`] — the in-memory map of live extension supervisors.
//!
//! rubix's "registry of known extensions" is the set of `Extension` principals in
//! SurrealDB, and the durable enablement state is the gated `lifecycle` record —
//! **not** a sealed manifest tree (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! "Reject for identity; borrow the registry idea"). This registry is only the
//! lightweight runtime half of that: a map from [`ExtensionId`] to the live
//! [`SupervisorHandle`], so the bridge and the admin projections can find the
//! handle for an extension without re-spawning it. The database stays the source
//! of truth; this map is a derived, rebuildable cache the boot reconciler brings
//! into agreement with the lifecycle records.
//!
//! Cheap to clone (an `Arc` bump) so it lives in the server's `AppState` and is
//! shared across request handlers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::handle::SupervisorHandle;
use super::id::ExtensionId;
use super::spec::ProcessSpec;
use super::task::{Identity, Supervisor};
use crate::error::Result;

/// A shared, cloneable map of `ExtensionId → SupervisorHandle`.
#[derive(Clone, Default)]
pub struct SupervisorRegistry {
    inner: Arc<Mutex<HashMap<ExtensionId, SupervisorHandle>>>,
}

impl std::fmt::Debug for SupervisorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.inner.lock().map(|g| g.len()).unwrap_or(0);
        f.debug_struct("SupervisorRegistry")
            .field("supervisors", &len)
            .finish()
    }
}

impl SupervisorRegistry {
    /// A fresh, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensure a supervisor is running for `id`, spawning one from `spec` if the
    /// extension is not already live.
    ///
    /// Idempotent: if a handle is already present and live, it is returned
    /// unchanged (a second `start` does not respawn a healthy child). A
    /// present-but-dead handle (stopped/failed) is replaced with a fresh
    /// supervisor.
    ///
    /// # Errors
    /// Propagates [`Supervisor::start`] failure (no tokio runtime).
    pub fn start(
        &self,
        id: ExtensionId,
        spec: ProcessSpec,
        identity: Identity,
    ) -> Result<SupervisorHandle> {
        {
            let guard = self.inner.lock().expect("supervisor registry poisoned");
            if let Some(existing) = guard.get(&id)
                && existing.is_live()
            {
                return Ok(existing.clone());
            }
        }
        let handle = Supervisor::start(id.clone(), spec, identity)?;
        self.inner
            .lock()
            .expect("supervisor registry poisoned")
            .insert(id, handle.clone());
        Ok(handle)
    }

    /// Stop the supervisor for `id`, if present, and drop it from the map.
    ///
    /// Returns `true` if a supervisor was present and asked to shut down. The
    /// durable `stop`/`disable` decision lives in the gated lifecycle record;
    /// this only tears down the runtime side. A subsequent
    /// [`get`](Self::get) returns `None`, so the projections fall back to the
    /// record's persisted state (exactly the starter "no live supervisor"
    /// degradation).
    pub async fn stop(&self, id: &ExtensionId) -> bool {
        let handle = self
            .inner
            .lock()
            .expect("supervisor registry poisoned")
            .remove(id);
        match handle {
            Some(h) => {
                h.shutdown().await;
                true
            }
            None => false,
        }
    }

    /// Stop any current supervisor for `id` and start a fresh one from `spec`.
    ///
    /// # Errors
    /// Propagates [`Supervisor::start`] failure.
    pub async fn restart(
        &self,
        id: ExtensionId,
        spec: ProcessSpec,
        identity: Identity,
    ) -> Result<SupervisorHandle> {
        self.stop(&id).await;
        let handle = Supervisor::start(id.clone(), spec, identity)?;
        self.inner
            .lock()
            .expect("supervisor registry poisoned")
            .insert(id, handle.clone());
        Ok(handle)
    }

    /// The live handle for `id`, if one is registered.
    #[must_use]
    pub fn get(&self, id: &ExtensionId) -> Option<SupervisorHandle> {
        self.inner
            .lock()
            .expect("supervisor registry poisoned")
            .get(id)
            .cloned()
    }

    /// Whether a supervisor is registered for `id` (regardless of its state).
    #[must_use]
    pub fn contains(&self, id: &ExtensionId) -> bool {
        self.inner
            .lock()
            .expect("supervisor registry poisoned")
            .contains_key(id)
    }

    /// Every registered `(id, handle)` pair, for the list/overview projections.
    #[must_use]
    pub fn list(&self) -> Vec<(ExtensionId, SupervisorHandle)> {
        self.inner
            .lock()
            .expect("supervisor registry poisoned")
            .iter()
            .map(|(id, h)| (id.clone(), h.clone()))
            .collect()
    }
}
