//! Integration: the undo boundary refuses data-plane and audit records.
//!
//! Exercises the SCOPE "Undo/redo" boundary against kv-mem: undo covers
//! user-facing definitions only. A change applied to a data-plane record (a
//! reading) and a change classified as the audit log are both refused at the push
//! boundary — nothing lands on the undo stack, so no reversal is possible and the
//! data plane / audit log stay out of undo's reach.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Capability, Change, Command, GateError, RecordKind, UndoStore, apply, create_grant,
    push_change, undo,
};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn operator(subject: &str) -> Principal {
    Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Operator)
}

#[tokio::test]
async fn a_data_plane_record_cannot_be_pushed_or_undone() {
    let handle = open_gate_store("undo_data_plane").await;
    let actor = operator("ingestor");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    // A reading is applied through the gate like any record, but it is data-plane.
    let target = Id::from_raw("reading-1");
    let applied = apply(
        handle.raw(),
        &Command::new(
            actor.clone(),
            Capability::RuleInvoke,
            target.clone(),
            Change::Create(serde_json::json!({ "temp": 21 })),
        ),
        None,
    )
    .await
    .expect("create reading");

    // Pushing it as data-plane is refused at the boundary.
    let mut store = UndoStore::new();
    let err = push_change(
        &mut store,
        &actor,
        Capability::RuleInvoke,
        &target,
        RecordKind::DataPlane,
        &Change::Create(serde_json::json!({ "temp": 21 })),
        &applied,
    )
    .expect_err("data-plane refused");
    assert!(matches!(err, GateError::UndoBoundary(_)));

    // Nothing landed on the stack, so there is nothing to undo.
    let undo_err = undo(handle.raw(), &mut store, &actor, &target)
        .await
        .expect_err("no undo history for a refused data-plane change");
    assert!(matches!(undo_err, GateError::NothingToReverse(_)));
}

#[tokio::test]
async fn an_audit_record_cannot_be_pushed() {
    let handle = open_gate_store("undo_audit").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("audit-row-1");
    let applied = apply(
        handle.raw(),
        &Command::new(
            actor.clone(),
            Capability::RuleInvoke,
            target.clone(),
            Change::Create(serde_json::json!({ "who": "alice" })),
        ),
        None,
    )
    .await
    .expect("create");

    let mut store = UndoStore::new();
    let err = push_change(
        &mut store,
        &actor,
        Capability::RuleInvoke,
        &target,
        RecordKind::Audit,
        &Change::Delete,
        &applied,
    )
    .expect_err("audit refused");
    assert!(matches!(err, GateError::UndoBoundary(_)));
}
