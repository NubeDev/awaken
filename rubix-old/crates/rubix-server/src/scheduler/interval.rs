//! The cadence loop for one `Interval` board. Owns a persistent
//! [`BoardEngine`] — one started reflow network kept alive for the board's
//! lifetime — and *scans* it every `seconds` instead of rebuilding the network
//! each tick. The board is re-read from the store each tick so a republished
//! version (a bumped `version`) rebuilds the engine, and disabling/deleting the
//! board drops it, without restarting the scheduler.

use std::time::Duration;

use rubix_flow::BoardEngine;
use tokio::sync::watch;
use uuid::Uuid;

use super::board_state_key;
use super::evaluate::BoardRunDeps;
use super::record::BoardRecord;

/// Drive one interval board until shutdown. Holds the board's engine (and the
/// version it was built from) across ticks; `seconds` is the **scan rate**, not a
/// rebuild trigger. Owns no graph beyond the engine: it looks the board up by its
/// globally-unique id each tick (so it needs no org/site scope). `slug` is
/// carried for logging and the output cache key.
pub(super) async fn run_interval(
    org: String,
    site_id: Option<Uuid>,
    slug: String,
    seconds: u64,
    deps: BoardRunDeps,
    mut shutdown: watch::Receiver<bool>,
) {
    // Stable node-state scope across versions, so a republish keeps the board's
    // node state (a trigger's clock) instead of resetting it.
    let state_key = board_state_key(&org, site_id, &slug);
    let mut ticker = tokio::time::interval(Duration::from_secs(seconds));
    // Skip missed ticks rather than bursting to catch up after a slow scan.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // The first tick fires immediately; consume it so the board's first scan is
    // one interval after launch, not at boot.
    ticker.tick().await;

    // The live engine and the board `version` it was built from. `None` until the
    // first scheduled tick, or after the board is disabled. Dropping it tears the
    // reflow network down (closes channels, unwinds the per-actor/forwarder
    // tasks) — the engine must be *dropped*, not merely stopped, to avoid leaking
    // forwarder tasks.
    let mut engine: Option<(BoardEngine, i64)> = None;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                // Re-fetch the *latest* version by stable identity, so a republish
                // takes effect on the next tick within this same loop.
                let board = {
                    let store = deps.store.clone();
                    let (org, slug) = (org.clone(), slug.clone());
                    tokio::task::spawn_blocking(move || store.get_board(&org, site_id, &slug)).await
                };
                match board {
                    Ok(Ok(board)) if board.is_scheduled() => {
                        if !ensure_engine(&mut engine, &board, &deps, &state_key) {
                            continue;
                        }
                        if let Some((engine, _)) = engine.as_mut() {
                            engine.scan().await;
                            deps.outputs
                                .record(&slug, &engine.current_values(), chrono::Utc::now().to_rfc3339());
                        }
                    }
                    Ok(Ok(_)) => {
                        // Disabled: drop the engine so its network is torn down.
                        if engine.take().is_some() {
                            tracing::debug!(board = %slug, "interval board disabled; engine dropped");
                        }
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
                    return; // dropping `engine` here tears the network down
                }
            }
        }
    }
}

/// Make `engine` hold a live engine for `board`'s current version, (re)building
/// it on first use or after a republish. Returns `false` (and leaves `engine`
/// empty) if the build failed, so the caller skips this scan.
fn ensure_engine(
    engine: &mut Option<(BoardEngine, i64)>,
    board: &BoardRecord,
    deps: &BoardRunDeps,
    state_key: &str,
) -> bool {
    let current = match engine {
        Some((_, version)) => *version,
        None => i64::MIN,
    };
    if current == board.version {
        return true;
    }
    // First scan or a republished version: drop the old engine (tearing its
    // network down) and build a fresh one from the new graph. The access is
    // scoped to the board's *stable* key, so node state (a trigger's clock)
    // carries across the republish rather than resetting.
    let access = deps.access_for(&board.graph, state_key);
    match board.graph.spawn_engine(access) {
        Ok(fresh) => {
            *engine = Some((fresh, board.version));
            tracing::debug!(board = %board.slug, version = board.version, "interval engine (re)built");
            true
        }
        Err(e) => {
            *engine = None;
            tracing::warn!(board = %board.slug, error = %e, "interval engine build failed");
            false
        }
    }
}
