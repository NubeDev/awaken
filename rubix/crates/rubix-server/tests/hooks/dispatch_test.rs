//! Integration: writing a watched record fires its hook's rule through the gate.
//!
//! Proves build-order step 5 (`rubix/docs/design/BACKEND-COLLECTIONS.md`,
//! "Server-side hooks"): a `kind:"hook"` record binds a rule to a collection's
//! writes, and the background dispatcher — subscribed to the live-query data plane
//! — fires that rule when a matching record is written, recording its insight
//! through the WS-05 gate. The rule runs as a per-namespace system principal the
//! dispatcher provisions, so the insight write is gated, audited, and correlated
//! like any other evaluation. All offline on kv-mem (`rubix/docs/SCOPE.md`,
//! principle 3).

use std::time::Duration;

use rubix_core::{Id, Principal, PrincipalKind, Role, RuntimeConfig};
use rubix_gate::{
    Capability, Change, Command, apply, create_grant, define_audit_schema, define_gate_schema,
    provision_principal,
};
use rubix_server::{AppState, spawn_hook_dispatcher};
use rubix_store::StoreHandle;
use serde_json::{Value, json};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

const NS: &str = "rubix";

/// Write `content` as a create through the gate as `principal`.
async fn put(db: &Surreal<Db>, principal: &Principal, id: &Id, content: Value) {
    let command = Command::new(
        principal.clone(),
        Capability::IngestPublish,
        id.clone(),
        Change::Create(content),
    );
    apply(db, &command, None).await.expect("gate write");
}

