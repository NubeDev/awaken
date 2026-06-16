//! Integration: the per-evaluation span tree records the full "why".
//!
//! The Rhai engine emits a span tree per evaluation — which sub-rules ran, the
//! values seen, and the decision — the deterministic answer to "why did this
//! fire" (`rubix/docs/SCOPE.md`, "Tracing"). This retrieves the tree by the
//! evaluation correlation id and asserts it nests the sub-rule under the parent
//! and carries the inputs and decisions, with one trace id across every node
//! (contract #3, `rubix/STACK-DEISGN.md`).

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_bus::ControlBus;
use rubix_core::Id;
use rubix_query::{CanonicalTable, Grain};
use rubix_rules::{Aggregate, Binding, Rule, RuleRegistry, RuleRuntime, evaluate};
use rubix_trace::{SampleRate, assemble_trace};

use fixture::open::{granted_session, open_rules_store, seed_reading};

#[tokio::test]
async fn the_span_tree_nests_the_subrule_and_shows_the_decision() {
    let database = "rule_span";
    let handle = open_rules_store(database).await;

    seed_reading(&handle, "temperature", 0, 30.0).await;
    seed_reading(&handle, "humidity", 0, 80.0).await;

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

    let forest = assemble_trace(handle.raw(), &evaluation.correlation_id)
        .await
        .expect("assemble trace");

    assert_eq!(forest.len(), 1, "one root span for the top rule");
    let root = &forest[0];
    assert_eq!(root.span.attributes["rule"], "hot-and-humid");
    assert_eq!(root.span.attributes["inputs"]["humidity"], 80.0);
    assert_eq!(root.span.attributes["decision"]["fired"], true);

    assert_eq!(
        root.children.len(),
        1,
        "the sub-rule nests under the parent"
    );
    let child_node = &root.children[0];
    assert_eq!(child_node.span.attributes["rule"], "hot");
    assert_eq!(child_node.span.attributes["inputs"]["temp"], 30.0);
    assert_eq!(child_node.span.attributes["decision"]["fired"], true);

    // Every node shares the one evaluation correlation id (contract #3).
    assert_eq!(root.span.trace_id, evaluation.correlation_id);
    assert_eq!(child_node.span.trace_id, evaluation.correlation_id);
}

#[tokio::test]
async fn a_leaf_rule_produces_a_single_root_span() {
    let database = "rule_span_leaf";
    let handle = open_rules_store(database).await;
    seed_reading(&handle, "temperature", 0, 30.0).await;

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

    let forest = assemble_trace(handle.raw(), &evaluation.correlation_id)
        .await
        .expect("assemble trace");
    assert_eq!(forest.len(), 1);
    assert!(
        forest[0].children.is_empty(),
        "a leaf rule has no sub-spans"
    );
    assert_eq!(forest[0].span.attributes["rule"], "high-temp");
}
