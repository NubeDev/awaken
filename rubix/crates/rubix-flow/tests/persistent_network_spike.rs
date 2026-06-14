//! Keystone spike for the flow-runtime redesign: does reflow 0.2 support a
//! *persistent* started `Network` — one we tick repeatedly and drain
//! incrementally — without a `shutdown()`/rebuild per tick?
//!
//! The whole redesign (a long-lived `BoardEngine` scanned on a cadence, instead
//! of build→run→shutdown every interval) rests on this. These tests prove the
//! mechanism against the real crate rather than the design doc's assumption, and
//! also pin down two facts the engine design depends on:
//!
//! 1. A source actor's outport is consumed by the network's fan-out forwarder,
//!    so `read_actor_output` on a *connected* node is unreliable — its packets go
//!    to the downstream actor, not to an outside drainer.
//! 2. Every wired link emits a `NetworkEvent::MessageSent { from, port, value }`
//!    on `get_event_receiver()`. That event stream — not `read_actor_output` — is
//!    the complete, race-free tap for the live-value bus.

use std::sync::Arc;
use std::time::Duration;

use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use reflow_actor::message::Message;
use reflow_network::network::NetworkEvent;
use rubix_core::{HisSample, PointValue};
use rubix_flow::{BoardGraph, FlowAccessError, PointAccess};
use serde_json::json;

/// Reads a fixed value; echoes each command so writes are observable.
struct FakeAccess;

#[async_trait]
impl PointAccess for FakeAccess {
    async fn read_point(&self, _keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
        Ok(Some(PointValue::Number(21.5)))
    }
    async fn write_point(
        &self,
        _keyexpr: &str,
        _priority: u8,
        value: PointValue,
    ) -> Result<Option<PointValue>, FlowAccessError> {
        Ok(Some(value))
    }
    async fn query_his(
        &self,
        _keyexpr: &str,
        _limit: usize,
    ) -> Result<Vec<HisSample>, FlowAccessError> {
        Ok(vec![])
    }
}

/// `read_point r1 → write_point w1`: r1 is the source (ticked), w1 is terminal.
fn read_to_write() -> BoardGraph {
    serde_json::from_value(json!({
        "nodes": [
            {"id": "r1", "component": "read_point", "config": {"point": "nube/hq/ahu-3/temp"}},
            {"id": "w1", "component": "write_point",
             "config": {"point": "nube/hq/ahu-3/fan", "priority": 8}}
        ],
        "connections": [
            {"from_node": "r1", "from_port": "output", "to_node": "w1", "to_port": "value"}
        ]
    }))
    .expect("parse board")
}

/// THE KEYSTONE: start the network once, then tick the source repeatedly and
/// drain incrementally, never calling `shutdown()` between ticks. If reflow only
/// supported single-shot runs the actor tasks would have exited after the first
/// tick and later ticks would produce nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn started_network_survives_repeated_ticks_without_shutdown() {
    // A changing source value (1, 2, 3, 4) so each tick is a distinct command —
    // `write_point` coalesces *unchanged* re-commands, which a constant value
    // would trigger, hiding the per-tick liveness this test is about.
    let mut net = read_to_write()
        .load(Arc::new(CountingAccess::default()))
        .expect("load");
    net.start().expect("start");

    const TICKS: usize = 4;
    for _ in 0..TICKS {
        // Re-tick the source on its declared inport — the same kick the
        // single-shot `run()` does once, here repeated on a live network.
        net.send_to_actor("r1", "trigger", Message::Flow)
            .expect("tick source");
        // Let the packet propagate r1 → forwarder → w1 before the next tick.
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    // Settle the last tick.
    tokio::time::sleep(Duration::from_millis(60)).await;

    // The terminal node w1 has no downstream, so no forwarder competes for its
    // outport: `read_actor_output` reliably drains every value it commanded.
    let w1: Vec<_> = net.read_actor_output("w1");
    let w1_values: Vec<_> = w1
        .iter()
        .filter(|(p, _)| p.as_str() == "output")
        .map(|(_, m)| serde_json::Value::from(m.clone()))
        .collect();
    assert_eq!(
        w1_values.len(),
        TICKS,
        "a persistent network must keep producing one terminal output per tick \
         (got {w1_values:?})"
    );
    assert_eq!(
        w1_values,
        vec![json!(1.0), json!(2.0), json!(3.0), json!(4.0)],
        "each tick re-reads the next value and commands it downstream: {w1_values:?}"
    );

    net.shutdown();
}

/// The live-value tap: every wired link surfaces on the event stream as a
/// `MessageSent`, carrying the *intermediate* node's value that
/// `read_actor_output` cannot see because the forwarder ate it. This is the
/// channel the persistent live-value bus must subscribe to.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn message_sent_events_tap_intermediate_link_values() {
    let mut net = read_to_write().load(Arc::new(FakeAccess)).expect("load");
    net.start().expect("start");
    let events = net.get_event_receiver();

    const TICKS: usize = 3;
    for _ in 0..TICKS {
        net.send_to_actor("r1", "trigger", Message::Flow)
            .expect("tick source");
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    tokio::time::sleep(Duration::from_millis(60)).await;

    // r1 is a *connected* source; the forwarder consumes its outport, so draining
    // r1 directly is unreliable — but the r1 → w1 edge fires a MessageSent each
    // tick, giving the bus r1's value without racing the forwarder.
    let mut r1_link_values = Vec::new();
    while let Ok(ev) = events.try_recv() {
        if let NetworkEvent::MessageSent {
            from_actor,
            from_port,
            message,
            ..
        } = ev
        {
            if from_actor == "r1" && from_port == "output" {
                r1_link_values.push(serde_json::Value::from(message));
            }
        }
    }
    assert_eq!(
        r1_link_values.len(),
        TICKS,
        "the r1 → w1 link must emit one MessageSent per tick (got {r1_link_values:?})"
    );
    assert!(
        r1_link_values.iter().all(|v| *v == json!(21.5)),
        "the tap carries the intermediate node's real value: {r1_link_values:?}"
    );

    net.shutdown();
}

