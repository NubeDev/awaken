//! Integration: the insight write is gated, and the firing reaches the bus.
//!
//! Recording an insight crosses the WS-05 gate as a `rule-invoke` command
//! (`rubix/docs/sessions/WS-11.md`): an ungranted principal is denied before any
//! write (fail closed, contract #2), and a granted firing is published on the
//! WS-07 in-process bus so a subscriber observes it carrying the evaluation
//! correlation id (contract #3).

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_bus::{ControlBus, subscribe};
use rubix_core::Id;
use rubix_query::{CanonicalTable, Grain};
use rubix_rules::{
    Aggregate, Binding, INSIGHT_EVENT_TYPE, Rule, RuleRegistry, RuleRuntime, evaluate,
};
use rubix_trace::{SampleRate, assemble_trace};

use fixture::open::{granted_session, open_rules_store, seed_reading, ungranted_session};

fn high_temp_rule() -> Rule {
    Rule::new(
        Id::from_raw("high-temp"),
        "temp > 15.0",
        vec![Binding::new(
            "temp",
            CanonicalTable::Records,
            "temperature",
            Grain::Minute,
            Aggregate::Avg,
        )],
        "high-temperature",
    )
}

/// A rule whose script returns a decision map carrying §5c evaluation scores and
/// a group id — turning the firing into a comparable evaluation datapoint.
fn scoring_rule() -> Rule {
    Rule::new(
        Id::from_raw("temp-eval"),
        r#"#{ fired: temp > 15.0, value: temp, scores: #{ groundedness: 0.9, severity: temp }, group_id: "thermal-qa" }"#,
        vec![Binding::new(
            "temp",
            CanonicalTable::Records,
            "temperature",
            Grain::Minute,
            Aggregate::Avg,
        )],
        "temp-evaluation",
    )
}

#[tokio::test]
async fn an_ungranted_principal_is_denied_the_insight_write() {
    let database = "rule_denied";
    let handle = open_rules_store(database).await;
    seed_reading(&handle, "temperature", 0, 30.0).await;

    let (principal, session) = ungranted_session(&handle, database, "mallory").await;
    let bus = ControlBus::new();
    let mut registry = RuleRegistry::new();
    registry.insert(high_temp_rule());

    let rt = RuleRuntime {
        gate_db: handle.raw(),
        session: session.connection(),
        trace_db: handle.raw(),
        bus: &bus,
        sample: SampleRate::new(0.0),
    };
    let result = evaluate(&rt, &registry, &principal, &Id::from_raw("high-temp")).await;
    assert!(result.is_err(), "no rule-invoke grant must deny the firing");

    // No insight was written, and no audit row produced — fail closed.
    let records: Option<i64> = handle
        .raw()
        .query("SELECT VALUE count() FROM record WHERE content.kind = 'high-temperature' GROUP ALL")
        .await
        .expect("read records")
        .take(0)
        .expect("decode");
    assert_eq!(records.unwrap_or(0), 0, "denied evaluation writes no insight");
}

#[tokio::test]
async fn a_firing_is_published_on_the_bus_with_the_correlation_id() {
    let database = "rule_published";
    let handle = open_rules_store(database).await;
    seed_reading(&handle, "temperature", 0, 30.0).await;

    let (principal, session) = granted_session(&handle, database, "alice").await;
    let bus = ControlBus::new();
    // Subscribe before evaluating so the firing is delivered.
    let mut subscription = subscribe(&bus, INSIGHT_EVENT_TYPE);

    let mut registry = RuleRegistry::new();
    registry.insert(high_temp_rule());

    let rt = RuleRuntime {
        gate_db: handle.raw(),
        session: session.connection(),
        trace_db: handle.raw(),
        bus: &bus,
        sample: SampleRate::new(0.0),
    };
    let evaluation = evaluate(&rt, &registry, &principal, &Id::from_raw("high-temp"))
        .await
        .expect("evaluate");

    assert!(evaluation.event_reach >= 1, "the subscriber must be reached");
    let event = subscription.recv().await.expect("receive firing");
    assert_eq!(event.event_type(), INSIGHT_EVENT_TYPE);
    assert_eq!(event.correlation_id(), &evaluation.correlation_id);
    assert_eq!(event.payload()["kind"], "high-temperature");
    assert_eq!(event.payload()["fired"], true);
    assert_eq!(
        event.payload()["insight_id"],
        evaluation.insight_id.as_str()
    );
}

#[tokio::test]
async fn evaluation_scores_round_trip_through_the_gate_correlated_to_the_trace() {
    let database = "rule_scores";
    let handle = open_rules_store(database).await;
    seed_reading(&handle, "temperature", 0, 30.0).await;

    let (principal, session) = granted_session(&handle, database, "alice").await;
    let bus = ControlBus::new();
    let mut registry = RuleRegistry::new();
    registry.insert(scoring_rule());

    let rt = RuleRuntime {
        gate_db: handle.raw(),
        session: session.connection(),
        trace_db: handle.raw(),
        bus: &bus,
        sample: SampleRate::new(0.0),
    };
    let evaluation = evaluate(&rt, &registry, &principal, &Id::from_raw("temp-eval"))
        .await
        .expect("evaluate");

    // The scores + group_id landed in the persisted insight content, written
    // through the gate (not a direct write).
    let content: Option<serde_json::Value> = handle
        .raw()
        .query("SELECT VALUE content FROM record WHERE content.kind = 'temp-evaluation' LIMIT 1")
        .await
        .expect("read insight")
        .take(0)
        .expect("decode");
    let content = content.expect("insight recorded");
    assert_eq!(content["scores"]["groundedness"], 0.9);
    assert_eq!(content["scores"]["severity"], 30.0);
    assert_eq!(content["group_id"], "thermal-qa");

    // The evaluation is correlated to the trace: the firing's correlation id is
    // the trace id of the persisted span tree.
    let forest = assemble_trace(handle.raw(), &evaluation.correlation_id)
        .await
        .expect("assemble trace");
    assert!(!forest.is_empty(), "the evaluation produced a span tree");
    assert_eq!(forest[0].span.trace_id, evaluation.correlation_id);
}
