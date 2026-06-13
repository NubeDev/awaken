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
use std::sync::{Arc, RwLock};

use rubix_flow::NodeOutput;
use serde::Serialize;
use utoipa::ToSchema;

/// One node's latest output on one port, with the wall-clock time it was
/// captured (RFC3339), so the UI can show freshness.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PortOutput {
    pub node: String,
    pub port: String,
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
    pub at: String,
}

/// A board's latest outputs, keyed by `(node, port)`.
type BoardPorts = HashMap<(String, String), PortOutput>;

/// Shared, cheaply-cloneable handle to the latest-outputs cache. Cloning shares
/// the same underlying map (it is an `Arc` inside).
#[derive(Clone, Default)]
pub struct BoardOutputs {
    inner: Arc<RwLock<HashMap<String, BoardPorts>>>,
}

impl BoardOutputs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace a board's latest outputs with the packets from one run. A run
    /// is a complete picture of what fired, so we overwrite the slug's entry
    /// rather than merge — a node that stopped emitting drops out, matching the
    /// canvas's stale-clearing behaviour.
    pub fn record(&self, slug: &str, outputs: &[NodeOutput], at: String) {
        let map: BoardPorts = outputs
            .iter()
            .map(|o| {
                let key = (o.node.clone(), o.port.clone());
                let out = PortOutput {
                    node: o.node.clone(),
                    port: o.port.clone(),
                    value: o.value.clone(),
                    at: at.clone(),
                };
                (key, out)
            })
            .collect();
        if let Ok(mut guard) = self.inner.write() {
            guard.insert(slug.to_string(), map);
        }
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

    /// Drop a board's cached outputs (on delete).
    pub fn clear(&self, slug: &str) {
        if let Ok(mut guard) = self.inner.write() {
            guard.remove(slug);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn out(node: &str, port: &str, value: serde_json::Value) -> NodeOutput {
        NodeOutput {
            node: node.into(),
            port: port.into(),
            value,
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
}
