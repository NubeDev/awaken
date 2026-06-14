//! Integration: the gate captures the before-image atomically with the write.
//!
//! Contract #1/#4 (`rubix/STACK-DEISGN.md`): an update through the gate captures
//! the prior content via SurrealDB `RETURN BEFORE`, in the same round trip as the
//! write — so the audit/undo before-summary is the real prior value, not a
//! separate read. Asserts the returned capture's `before` is the original content
//! and `after` is the new content, and that the audit row stored that before.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, Change, Command, apply, create_grant};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn operator(subject: &str) -> Principal {
    Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Operator)
}

#[tokio::test]
async fn an_update_captures_the_prior_content_atomically() {
    let handle = open_gate_store("cmd_capture").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("rec-7");
    let original = serde_json::json!({ "temp": 21 });
    let revised = serde_json::json!({ "temp": 25 });

    let create = Command::new(
        actor.clone(),
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(original.clone()),
    );
    apply(handle.raw(), &create, None).await.expect("create");

    let update = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Update(revised.clone()),
    );
    let applied = apply(handle.raw(), &update, None).await.expect("update");

    // The capture carries the real prior value and the new value.
    assert_eq!(applied.captured.before, Some(original.clone()));
    assert_eq!(applied.captured.after, Some(revised.clone()));

    // The update's audit row stored that same before-image.
    let mut audit_resp = handle
        .raw()
        .query(
            "SELECT before FROM audit \
             WHERE subject = $subject AND action = 'update'",
        )
        .bind(("subject", "alice".to_owned()))
        .await
        .expect("read audit");
    let before: Option<serde_json::Value> = audit_resp.take("before").expect("before");
    assert_eq!(before, Some(original));
}
