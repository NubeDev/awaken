//! Real liveness: probe the supervisor for process-flavour, the session for the
//! rest.
//!
//! The builtin [`probe_health`](crate::probe_health) runs a `RETURN true` on the
//! extension's scoped session — it proves the *session* is signed in, not that
//! any child is alive (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase 5). For a
//! process-flavour extension that is the wrong question: the session can be
//! perfectly valid while the child has crashed. [`probe_extension_health`] fixes
//! that by asking the **supervisor** first:
//!
//! - If a supervisor is registered for the extension, its liveness *is* the
//!   answer — `Running` → [`HealthStatus::Healthy`], anything else (starting,
//!   crashed, stopped, mid-restart) → [`HealthStatus::Unhealthy`]. No session
//!   ping; the process is the workload.
//! - If no supervisor is registered (builtin extension, or one never started),
//!   fall back to the session ping — the host does the work in-process, so a
//!   live session is the meaningful liveness signal.

use rubix_gate::ScopedSession;

use crate::control::{HealthStatus, probe_health};
use crate::error::Result;
use crate::supervisor::ExtensionId;

use super::ExtensionRuntime;

/// Probe an extension's real liveness.
///
/// Consults the supervisor for `id` first (process-flavour liveness); falls back
/// to the scoped-session ping when no supervisor is registered (the builtin
/// path). Returns the verdict, not an error, when a process-flavour child is
/// simply down — an error is reserved for a failed probe.
///
/// # Errors
/// Returns the session-ping error (via [`probe_health`]) only on the builtin
/// fallback path, when the scoped query itself fails.
pub async fn probe_extension_health(
    rt: &ExtensionRuntime,
    id: &ExtensionId,
    session: &ScopedSession,
) -> Result<HealthStatus> {
    if let Some(handle) = rt.supervisors.get(id) {
        return Ok(if handle.is_live() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        });
    }
    // No supervised child — builtin (or never started). The session ping is the
    // meaningful liveness signal: the host runs the work under this session.
    probe_health(session).await
}
