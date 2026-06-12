//! Board loading tests against a fake [`PointAccess`].

use std::sync::Arc;

use std::sync::Mutex;

use rubix_core::{HisSample, PointValue};
use rubix_flow::{AgentOutcome, AgentRequest, BoardGraph, PointAccess};
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

/// Point access whose agent returns a canned outcome and records the request,
/// proving an awaited `agent_call` blocks on the run and surfaces its decision.
#[derive(Default)]
struct AwaitingAgentAccess {
    seen: Mutex<Vec<AgentRequest>>,
}

impl PointAccess for AwaitingAgentAccess {
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
    fn request_agent_blocking(&self, request: AgentRequest) -> anyhow::Result<AgentOutcome> {
        self.seen.lock().unwrap().push(request);
        Ok(AgentOutcome {
            run_id: "run-1".into(),
            response: "raise the setpoint".into(),
            steps: 2,
        })
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn awaited_agent_call_surfaces_the_decision_downstream() {
    // read_point → agent_call(await) → write_point: the awaited node blocks on
    // the agent and emits its response text, which the write node commands.
    let graph: BoardGraph = serde_json::from_value(json!({
        "nodes": [
            {"id": "r1", "component": "read_point", "config": {"point": "nube/hq/ahu-3/temp"}},
            {"id": "a1", "component": "agent_call",
             "config": {"prompt": "ahu-3 is off", "await": true}},
            {"id": "w1", "component": "write_point",
             "config": {"point": "nube/hq/ahu-3/sp", "priority": 8}}
        ],
        "connections": [
            {"from_node": "r1", "from_port": "output", "to_node": "a1", "to_port": "value"},
            {"from_node": "a1", "from_port": "output", "to_node": "w1", "to_port": "value"}
        ]
    }))
    .expect("parse board");
    let access = Arc::new(AwaitingAgentAccess::default());
    let outputs = graph.run(access.clone()).await.expect("run board");

    // The awaited node's `output` is consumed by the connector that forwards it
    // to the write node, so the decision is observable at the terminal write
    // node: it commanded the point with the agent's response text. That the
    // value reached the (priority-array) write proves the awaited agent_call
    // surfaced the decision downstream in the same single-shot run.
    let written = outputs
        .iter()
        .find(|o| o.node == "w1" && o.port == "output")
        .expect("write_point output present");
    assert_eq!(written.value, json!("raise the setpoint"));

    // The agent was actually asked, with the read value winning over the static
    // prompt (inport beats config, the documented precedence).
    let seen = access.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].prompt, "21.5");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn awaited_agent_call_without_an_agent_errors() {
    let graph: BoardGraph = serde_json::from_value(json!({
        "nodes": [{"id": "a1", "component": "agent_call",
                   "config": {"prompt": "do it", "await": true}}],
        "connections": []
    }))
    .expect("parse board");
    // FakeAccess does not override `request_agent_blocking`, so it fails closed.
    let outputs = graph.run(Arc::new(FakeAccess)).await.expect("run board");
    let err = outputs
        .iter()
        .find(|o| o.node == "a1" && o.port == "error")
        .expect("agent_call error present");
    assert!(
        err.value.to_string().contains("no agent runtime"),
        "{}",
        err.value
    );
}

#[test]
fn board_roundtrips_through_json() {
    let graph: BoardGraph = serde_json::from_value(board_json()).unwrap();
    let encoded = serde_json::to_value(&graph).unwrap();
    let back: BoardGraph = serde_json::from_value(encoded).unwrap();
    assert_eq!(back.nodes.len(), 2);
    assert_eq!(back.connections.len(), 1);
}
