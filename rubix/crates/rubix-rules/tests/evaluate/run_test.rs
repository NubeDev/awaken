//! Integration: a rule fires offline on a window value and records its insight.
//!
//! Proves the core path of the rule runtime (`rubix/docs/sessions/WS-11.md`): a
//! known reading series is rolled up into a window value, a rule's script decides
//! on it, and the decision is recorded as an insight through the WS-05 gate and
//! published as a data-change event — all offline on kv-mem, no cloud dependency
//! (`rubix/docs/SCOPE.md`, principle 3).

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_bus::ControlBus;
use rubix_core::Id;
use rubix_query::{CanonicalTable, Grain};
use rubix_rules::{Aggregate, Binding, Rule, RuleRegistry, RuleRuntime, evaluate};
use rubix_trace::SampleRate;

use fixture::open::{granted_session, open_rules_store, seed_reading};

/// Build a runtime over `handle` and `session`, persisting every span.
fn runtime<'a>(
    handle: &'a rubix_store::StoreHandle,
    session: &'a rubix_gate::ScopedSession,
    bus: &'a ControlBus,
) -> RuleRuntime<'a> {
    RuleRuntime {
        gate_db: handle.raw(),
        session: session.connection(),
        trace_db: handle.raw(),
        bus,
        sample: SampleRate::new(0.0),
    }
}

#[tokio::test]
async fn a_rule_fires_on_a_window_value_and_records_its_insight() {
    let database = "rule_fire";
    let handle = open_rules_store(database).await;

    // Average of the first minute is (10+20+30)/3 = 20 > 15, so the rule fires.
    seed_reading(&handle, "temperature", 0, 10.0).await;
    seed_reading(&handle, "temperature", 20, 20.0).await;
    seed_reading(&handle, "temperature", 40, 30.0).await;

    let (principal, session) = granted_session(&handle, database, "alice").await;
    let bus = ControlBus::new();

    let mut registry = RuleRegistry::new();
    let rule = Rule::new(
        Id::from_raw("high-temp"),
        r#"#{ fired: temp > 15.0, value: temp, reason: "minute avg over threshold" }"#,
        vec![Binding::new(
            "temp",
            CanonicalTable::Records,
            "temperature",
            Grain::Minute,
            Aggregate::Avg,
        )],
        "high-temperature",
    );
    registry.insert(rule);

    let rt = runtime(&handle, &session, &bus);
    let evaluation = evaluate(&rt, &registry, &principal, &Id::from_raw("high-temp"))
        .await
        .expect("evaluate");

    assert!(evaluation.decision.fired, "avg 20 > 15 must fire");
    assert_eq!(evaluation.decision.value, 20.0);

    // The insight was recorded through the gate as a generic record.
    let stored: Option<serde_json::Value> = handle
        .raw()
        .query("SELECT * FROM record WHERE id = $id")
        .bind(("id", surreal_id(&evaluation.insight_id)))
        .await
        .expect("read insight")
        .take(0)
        .expect("decode insight");
    let stored = stored.expect("insight row present");
    assert_eq!(stored["content"]["kind"], "high-temperature");
    assert_eq!(stored["content"]["fired"], true);

    // The gate wrote an audit row carrying the evaluation correlation id.
    let audit_count: Option<i64> = handle
        .raw()
        .query("SELECT VALUE count() FROM audit WHERE correlation_id = $cid GROUP ALL")
        .bind(("cid", evaluation.correlation_id.to_string()))
        .await
        .expect("read audit")
        .take(0)
        .expect("decode audit");
    assert_eq!(audit_count.unwrap_or(0), 1, "one audit row for the firing");
}

#[tokio::test]
async fn a_rule_below_threshold_records_a_non_firing_insight() {
    let database = "rule_quiet";
    let handle = open_rules_store(database).await;

    seed_reading(&handle, "temperature", 0, 1.0).await;
    seed_reading(&handle, "temperature", 20, 2.0).await;

    let (principal, session) = granted_session(&handle, database, "bob").await;
    let bus = ControlBus::new();

    let mut registry = RuleRegistry::new();
    registry.insert(Rule::new(
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
    ));

    let rt = runtime(&handle, &session, &bus);
    let evaluation = evaluate(&rt, &registry, &principal, &Id::from_raw("high-temp"))
        .await
        .expect("evaluate");

    assert!(!evaluation.decision.fired, "avg 1.5 < 15 must not fire");
    // A non-firing decision is still recorded — the insight is the firing record,
    // fired or not, so a downstream consumer sees every evaluation.
    let exists: Option<i64> = handle
        .raw()
        .query("SELECT VALUE count() FROM record WHERE id = $id GROUP ALL")
        .bind(("id", surreal_id(&evaluation.insight_id)))
        .await
        .expect("read insight")
        .take(0)
        .expect("decode");
    assert_eq!(exists.unwrap_or(0), 1);
}

/// Build a SurrealDB record-id thing for the `record` table from a rubix id.
fn surreal_id(id: &Id) -> surrealdb::types::RecordId {
    surrealdb::types::RecordId::new("record", id.as_str())
}
