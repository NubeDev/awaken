//! Per-driver supervision loop: spawn, confirm bus attachment, watch for exit,
//! then restart with jittered exponential backoff. Runs detached per driver.

use std::time::Duration;

use rubix_driver::DriverManifest;
use zenoh::Session;

use super::backoff::Backoff;
use super::health::await_attach;
use super::reap::await_clear;
use super::spawn::spawn;

const ATTACH_TIMEOUT: Duration = Duration::from_secs(10);
const REAP_TIMEOUT: Duration = Duration::from_secs(15);

/// Supervise one driver until `shutdown` resolves. Each iteration waits for any
/// stale liveliness to clear, spawns the process, confirms it attaches to the
/// bus, then waits on its exit; failures drive the backoff.
pub async fn supervise(
    session: Session,
    manifest: DriverManifest,
    backoff: Backoff,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let name = manifest.identity.name.clone();
    let mut failures: u32 = 0;
    loop {
        if *shutdown.borrow() {
            return;
        }
        if !await_clear(&session, &name, REAP_TIMEOUT).await {
            failures = failures.saturating_add(1);
            wait_backoff(&backoff, failures, &mut shutdown).await;
            continue;
        }

        let mut child = match spawn(&manifest) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(driver = %name, error = %e, "spawn failed");
                failures = failures.saturating_add(1);
                wait_backoff(&backoff, failures, &mut shutdown).await;
                continue;
            }
        };

        if !await_attach(&session, &name, ATTACH_TIMEOUT).await {
            tracing::warn!(driver = %name, "did not attach to bus; killing");
            let _ = child.kill().await;
            failures = failures.saturating_add(1);
            wait_backoff(&backoff, failures, &mut shutdown).await;
            continue;
        }
        tracing::info!(driver = %name, "driver up");
        failures = 0;

        tokio::select! {
            status = child.wait() => {
                tracing::warn!(driver = %name, ?status, "driver exited; restarting");
                failures = failures.saturating_add(1);
                wait_backoff(&backoff, failures, &mut shutdown).await;
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    let _ = child.kill().await;
                    return;
                }
            }
        }
    }
}

/// Sleep the jittered backoff for `failures`, returning early if shutdown fires.
async fn wait_backoff(
    backoff: &Backoff,
    failures: u32,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
) {
    let delay = backoff.delay(failures, jitter());
    tokio::select! {
        _ = tokio::time::sleep(delay) => {}
        _ = shutdown.changed() => {}
    }
}

/// Full-jitter fraction in `[0, 1)` from the wall clock — good enough to
/// de-correlate a fleet without pulling in an RNG dependency.
fn jitter() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    f64::from(nanos % 1_000_000_000) / 1_000_000_000.0
}
