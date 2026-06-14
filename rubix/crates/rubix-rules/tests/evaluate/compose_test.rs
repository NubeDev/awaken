//! Integration: a composed rule (rule-calls-rule) works offline.
//!
//! Rules are composable — a rule invokes another rule (`rubix/docs/SCOPE.md`,
//! "Rhai — rules and insights"). This proves a parent rule's decision reflects a
//! child rule it invokes, with no cloud dependency, and that an undeclared
//! sub-rule fails closed rather than silently producing a partial decision.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_bus::ControlBus;
use rubix_core::Id;
use rubix_query::{CanonicalTable, Grain};
use rubix_rules::{Aggregate, Binding, Rule, RuleRegistry, RuleRuntime, evaluate};
use rubix_trace::SampleRate;

use fixture::open::{granted_session, open_rules_store, seed_reading};

#[tokio::test]
async fn a_parent_decision_reflects_the_child_it_invokes() {
    let database = "rule_compose";
    let handle = open_rules_store(database).await;

    // Child reads the temperature window; parent reads humidity and combines the
    // child's decision with its own.
    seed_reading(&handle, "temperature", 0, 30.0).await; // child: 30 > 25 -> fires (1.0)
    seed_reading(&handle, "humidity", 0, 80.0).await; // parent: 80 > 70 AND child fired

    let (principal, session) = granted_session(&handle, database, "alice").await;
    let bus = ControlBus::new();

    let child = Rule::new(
        Id::from_raw("hot"),
        "temp > 25.0",
        vec![Binding::new(
            "temp",
            CanonicalTable::Records,
            "temperature",
            Grain::Minute,
            Aggregate::Avg,
        )],
        "hot",
    );
    let parent = Rule::new(
        Id::from_raw("hot-and-humid"),
        // `invoke("hot")` returns the child's decision value (1.0 when it fired).
        r#"#{ fired: humidity > 70.0 && invoke("hot") > 0.5, value: humidity, reason: "hot and humid" }"#,
        vec![Binding::new(
            "humidity",
            CanonicalTable::Records,
            "humidity",
            Grain::Minute,
            Aggregate::Avg,
        )],
        "hot-and-humid",
    )
    .composing(vec![Id::from_raw("hot")]);

    let mut registry = RuleRegistry::new();
    registry.insert(child);
    registry.insert(parent);

    let rt = RuleRuntime {
        gate_db: handle.raw(),
        session: session.connection(),
        trace_db: handle.raw(),
        bus: &bus,
        sample: SampleRate::new(0.0),
    };
    let evaluation = evaluate(&rt, &registry, &principal, &Id::from_raw("hot-and-humid"))
        .await
        .expect("evaluate composed rule");

    assert!(
        evaluation.decision.fired,
        "humidity 80 > 70 and child fired -> parent fires"
    );
    assert_eq!(evaluation.decision.value, 80.0);
}

#[tokio::test]
async fn a_parent_does_not_fire_when_its_child_does_not() {
    let database = "rule_compose_quiet";
    let handle = open_rules_store(database).await;

    seed_reading(&handle, "temperature", 0, 10.0).await; // child: 10 > 25 -> no fire (0.0)
    seed_reading(&handle, "humidity", 0, 80.0).await; // parent humidity high, but child gates it

    let (principal, session) = granted_session(&handle, database, "bob").await;
    let bus = ControlBus::new();

    let child = Rule::new(
        Id::from_raw("hot"),
        "temp > 25.0",
        vec![Binding::new(
            "temp",
            CanonicalTable::Records,
            "temperature",
            Grain::Minute,
            Aggregate::Avg,
        )],
        "hot",
    );
    let parent = Rule::new(
        Id::from_raw("hot-and-humid"),
        r#"humidity > 70.0 && invoke("hot") > 0.5"#,
        vec![Binding::new(
            "humidity",
            CanonicalTable::Records,
            "humidity",
            Grain::Minute,
            Aggregate::Avg,
        )],
        "hot-and-humid",
    )
    .composing(vec![Id::from_raw("hot")]);

    let mut registry = RuleRegistry::new();
    registry.insert(child);
    registry.insert(parent);

    let rt = RuleRuntime {
        gate_db: handle.raw(),
        session: session.connection(),
        trace_db: handle.raw(),
        bus: &bus,
        sample: SampleRate::new(0.0),
    };
    let evaluation = evaluate(&rt, &registry, &principal, &Id::from_raw("hot-and-humid"))
        .await
        .expect("evaluate");
    assert!(!evaluation.decision.fired, "the child gates the parent off");
}

#[tokio::test]
async fn an_undeclared_subrule_fails_closed() {
    let database = "rule_compose_missing";
    let handle = open_rules_store(database).await;
    seed_reading(&handle, "humidity", 0, 80.0).await;

    let (principal, session) = granted_session(&handle, database, "carol").await;
    let bus = ControlBus::new();

    // Declares a sub-rule id that is not in the registry.
    let parent = Rule::new(
        Id::from_raw("orphan-parent"),
        r#"invoke("ghost") > 0.5"#,
        Vec::new(),
        "orphan",
    )
    .composing(vec![Id::from_raw("ghost")]);

    let mut registry = RuleRegistry::new();
    registry.insert(parent);

    let rt = RuleRuntime {
        gate_db: handle.raw(),
        session: session.connection(),
        trace_db: handle.raw(),
        bus: &bus,
        sample: SampleRate::new(0.0),
    };
    let result = evaluate(&rt, &registry, &principal, &Id::from_raw("orphan-parent")).await;
    assert!(result.is_err(), "an unresolved sub-rule must fail the eval");
}
