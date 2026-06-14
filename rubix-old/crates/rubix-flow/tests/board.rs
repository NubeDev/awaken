//! Board loading tests against a fake [`PointAccess`].

use std::sync::Arc;

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use rubix_core::{HisSample, PointValue, SparkSeverity};
use rubix_flow::{AgentOutcome, AgentRequest, BoardGraph, FlowAccessError, PointAccess, SparkDraft};
use rubix_rules::{MemoryRuleStore, RuleStore, StoredRule};
use serde_json::json;

/// In-memory point access: a single point with a fixed value and history.
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

/// Point access whose agent returns a canned outcome and records the request,
/// proving an awaited `agent_call` blocks on the run and surfaces its decision.
#[derive(Default)]
struct AwaitingAgentAccess {
    seen: Mutex<Vec<AgentRequest>>,
}

#[async_trait]
impl PointAccess for AwaitingAgentAccess {
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
    async fn request_agent_awaited(
        &self,
        request: AgentRequest,
    ) -> Result<AgentOutcome, FlowAccessError> {
        self.seen.lock().unwrap().push(request);
        Ok(AgentOutcome {
            run_id: "run-1".into(),
            response: "raise the setpoint".into(),
            steps: 2,
        })
    }
}

/// Point access backing the L1 rule-node path: a history series, a captured
/// spark sink, and an optional rule store for stored-rule resolution.
struct RuleAccess {
    history: Vec<HisSample>,
    store: Option<Arc<dyn RuleStore>>,
    sparks: Mutex<Vec<SparkDraft>>,
}