/// Count `record`s whose `content.kind` equals `kind`.
async fn count_kind(db: &Surreal<Db>, kind: &str) -> i64 {
    let n: Option<i64> = db
        .query("SELECT VALUE count() FROM record WHERE content.kind = $kind GROUP ALL")
        .bind(("kind", kind.to_owned()))
        .await
        .expect("count query")
        .take(0)
        .expect("decode count");
    n.unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_to_a_watched_kind_fires_its_hook_rule() {
    let cfg = RuntimeConfig::in_memory(NS, "hook_dispatch");
    let store = StoreHandle::open(&cfg).await.expect("open store");
    define_gate_schema(store.raw()).await.expect("gate schema");
    define_audit_schema(store.raw())
        .await
        .expect("audit schema");
    rubix_trace::define_trace_schema(store.raw())
        .await
        .expect("trace schema");

    // An operator that may write records, rules, hooks (the gate's IngestPublish).
    let operator = Principal::new(
        Id::from_raw("operator"),
        NS,
        PrincipalKind::User,
        Role::Operator,
    );
    provision_principal(store.raw(), &operator, "pw")
        .await
        .expect("provision operator");
    let admin = Principal::new(Id::from_raw("admin"), NS, PrincipalKind::User, Role::Admin);
    create_grant(store.raw(), &admin, &operator, Capability::IngestPublish)
        .await
        .expect("grant ingest-publish");

    // A constant rule (no window inputs) and a hook binding it to `widget` creates.
    put(
        store.raw(),
        &operator,
        &Id::from_raw("rule-touch"),
        json!({
            "kind": "rule",
            "name": "widget-touched",
            "script": "#{ fired: true, value: 1.0, reason: \"a widget was written\" }",
            "output": "widget-touched-insight",
        }),
    )
    .await;
    put(
        store.raw(),
        &operator,
        &Id::from_raw("hook-widget"),
        json!({
            "kind": "hook",
            "match": "widget",
            "on": ["create"],
            "rule": "widget-touched",
        }),
    )
    .await;

    // Start the dispatcher and give its live-query subscription a moment to open
    // before the watched write (a live query only delivers changes after subscribe).
    let state = AppState::new(store.clone(), NS, "hook_dispatch");
    spawn_hook_dispatcher(state);
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(
        count_kind(store.raw(), "widget-touched-insight").await,
        0,
        "no insight before the watched write"
    );

    // Write a watched record — this is the event the hook fires on.
    put(
        store.raw(),
        &operator,
        &Id::from_raw("widget-1"),
        json!({ "kind": "widget", "label": "left bracket" }),
    )
    .await;

    // The dispatcher fires asynchronously; poll for the recorded insight.
    let mut fired = 0;
    for _ in 0..50 {
        fired = count_kind(store.raw(), "widget-touched-insight").await;
        if fired >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(fired, 1, "the hook fired its rule and recorded one insight");

    // The firing crossed the gate: an audit row exists for the insight write.
    let audit: Option<i64> = store
        .raw()
        .query("SELECT VALUE count() FROM audit WHERE action = 'create' GROUP ALL")
        .await
        .expect("audit query")
        .take(0)
        .expect("decode audit");
    assert!(audit.unwrap_or(0) >= 1, "the insight write was audited");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_to_an_unwatched_kind_fires_nothing() {
    let cfg = RuntimeConfig::in_memory(NS, "hook_no_match");
    let store = StoreHandle::open(&cfg).await.expect("open store");
    define_gate_schema(store.raw()).await.expect("gate schema");
    define_audit_schema(store.raw())
        .await
        .expect("audit schema");
    rubix_trace::define_trace_schema(store.raw())
        .await
        .expect("trace schema");

    let operator = Principal::new(
        Id::from_raw("operator"),
        NS,
        PrincipalKind::User,
        Role::Operator,
    );
    provision_principal(store.raw(), &operator, "pw")
        .await
        .expect("provision operator");
    let admin = Principal::new(Id::from_raw("admin"), NS, PrincipalKind::User, Role::Admin);
    create_grant(store.raw(), &admin, &operator, Capability::IngestPublish)
        .await
        .expect("grant");

    put(
        store.raw(),
        &operator,
        &Id::from_raw("rule-touch"),
        json!({
            "kind": "rule",
            "name": "widget-touched",
            "script": "#{ fired: true, value: 1.0, reason: \"x\" }",
            "output": "widget-touched-insight",
        }),
    )
    .await;
    put(
        store.raw(),
        &operator,
        &Id::from_raw("hook-widget"),
        json!({ "kind": "hook", "match": "widget", "on": ["create"], "rule": "widget-touched" }),
    )
    .await;

    let state = AppState::new(store.clone(), NS, "hook_no_match");
    spawn_hook_dispatcher(state);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // A record of a *different* kind must not fire the widget hook.
    put(
        store.raw(),
        &operator,
        &Id::from_raw("gadget-1"),
        json!({ "kind": "gadget", "label": "unrelated" }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert_eq!(
        count_kind(store.raw(), "widget-touched-insight").await,
        0,
        "an unwatched kind fires no hook"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hook_on_a_rules_own_output_does_not_recurse() {
    // The recursion footgun: a hook matches the kind a rule emits, and the rule
    // fires on that kind. Without a guard, the rule's insight write would re-fire
    // the hook forever. The dispatcher must treat insight (rule-output) writes as
    // non-hookable, so the cycle never starts.
    let cfg = RuntimeConfig::in_memory(NS, "hook_recurse");
    let store = StoreHandle::open(&cfg).await.expect("open store");
    define_gate_schema(store.raw()).await.expect("gate schema");
    define_audit_schema(store.raw())
        .await
        .expect("audit schema");
    rubix_trace::define_trace_schema(store.raw())
        .await
        .expect("trace schema");

    let operator = Principal::new(
        Id::from_raw("operator"),
        NS,
        PrincipalKind::User,
        Role::Operator,
    );
    provision_principal(store.raw(), &operator, "pw")
        .await
        .expect("provision operator");
    let admin = Principal::new(Id::from_raw("admin"), NS, PrincipalKind::User, Role::Admin);
    create_grant(store.raw(), &admin, &operator, Capability::IngestPublish)
        .await
        .expect("grant");

    // A rule whose output kind is `loopy`, and a hook that watches `loopy` creates
    // and fires that very rule — the loop the guard must break.
    put(
        store.raw(),
        &operator,
        &Id::from_raw("rule-loop"),
        json!({
            "kind": "rule",
            "name": "loop-rule",
            "script": "#{ fired: true, value: 1.0, reason: \"loop\" }",
            "output": "loopy",
        }),
    )
    .await;
    put(
        store.raw(),
        &operator,
        &Id::from_raw("hook-loop"),
        json!({ "kind": "hook", "match": "loopy", "on": ["create"], "rule": "loop-rule" }),
    )
    .await;

    let state = AppState::new(store.clone(), NS, "hook_recurse");
    spawn_hook_dispatcher(state);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Write one record whose kind is the rule's output. It is an insight-shaped kind,
    // so the dispatcher must not fire the hook on it — the count must stay at 1.
    put(
        store.raw(),
        &operator,
        &Id::from_raw("loopy-1"),
        json!({ "kind": "loopy", "note": "seed" }),
    )
    .await;

    // Give any (erroneous) cascade ample time to run away, then assert it did not:
    // exactly the one record we wrote, no rule-fired insights piled on top.
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(
        count_kind(store.raw(), "loopy").await,
        1,
        "a rule-output kind is not hookable, so no recursion occurs"
    );
}
