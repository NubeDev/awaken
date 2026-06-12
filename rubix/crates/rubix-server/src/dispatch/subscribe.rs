//! The subscribe loop: drain `**/spark/**` and activate an agent run per
//! finding. Runs detached until shutdown. Each run is awaited in turn so a
//! single agent runtime is not driven concurrently by a burst of findings —
//! dispatch favors ordered, bounded execution over throughput (a building's
//! finding rate is low and runs command real points).

use std::sync::Arc;

use awaken_runtime::AgentRuntime;
use futures::StreamExt;
use tokio::sync::watch;

use super::run::dispatch_spark;
use crate::bus::ZenohBus;
use crate::store::Store;

/// The keyexpr every site's spark publications fall under
/// (`{org}/{site}/spark/{rule}/{id}`).
const SPARK_KEY: &str = "**/spark/**";

/// Subscribe to spark publications and dispatch each to the agent until
/// shutdown. A declare failure is logged and the loop exits (dispatch is off,
/// surfaced in the log, rather than crashing the server).
pub(super) async fn run_dispatch(
    bus: ZenohBus,
    runtime: Arc<AgentRuntime>,
    store: Store,
    mut shutdown: watch::Receiver<bool>,
) {
    let subscriber = match bus.session_clone().declare_subscriber(SPARK_KEY).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "dispatch: spark subscriber declare failed");
            return;
        }
    };
    let mut stream = subscriber.stream();
    loop {
        tokio::select! {
            sample = stream.next() => {
                match sample {
                    Some(sample) => {
                        dispatch_spark(&sample.payload().to_bytes(), &runtime, &store).await;
                    }
                    None => {
                        tracing::debug!("dispatch: spark stream closed");
                        return;
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    tracing::debug!("dispatch loop stopping");
                    return;
                }
            }
        }
    }
}
