//! The bridge from a gated lifecycle command to a running process.
//!
//! Phase 1 ([`supervisor`](crate::supervisor)) can spawn a child; phase 3
//! ([`metrics`](crate::metrics)) can count one. This module is the **bridge**
//! that wires them to the gate (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase
//! 2): nothing reads the gated `lifecycle` field back until something here does.
//!
//! Two halves, both keeping the gated lifecycle record as the single source of
//! truth (there is **no** separate enablement side-table):
//!
//! - **Handler-drives** ([`drive_lifecycle`]): after the gated `lifecycle` write
//!   lands, drive the supervisor to match (`start` → spawn, `stop`/`disable` →
//!   shut down) and report the observed state. The gate decides *who may*; this
//!   only *acts* on a decision already audited.
//! - **Boot reconciler** ([`reconcile_from_records`] / [`reconcile_on_session`]):
//!   on host boot, read every extension's current lifecycle record and bring the
//!   supervisor map into agreement — extensions last left in `start` are
//!   re-spawned, `stop`/`disable` stay down. This is rubix's gate-native
//!   equivalent of starter's "EnablementStore queried at boot", making the
//!   edge-reboot story work without a side table.
//!
//! It also hosts the real liveness probe ([`probe_extension_health`]): a
//! process-flavour extension's liveness is its supervisor's, not a session ping
//! that only proves the session is signed in (phase 5).
//!
//! [`ExtensionRuntime`] bundles the supervisor + metrics registries so the host
//! (`rubix-server`'s `AppState`) holds one cloneable handle and both the HTTP
//! lifecycle route and the boot path share the same live maps.

mod drive;
mod health;
mod reconcile;

pub use drive::{LifecycleOutcome, drive_lifecycle};
pub use health::probe_extension_health;
pub use reconcile::{ReconcileReport, reconcile_from_records, reconcile_on_session};

use crate::metrics::MetricsRegistry;
use crate::supervisor::SupervisorRegistry;

/// The live runtime state for the extension system: the supervisor map and the
/// metrics registry, bundled so a host threads one cloneable handle.
///
/// Cloning is an `Arc` bump on each inner registry, so an `AppState` clone keeps
/// pointing at the *same* live supervisors and counters — a lifecycle command on
/// one request and a metrics read on another see one shared world.
#[derive(Clone, Debug, Default)]
pub struct ExtensionRuntime {
    /// The map of `ExtensionId → SupervisorHandle` for live process-flavour
    /// extensions.
    pub supervisors: SupervisorRegistry,
    /// The per-extension counter registry the planes bump and the admin surface
    /// folds into the metrics view.
    pub metrics: MetricsRegistry,
}

impl ExtensionRuntime {
    /// A fresh runtime with empty supervisor and metrics maps.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
