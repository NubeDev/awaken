//! Integration: undoing a delete recreates the record from the captured prior.
//!
//! Exercises `ChangeRecord`'s inverse over a real gate capture against kv-mem: a
//! definition delete captures the prior content via `RETURN BEFORE`, the inverse
//! is a create of that content, and `undo` replays it through the gate so the
//! deleted definition reappears with its original value — the before/after
//! snapshot makes the reversal cheap (SCOPE "Undo/redo").

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

#[tokio::test]
async fn undoing_a_delete_recreates_the_prior_content() {
    let handle = open_gate_store("undo_delete").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("tag-def-1");
    let original = serde_json::json!({ "name": "critical" });

    apply(
        handle.raw(),
        &Command::new(
            actor.clone(),
            Capability::RuleInvoke,
            target.clone(),
            Change::Create(original.clone()),
        ),
        None,
    )
    .await
    .expect("create");

    // Delete the definition through the gate — the capture holds the prior content.
    let deleted = apply(
        handle.raw(),
        &Command::new(
            actor.clone(),
            Capability::RuleInvoke,
            target.clone(),
            Change::Delete,
        ),
        None,
    )
    .await
    .expect("delete");
    assert_eq!(deleted.captured.before, Some(original.clone()));
    assert_eq!(content(&handle, &target).await, None);

    // Push the delete; its inverse is a create of the captured prior content.
    let mut store = UndoStore::new();
    push_change(
        &mut store,
        &actor,
        Capability::RuleInvoke,
        &target,
        RecordKind::Definition,
        &Change::Delete,
        &deleted,
    )
    .expect("push");

    // Undo recreates the deleted definition with its original value.
    undo(handle.raw(), &mut store, &actor, &target)
        .await
        .expect("undo");
    assert_eq!(content(&handle, &target).await, Some(original));
}