impl RuleAccess {
    fn series(values: &[f64], store: Option<Arc<dyn RuleStore>>) -> Self {
        let history = values
            .iter()
            .enumerate()
            .map(|(i, v)| HisSample {
                ts: Utc.timestamp_opt(i as i64 * 60, 0).unwrap(),
                value: PointValue::Number(*v),
            })
            .collect();
        Self {
            history,
            store,
            sparks: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl PointAccess for RuleAccess {
    async fn read_point(&self, _keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
        Ok(None)
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
        limit: usize,
    ) -> Result<Vec<HisSample>, FlowAccessError> {
        Ok(self.history.iter().take(limit).cloned().collect())
    }
    async fn emit_spark(&self, draft: SparkDraft) -> Result<(), FlowAccessError> {
        self.sparks.lock().unwrap().push(draft);
        Ok(())
    }
    fn rule_store(&self) -> Option<Arc<dyn RuleStore>> {
        self.store.clone()
    }
}

/// A rule script that flags when the series has an anomaly (the proven
/// script→decision bridge: `anomalies` produces a flag column, `any_true`
/// reduces it to a bool without iterating rows).
fn anomaly_rule(severity: &str, msg: &str) -> String {
    format!(
        "let flagged = df.anomalies(\"value\", 1.5); \
         if any_true(flagged, \"value_anomaly\") {{ finding(\"{severity}\", \"{msg}\") }}"
    )
}

/// query_his → rule → emit_spark, with the rule node carrying `config`. The
/// rule's verdict drives emit_spark through the structured `finding` inport.
fn rule_board(rule_config: serde_json::Value) -> serde_json::Value {
    json!({
        "nodes": [
            {"id": "q1", "component": "query_his", "config": {"point": "nube/hq/ahu-3/temp"}},
            {"id": "rule1", "component": "rule", "config": rule_config},
            {"id": "e1", "component": "emit_spark",
             "config": {"site": "nube/hq", "rule": "temp-check", "severity": "info"}}
        ],
        "connections": [
            {"from_node": "q1", "from_port": "output", "to_node": "rule1", "to_port": "input"},
            {"from_node": "rule1", "from_port": "finding", "to_node": "e1", "to_port": "finding"}
        ]
    })
}

/// query_his → rule, with no outbound connection on the rule node, so its own
/// outports (`clear`/`error`) are readable from the run output (a connected
/// node's outport is drained by the network's forwarder).
fn rule_only_board(rule_config: serde_json::Value) -> serde_json::Value {
    json!({
        "nodes": [
            {"id": "q1", "component": "query_his", "config": {"point": "nube/hq/ahu-3/temp"}},
            {"id": "rule1", "component": "rule", "config": rule_config}
        ],
        "connections": [
            {"from_node": "q1", "from_port": "output", "to_node": "rule1", "to_port": "input"}
        ]
    })
}

// A clear, single-spike series: three flat samples then a spike, so
// `anomalies(z=1.5)` flags the last row.
const SPIKE: [f64; 4] = [20.0, 20.0, 20.0, 60.0];
const FLAT: [f64; 4] = [20.0, 20.0, 20.0, 20.0];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_node_flags_and_emits_with_rule_severity() {
    // L1 + L3: a board runs query→rule→emit_spark; the rule's `fault` severity
    // reaches the spark, overriding emit_spark's static `info` config.
    let cfg = json!({ "script": anomaly_rule("fault", "spike detected") });
    let graph: BoardGraph = serde_json::from_value(rule_board(cfg)).expect("parse board");
    let access = Arc::new(RuleAccess::series(&SPIKE, None));
    graph.run(access.clone()).await.expect("run board");

    let sparks = access.sparks.lock().unwrap();
    assert_eq!(sparks.len(), 1, "expected one emitted spark");
    assert_eq!(sparks[0].severity, SparkSeverity::Fault);
    assert_eq!(sparks[0].message, "spike detected");
    assert_eq!(sparks[0].rule, "temp-check");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_node_clear_result_emits_no_spark() {
    let cfg = json!({ "script": anomaly_rule("fault", "spike detected") });
    let graph: BoardGraph = serde_json::from_value(rule_board(cfg.clone())).expect("parse board");
    let access = Arc::new(RuleAccess::series(&FLAT, None));
    graph.run(access.clone()).await.expect("run board");
    assert!(access.sparks.lock().unwrap().is_empty(), "clear must not emit");

    // The rule node fires its `clear` port — a normal no-finding, not an error.
    let graph: BoardGraph = serde_json::from_value(rule_only_board(cfg)).expect("parse board");
    let outputs = graph
        .run(Arc::new(RuleAccess::series(&FLAT, None)))
        .await
        .expect("run board");
    assert!(outputs.iter().any(|o| o.node == "rule1" && o.port == "clear"));
    assert!(!outputs.iter().any(|o| o.node == "rule1" && o.port == "error"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_node_resolves_a_stored_rule_by_name() {
    // L2: the rule node references a stored rule by id; the tenant store resolves
    // it and the verdict flows to emit_spark.
    let store = MemoryRuleStore::new().with(StoredRule::new(
        "stored-1",
        "temp-high",
        anomaly_rule("warning", "stored flagged"),
    ));
    let cfg = json!({ "rule": "temp-high" });
    let graph: BoardGraph = serde_json::from_value(rule_board(cfg)).expect("parse board");
    let access = Arc::new(RuleAccess::series(&SPIKE, Some(Arc::new(store))));
    graph.run(access.clone()).await.expect("run board");

    let sparks = access.sparks.lock().unwrap();
    assert_eq!(sparks.len(), 1);
    assert_eq!(sparks[0].severity, SparkSeverity::Warning);
    assert_eq!(sparks[0].message, "stored flagged");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_node_broken_script_fails_the_node() {
    let cfg = json!({ "script": "this is not valid rhai @@@" });
    let graph: BoardGraph = serde_json::from_value(rule_only_board(cfg)).expect("parse board");
    let access = Arc::new(RuleAccess::series(&SPIKE, None));
    let outputs = graph.run(access.clone()).await.expect("run board");

    assert!(access.sparks.lock().unwrap().is_empty());
    let err = outputs
        .iter()
        .find(|o| o.node == "rule1" && o.port == "error")
        .expect("rule error present");
    assert!(err.value.to_string().contains("rule:"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_node_caps_breach_fails_the_node() {
    // Truncated input must fail, not fold partial rows into a finding.
    let cfg = json!({ "script": anomaly_rule("fault", "x"), "max_rows": 2 });
    let graph: BoardGraph = serde_json::from_value(rule_only_board(cfg)).expect("parse board");
    let access = Arc::new(RuleAccess::series(&SPIKE, None));
    let outputs = graph.run(access).await.expect("run board");
    let err = outputs
        .iter()
        .find(|o| o.node == "rule1" && o.port == "error")
        .expect("caps-breach error present");
    assert!(err.value.to_string().contains("cap exceeded"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_node_stored_without_store_fails_closed() {
    let cfg = json!({ "rule": "temp-high" });
    let graph: BoardGraph = serde_json::from_value(rule_only_board(cfg)).expect("parse board");
    let access = Arc::new(RuleAccess::series(&SPIKE, None));
    let outputs = graph.run(access.clone()).await.expect("run board");

    assert!(access.sparks.lock().unwrap().is_empty());
    assert!(outputs.iter().any(|o| o.node == "rule1" && o.port == "error"));
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
