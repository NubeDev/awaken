//! The cadence loop for one `Interval` board. Ticks every `seconds`, runs the
//! board, and stops when the shutdown signal flips. The board's graph is
//! re-read from the store each tick so a republished version takes effect
//! without restarting the scheduler.

use std::time::Duration;

use tokio::sync::watch;

use std::sync::Arc;

use awaken_runtime::AgentRuntime;

use super::evaluate::evaluate;
use super::outputs::BoardOutputs;
use crate::bus::ZenohBus;
use crate::store::Store;

/// Drive one interval board until shutdown. Owns no graph: it looks the board
/// up by slug each tick, so disabling or deleting the board makes the next
/// tick a no-op rather than running a stale graph.
pub(super) async fn run_interval(
    slug: String,
    seconds: u64,
    store: Store,
    bus: Option<ZenohBus>,
    agent: Option<Arc<AgentRuntime>>,
    outputs: BoardOutputs,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(seconds));
    // Skip missed ticks rather than bursting to catch up after a slow run.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // The first tick fires immediately; consume it so the board's first run is
    // one interval after launch, not at boot.
    ticker.tick().await;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let lookup = {
                    let store = store.clone();
                    let slug = slug.clone();
                    tokio::task::spawn_blocking(move || store.get_board(&slug)).await
                };
                match lookup {
                    Ok(Ok(board)) if board.is_scheduled() => {
                        evaluate(&slug, &board.graph, &store, &bus, &agent, &outputs).await;
                    }
                    Ok(Ok(_)) => {
                        tracing::debug!(board = %slug, "interval board disabled; skipping tick");
                    }
                    Ok(Err(e)) => {
                        tracing::warn!(board = %slug, error = %e, "interval board lookup failed");
                    }
                    Err(e) => {
                        tracing::warn!(board = %slug, error = %e, "interval lookup task failed");
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    tracing::debug!(board = %slug, "interval loop stopping");
                    return;
                }
            }
        }
    }
}
