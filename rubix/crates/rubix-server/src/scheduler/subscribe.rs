//! The subscription loop for one `Subscription` board. Declares a zenoh
//! subscriber on the board's key and runs the board each time a `cur` sample
//! arrives — the edge "run a control board off a live value" trigger. The
//! graph is re-read from the store per sample so a republished version takes
//! effect without restarting the subscriber.

use futures::StreamExt;
use tokio::sync::watch;

use super::evaluate::{evaluate, BoardRunDeps};
use crate::bus::ZenohBus;

/// Drive one subscription board until shutdown. A declare failure is logged
/// and the loop exits — the board simply won't fire, surfaced in the log,
/// rather than crashing the scheduler. The bus backs both the subscriber (passed
/// concretely) and, via `deps.bus`, the board's `emit_spark` publishing.
pub(super) async fn run_subscription(
    slug: String,
    key: String,
    bus: ZenohBus,
    deps: BoardRunDeps,
    mut shutdown: watch::Receiver<bool>,
) {
    let subscriber = match bus.session_clone().declare_subscriber(&key).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(board = %slug, key = %key, error = %e, "subscriber declare failed");
            return;
        }
    };
    let mut stream = subscriber.stream();
    loop {
        tokio::select! {
            sample = stream.next() => {
                match sample {
                    Some(_) => {
                        let lookup = {
                            let store = deps.store.clone();
                            let slug = slug.clone();
                            tokio::task::spawn_blocking(move || store.get_board(&slug)).await
                        };
                        match lookup {
                            Ok(Ok(board)) if board.is_scheduled() => {
                                evaluate(&slug, &board.graph, &deps).await;
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
                        tracing::debug!(board = %slug, "subscriber stream closed");
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
