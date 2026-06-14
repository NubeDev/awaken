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

use reflow_actor::message::Message;
use reflow_network::network::NetworkEvent;
use rubix_core::{HisSample, PointValue};
use rubix_flow::{BoardGraph, PointAccess};
use serde_json::json;

/// Reads a fixed value; records each command so writes are observable.
struct FakeAccess;

impl PointAccess for FakeAccess {
    fn read_point(&self, _keyexpr: &str) -> anyhow::Result<Option<PointValue>> {
        Ok(Some(PointValue::Number(21.5)))
    }
    fn write_point(
        &self,
        _keyexpr: &str,
        _priority: u8,
        value: PointValue,
    ) -> anyhow::Result<Option<PointValue>> {
        Ok(Some(value))
    }
    fn query_his(&self, _keyexpr: &str, _limit: usize) -> anyhow::Result<Vec<HisSample>> {
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
    let mut net = read_to_write().load(Arc::new(FakeAccess)).expect("load");
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
    assert!(
        w1_values.iter().all(|v| *v == json!(21.5)),
        "each tick re-reads 21.5 and commands it downstream: {w1_values:?}"
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
