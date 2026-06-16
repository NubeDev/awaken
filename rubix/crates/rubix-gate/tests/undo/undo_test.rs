//! Integration: editing a definition then calling `undo` restores the prior
//! value, and the undo itself produces an audit row.
//!
//! Exercises the SCOPE "Undo/redo" boundary end to end against kv-mem: a granted
//! principal updates a definition record through the WS-05 gate, the change is
//! pushed onto the undo stack, and `undo` replays the inverse back through the
//! gate — the record holds the prior content again and a *new* audit row exists
//! for the undo's reversing action, threaded by the same correlation id.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Capability, Change, Command, RecordKind, UndoStore, apply, create_grant, push_change, undo,
};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn operator(subject: &str) -> Principal {
    Principal::new(
        Id::from_raw(subject),
        NS,
        PrincipalKind::User,
        Role::Operator,
    )
}

async fn content(handle: &rubix_store::StoreHandle, target: &Id) -> Option<serde_json::Value> {
    rubix_core::read_record(handle.raw(), target)
        .await
        .expect("read record")
        .map(|record| record.content)
}

async fn count_action_rows(handle: &rubix_store::StoreHandle, subject: &str, action: &str) -> i64 {
    let mut response = handle
        .raw()
        .query(
            "SELECT VALUE count() FROM audit \
             WHERE subject = $subject AND action = $action GROUP ALL",
        )
        .bind(("subject", subject.to_owned()))
        .bind(("action", action.to_owned()))
        .await
        .expect("count audit");
    let count: Option<i64> = response.take(0).expect("decode count");
    count.unwrap_or(0)
}

#[tokio::test]
async fn undo_restores_the_prior_value_and_audits_the_reversal() {
    let handle = open_gate_store("undo_restore").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("dash-1");
    let original = serde_json::json!({ "title": "first" });
    let revised = serde_json::json!({ "title": "second" });

    // Create the definition, then update it through the gate.
    let create = Command::new(
        actor.clone(),
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(original.clone()),
    );
    apply(handle.raw(), &create, None).await.expect("create");

    let update = Command::new(
        actor.clone(),
        Capability::RuleInvoke,
        target.clone(),
        Change::Update(revised.clone()),
    );
    let applied = apply(handle.raw(), &update, None).await.expect("update");
    assert_eq!(content(&handle, &target).await, Some(revised.clone()));

    // Push the definition update onto the undo stack.
    let mut store = UndoStore::new();
    push_change(
        &mut store,
        &actor,
        Capability::RuleInvoke,
        &target,
        RecordKind::Definition,
        &Change::Update(revised),
        &applied,
    )
    .expect("push");

    let audits_before_undo = count_action_rows(&handle, "alice", "update").await;

    // Undo: the inverse restores the prior content, replayed through the gate.
    let undone = undo(handle.raw(), &mut store, &actor, &target)
        .await
        .expect("undo");

    // The record holds the prior value again.
    assert_eq!(content(&handle, &target).await, Some(original));

    // The undo produced a new audit row (it went through the gate) carrying the
    // original correlation id — the chain stays threaded.
    assert_eq!(
        count_action_rows(&handle, "alice", "update").await,
        audits_before_undo + 1,
        "undo writes its own audit row"
    );
    assert_eq!(undone.correlation_id, applied.correlation_id);
}

#[tokio::test]
async fn undo_with_no_history_returns_nothing_to_reverse() {
    let handle = open_gate_store("undo_empty").await;
    let actor = operator("bob");
    let mut store = UndoStore::new();

    let err = undo(handle.raw(), &mut store, &actor, &Id::from_raw("missing"))
        .await
        .expect_err("empty stack");
    assert!(matches!(err, rubix_gate::GateError::NothingToReverse(_)));
}
