//! The subscription loop for one `Subscription` board. Subscribes to the board's
//! key through the seam's `watch` primitive (the one event substrate, shared with
//! any future `watch`-consuming node — no second zenoh subscriber on the same
//! key) and runs the board each time a `cur` sample arrives. The graph is re-read
//! from the store per sample so a republished version takes effect without
//! restarting the subscriber.

use futures::StreamExt;
use rubix_flow::PointAccess;
use tokio::sync::watch;

use super::evaluate::{evaluate, BoardRunDeps};

/// Drive one subscription board until shutdown. A `watch` declare failure is
/// logged and the loop exits — the board simply won't fire, surfaced in the log,
/// rather than crashing the scheduler. `watch` is backed by `deps.bus`, which the
/// scheduler has already confirmed is present before spawning this loop.
pub(super) async fn run_subscription(
    board_id: uuid::Uuid,
    slug: String,
    key: String,
    deps: BoardRunDeps,
    mut shutdown: watch::Receiver<bool>,
) {
    let access = deps.watch_access();
    let mut stream = match access.watch(&key).await {
        Ok(stream) => stream,
        Err(e) => {
            tracing::warn!(board = %slug, key = %key, error = %e, "watch declare failed");
            return;
        }
    };
    loop {
        tokio::select! {
            sample = stream.next() => {
                match sample {
                    Some(_) => {
                        let lookup = {
                            let store = deps.store.clone();
                            tokio::task::spawn_blocking(move || store.get_board_by_id(board_id)).await
                        };
                        match lookup {
                            Ok(Ok(board)) if board.is_scheduled() => {
                                evaluate(&slug, board_id, &board.graph, &deps).await;
                            }
                            Ok(Ok(_)) => {}
                            Ok(Err(e)) => {
                                tracing::warn!(board = %slug, error = %e, "subscription board lookup failed");
                            }
                            Err(e) => {
                                tracing::warn!(board = %slug, error = %e, "subscription lookup task failed");
                            }
                        }
                    }
                    None => {
                        tracing::debug!(board = %slug, "watch stream closed");
                        return;
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    tracing::debug!(board = %slug, "subscription loop stopping");
                    return;
                }
            }
        }
    }
}
