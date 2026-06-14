//! Integration: `redo` re-applies the forward change after an undo.
//!
//! Exercises the SCOPE "Undo/redo" redo path against kv-mem: after a definition
//! update is undone (prior value restored), `redo` replays the forward change
//! back through the WS-05 gate — the record holds the revised value again and the
//! redo, like the undo, produces its own audit row.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Capability, Change, Command, RecordKind, UndoStore, apply, create_grant, push_change, redo, undo,
};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn operator(subject: &str) -> Principal {
    Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Operator)
}

async fn content(handle: &rubix_store::StoreHandle, target: &Id) -> Option<serde_json::Value> {
    rubix_core::read_record(handle.raw(), target)
        .await
        .expect("read record")
        .map(|record| record.content)
}

#[tokio::test]
async fn redo_reapplies_the_forward_change() {
    let handle = open_gate_store("redo_forward").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("rule-1");
    let original = serde_json::json!({ "threshold": 10 });
    let revised = serde_json::json!({ "threshold": 20 });

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

    let applied = apply(
        handle.raw(),
        &Command::new(
            actor.clone(),
            Capability::RuleInvoke,
            target.clone(),
            Change::Update(revised.clone()),
        ),
        None,
    )
    .await
    .expect("update");

    let mut store = UndoStore::new();
    push_change(
        &mut store,
        &actor,
        Capability::RuleInvoke,
        &target,
        RecordKind::Definition,
        &Change::Update(revised.clone()),
        &applied,
    )
    .expect("push");

    // Undo back to the original, then redo forward to the revised value.
    undo(handle.raw(), &mut store, &actor, &target)
        .await
        .expect("undo");
    assert_eq!(content(&handle, &target).await, Some(original));

    redo(handle.raw(), &mut store, &actor, &target)
        .await
        .expect("redo");
    assert_eq!(content(&handle, &target).await, Some(revised));

    // After redo, the entry is re-armed for undo and the redo stack is empty.
    assert!(
        redo(handle.raw(), &mut store, &actor, &target)
            .await
            .is_err(),
        "redo stack is empty after re-applying"
    );
    assert!(
        undo(handle.raw(), &mut store, &actor, &target)
            .await
            .is_ok(),
        "the redone change is undoable again"
    );
}