/// Reads a monotonically increasing value each call, so retained-value updates
/// across scans are observable.
#[derive(Default)]
struct CountingAccess {
    n: AtomicU64,
}

#[async_trait]
impl PointAccess for CountingAccess {
    async fn read_point(&self, _keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
        let n = self.n.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(Some(PointValue::Number(n as f64)))
    }
    async fn write_point(
        &self,
        _keyexpr: &str,
        _priority: u8,
        value: PointValue,
    ) -> Result<Option<PointValue>, FlowAccessError> {
        Ok(Some(value))
    }
    async fn query_his(
        &self,
        _keyexpr: &str,
        _limit: usize,
    ) -> Result<Vec<HisSample>, FlowAccessError> {
        Ok(vec![])
    }
}

/// The `BoardEngine` realizes the keystone: one started network scanned
/// repeatedly, retaining the latest value of each link — interior (r1, from the
/// event tap) and terminal (w1) — and updating it every scan.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn board_engine_retains_and_updates_link_values_across_scans() {
    let access = std::sync::Arc::new(CountingAccess::default());
    let mut engine = read_to_write()
        .spawn_engine(access)
        .expect("spawn engine");

    // First scan reads 1 and commands it downstream.
    engine.scan().await;
    let after_first = engine.current_values();
    let r1_first = after_first
        .iter()
        .find(|o| o.node == "r1" && o.port == "output")
        .expect("r1 interior link captured via the event tap");
    let w1_first = after_first
        .iter()
        .find(|o| o.node == "w1" && o.port == "output")
        .expect("w1 terminal output captured directly");
    assert_eq!(r1_first.value, json!(1.0));
    assert_eq!(w1_first.value, json!(1.0));

    // A second scan updates the retained values in place — no rebuild, no blank.
    engine.scan().await;
    let after_second = engine.current_values();
    let w1_second = after_second
        .iter()
        .find(|o| o.node == "w1" && o.port == "output")
        .expect("w1 still present after the second scan");
    assert_eq!(
        w1_second.value,
        json!(2.0),
        "the retained value advances each scan on the same live network"
    );
    // Exactly one value per (node, port) is retained — not an accumulating log.
    assert_eq!(
        after_second.iter().filter(|o| o.node == "w1").count(),
        1,
        "retained snapshot keeps one value per link"
    );
}

/// Reads a constant value and counts commands, so write coalescing is visible.
#[derive(Default)]
struct StickyAccess {
    writes: AtomicU64,
}

#[async_trait]
impl PointAccess for StickyAccess {
    async fn read_point(&self, _keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
        Ok(Some(PointValue::Number(7.0)))
    }
    async fn write_point(
        &self,
        _keyexpr: &str,
        _priority: u8,
        value: PointValue,
    ) -> Result<Option<PointValue>, FlowAccessError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(Some(value))
    }
    async fn query_his(
        &self,
        _keyexpr: &str,
        _limit: usize,
    ) -> Result<Vec<HisSample>, FlowAccessError> {
        Ok(vec![])
    }
}

/// On the persistent engine, `write_point` re-ticks every scan but coalesces an
/// unchanged command: a board commanding the same value should hit the priority
/// array once, not once per scan.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn board_engine_coalesces_unchanged_writes() {
    let access = std::sync::Arc::new(StickyAccess::default());
    let mut engine = read_to_write()
        .spawn_engine(access.clone())
        .expect("spawn engine");

    for _ in 0..3 {
        engine.scan().await;
    }

    assert_eq!(
        access.writes.load(Ordering::SeqCst),
        1,
        "an unchanged value commands the priority array once, then coalesces"
    );
}

/// A lone `trigger` node (source and terminal) with a tiny period, so it fires
/// every scan.
fn trigger_board() -> BoardGraph {
    serde_json::from_value(json!({
        "nodes": [
            {"id": "t1", "component": "trigger", "config": {"every": 0.001, "unit": "sec"}}
        ],
        "connections": []
    }))
    .expect("parse board")
}

/// Stage D: the trigger's fire count lives in actor state, which now survives
/// across scans on the persistent engine — so `count` advances rather than
/// resetting (it no longer needs the process-global registry).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trigger_state_persists_across_scans() {
    let mut engine = trigger_board()
        .spawn_engine(std::sync::Arc::new(FakeAccess))
        .expect("spawn engine");

    engine.scan().await;
    let count_after_first = trigger_count(&engine);

    engine.scan().await;
    let count_after_second = trigger_count(&engine);

    assert_eq!(count_after_first, Some(json!(1)), "first scan is the boot fire");
    assert_eq!(
        count_after_second,
        Some(json!(2)),
        "the fire count advances across scans — actor state survived"
    );
}

fn trigger_count(engine: &rubix_flow::BoardEngine) -> Option<serde_json::Value> {
    engine
        .current_values()
        .into_iter()
        .find(|o| o.node == "t1" && o.port == "count")
        .map(|o| o.value)
}
