//! JSON-RPC `health` probe — the one read-only control method.
//!
//! `health` reports the extension's liveness (`rubix/docs/sessions/WS-13.md`).
//! Unlike every other control method it crosses **no** command and writes **no**
//! audit row: a liveness probe is not a mutation, so routing it through the gate
//! would pollute the audit log. It runs a trivial read on the extension's WS-03
//! scoped session, which confirms both that the engine answers and that the
//! extension's session is still validly signed in to its namespace — a probe
//! that returns is proof the scoped session is live.

use rubix_gate::ScopedSession;

use crate::error::{ExtError, Result};

/// The liveness verdict a `health` probe returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// The extension is live — its scoped session answered (builtin fallback) or
    /// its supervised child is `Running` (process-flavour, see
    /// [`probe_extension_health`](crate::runtime::probe_extension_health)).
    Healthy,
    /// The extension is **not** live — a process-flavour extension whose
    /// supervised child is not `Running` (stopped, crashed, failed, or
    /// mid-restart). Distinct from an error: the probe itself succeeded, the
    /// verdict is just negative.
    Unhealthy,
}

/// Probe the extension's liveness on its scoped `session`.
///
/// Issues a trivial `RETURN true` on the principal-scoped connection. A
/// successful round-trip proves the engine is reachable and the session is still
/// signed in; the call returns [`HealthStatus::Healthy`]. No command is applied
/// and no audit row is written.
///
/// # Errors
/// Returns [`ExtError::Command`] if the probe query itself fails.
pub async fn probe_health(session: &ScopedSession) -> Result<HealthStatus> {
    let mut response = session
        .connection()
        .query("RETURN true")
        .await
        .map_err(|e| ExtError::Command(e.to_string()))?;
    let alive: Option<bool> = response
        .take(0)
        .map_err(|e| ExtError::Command(e.to_string()))?;
    if alive == Some(true) {
        Ok(HealthStatus::Healthy)
    } else {
        Err(ExtError::Command("health probe returned no liveness".to_owned()))
    }
}
