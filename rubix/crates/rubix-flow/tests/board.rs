//! Board loading tests against a fake [`PointAccess`].

use std::sync::Arc;

use rubix_core::{HisSample, PointValue};
use rubix_flow::{BoardGraph, PointAccess};
use serde_json::json;

/// In-memory point access: a single point with a fixed value and history.
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

fn board_json() -> serde_json::Value {
    json!({
        "nodes": [
            {"id": "r1", "component": "read_point", "config": {"point": "nube/hq/ahu-3/temp"}},
            {"id": "w1", "component": "write_point", "config": {"point": "nube/hq/ahu-3/fan", "priority": 8}}
        ],
        "connections": [
            {"from_node": "r1", "from_port": "output", "to_node": "w1", "to_port": "value"}
        ]
    })
}

#[test]
fn valid_board_loads_into_network() {
    let graph: BoardGraph = serde_json::from_value(board_json()).expect("parse board");
    let access: Arc<dyn PointAccess> = Arc::new(FakeAccess);
    graph.load(access).expect("load board");
}

#[test]
fn unknown_component_fails_closed() {
    let graph: BoardGraph = serde_json::from_value(json!({
        "nodes": [{"id": "x", "component": "frobnicate", "config": {}}],
        "connections": []
    }))
    .expect("parse board");
    let access: Arc<dyn PointAccess> = Arc::new(FakeAccess);
    match graph.load(access) {
        Err(rubix_flow::FlowError::UnknownComponent(c)) => assert_eq!(c, "frobnicate"),
        Ok(_) => panic!("expected unknown component to fail closed"),
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn running_a_board_emits_node_output() {
    let graph: BoardGraph = serde_json::from_value(json!({
        "nodes": [
            {"id": "r1", "component": "read_point", "config": {"point": "nube/hq/ahu-3/temp"}}
        ],
        "connections": []
    }))
    .expect("parse board");
    let access: Arc<dyn PointAccess> = Arc::new(FakeAccess);
    let outputs = graph.run(access).await.expect("run board");
    // The read_point source node reads the fake 21.5 and emits it on `output`.
    let out = outputs
        .iter()
        .find(|o| o.node == "r1" && o.port == "output")
        .expect("r1 output present");
    assert_eq!(out.value, json!(21.5));
}

#[test]
fn board_roundtrips_through_json() {
    let graph: BoardGraph = serde_json::from_value(board_json()).unwrap();
    let encoded = serde_json::to_value(&graph).unwrap();
    let back: BoardGraph = serde_json::from_value(encoded).unwrap();
    assert_eq!(back.nodes.len(), 2);
    assert_eq!(back.connections.len(), 1);
}
