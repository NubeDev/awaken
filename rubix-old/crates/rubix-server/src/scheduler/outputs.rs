//! Latest per-node output values from board runs, kept in memory so a client
//! can see what an enabled (autonomously running) board is producing.
//!
//! A scheduled board's durable effect is its point writes and emitted sparks;
//! the transient packets each node emits are not persisted. This cache keeps
//! only the *latest* value per `(slug, node, port)` from the most recent run of
//! each board — enough to paint live values on the editor canvas, not a history
//! tier. It is written by [`super::evaluate`] (scheduled runs) and the on-demand
//! run endpoints, and read by `GET /boards/{slug}/outputs`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use rubix_flow::NodeOutput;
use serde::Serialize;
use tokio::sync::broadcast;
use utoipa::ToSchema;

/// How many snapshots a slow live-stream subscriber may fall behind before it is
/// fast-forwarded to the latest (a lagged SSE client skips stale frames rather
/// than wedging the broadcast). Control boards emit a few links per scan, so a
/// small ring is ample.
const STREAM_LAG: usize = 16;

/// One node's latest output on one port, with the wall-clock time it was
/// captured (RFC3339), so the UI can show freshness.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PortOutput {
    pub node: String,
    pub port: String,
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
    /// Link quality (`ok`/`fault`/`null`) so a retained value is self-describing.
    pub quality: String,
    pub at: String,
}

/// A board's latest outputs, keyed by `(node, port)`.
type BoardPorts = HashMap<(String, String), PortOutput>;

/// Shared, cheaply-cloneable handle to the latest-outputs cache plus a per-board
/// live broadcast. Cloning shares the same underlying map and channels (both are
/// `Arc` inside). Every `record` updates the cache *and* pushes the new snapshot
/// to that board's subscribers, so the SSE stream and the REST snapshot are fed
/// from the same point — a scheduled scan, a subscription run, or an on-demand
/// run all surface live without a separate path.
#[derive(Clone, Default)]
pub struct BoardOutputs {
    inner: Arc<RwLock<HashMap<String, BoardPorts>>>,
    channels: Arc<Mutex<HashMap<String, broadcast::Sender<Vec<PortOutput>>>>>,
}

impl BoardOutputs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace a board's latest outputs with the packets from one run, and push
    /// the new snapshot to any live subscribers. A run is a complete picture of
    /// what fired, so we overwrite the slug's entry rather than merge — a node
    /// that stopped emitting drops out, matching the canvas's stale-clearing
    /// behaviour.
    pub fn record(&self, slug: &str, outputs: &[NodeOutput], at: String) {
        let map: BoardPorts = outputs
            .iter()
            .map(|o| {
                let key = (o.node.clone(), o.port.clone());
                let out = PortOutput {
                    node: o.node.clone(),
                    port: o.port.clone(),
                    value: o.value.clone(),
                    quality: o.quality.as_str().to_string(),
                    at: at.clone(),
                };
                (key, out)
            })
            .collect();
        let snapshot: Vec<PortOutput> = map.values().cloned().collect();
        if let Ok(mut guard) = self.inner.write() {
            guard.insert(slug.to_string(), map);
        }
        self.broadcast(slug, snapshot);
    }

    /// The latest outputs for a board, or an empty vec if it has not run since
    /// the server started.
    pub fn latest(&self, slug: &str) -> Vec<PortOutput> {
        self.inner
            .read()
            .ok()
            .and_then(|g| g.get(slug).map(|m| m.values().cloned().collect()))
            .unwrap_or_default()
    }

    /// Subscribe to a board's live snapshots: returns the current snapshot to
    /// seed the client plus a receiver for every subsequent `record`. The
    /// receiver is created lazily and outlives individual runs, so a client can
    /// connect before the board has produced anything.
    pub fn subscribe(&self, slug: &str) -> (Vec<PortOutput>, broadcast::Receiver<Vec<PortOutput>>) {
        let rx = self.sender(slug).subscribe();
        (self.latest(slug), rx)
    }

    /// Drop a board's cached outputs (on delete/disable) and push an empty
    /// snapshot so live subscribers blank rather than hold a stale picture.
    pub fn clear(&self, slug: &str) {
        if let Ok(mut guard) = self.inner.write() {
            guard.remove(slug);
        }
        self.broadcast(slug, Vec::new());
    }

    /// The broadcast sender for `slug`, created on first use.
    fn sender(&self, slug: &str) -> broadcast::Sender<Vec<PortOutput>> {
        let mut channels = self.channels.lock().expect("board-outputs channels poisoned");
        channels
            .entry(slug.to_string())
            .or_insert_with(|| broadcast::channel(STREAM_LAG).0)
            .clone()
    }

    /// Push a snapshot to a board's subscribers, if any. A send with no
    /// receivers is a no-op (not an error) — we keep the sender for later.
    fn broadcast(&self, slug: &str, snapshot: Vec<PortOutput>) {
        let sender = {
            let channels = self.channels.lock().expect("board-outputs channels poisoned");
            channels.get(slug).cloned()
        };
        if let Some(sender) = sender {
            let _ = sender.send(snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn out(node: &str, port: &str, value: serde_json::Value) -> NodeOutput {
        let quality = rubix_flow::Quality::of(port, &value);
        NodeOutput {
            node: node.into(),
            port: port.into(),
            value,
            quality,
        }
    }

    #[test]
    fn record_then_latest_roundtrips() {
        let cache = BoardOutputs::new();
        cache.record(
            "b",
            &[out("trigger", "output", json!(true)), out("trigger", "count", json!(3))],
            "2026-06-13T00:00:00Z".into(),
        );
        let mut got = cache.latest("b");
        got.sort_by(|a, b| a.port.cmp(&b.port));
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].port, "count");
        assert_eq!(got[0].value, json!(3));
    }

    #[test]
    fn a_new_run_replaces_the_previous_outputs() {
        let cache = BoardOutputs::new();
        cache.record("b", &[out("n", "a", json!(1))], "t1".into());
        cache.record("b", &[out("n", "b", json!(2))], "t2".into());
        let got = cache.latest("b");
        assert_eq!(got.len(), 1, "stale port from the first run is gone");
        assert_eq!(got[0].port, "b");
    }

    #[test]
    fn unknown_board_is_empty_not_an_error() {
        assert!(BoardOutputs::new().latest("nope").is_empty());
    }

    #[test]
    fn subscribe_seeds_then_record_pushes_the_new_snapshot() {
        let cache = BoardOutputs::new();
        let (seed, mut rx) = cache.subscribe("b");
        assert!(seed.is_empty(), "no run yet → empty seed");

        cache.record("b", &[out("n", "a", json!(7))], "t1".into());
        let pushed = rx.try_recv().expect("subscriber receives the run's snapshot");
        assert_eq!(pushed.len(), 1);
        assert_eq!(pushed[0].value, json!(7));
    }

    #[test]
    fn clear_pushes_an_empty_snapshot_so_subscribers_blank() {
        let cache = BoardOutputs::new();
        let (_seed, mut rx) = cache.subscribe("b");
        cache.record("b", &[out("n", "a", json!(1))], "t1".into());
        let _ = rx.try_recv();

        cache.clear("b");
        let pushed = rx.try_recv().expect("clear pushes a frame");
        assert!(pushed.is_empty(), "cleared board blanks live subscribers");
    }
}
