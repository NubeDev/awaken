//! Integration: a command driven through the gate applies the record change and
//! writes an immutable audit row carrying its correlation id.
//!
//! Exercises contract #1/#3/#4 (`rubix/STACK-DEISGN.md`) end to end against a
//! live kv-mem SurrealDB: a granted principal's create lands the record, an audit
//! row is appended with the principal, action, before/after, and the same
//! correlation id `apply` returned, and a principal without the grant is refused
//! before any write or audit row exists.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, Change, Command, GateError, apply, create_grant};

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

async fn count_audit_rows(handle: &rubix_store::StoreHandle, subject: &str) -> i64 {
    let mut response = handle
        .raw()
        .query("SELECT VALUE count() FROM audit WHERE subject = $subject GROUP ALL")
        .bind(("subject", subject.to_owned()))
        .await
        .expect("count audit");
    let count: Option<i64> = response.take(0).expect("decode count");
    count.unwrap_or(0)
}

#[tokio::test]
async fn a_granted_command_applies_the_record_and_audits_it() {
    let handle = open_gate_store("cmd_apply").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("rec-1");
    let command = Command::new(
        actor.clone(),
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "temp": 21 })),
    );

    let applied = apply(handle.raw(), &command, None).await.expect("apply");

    // The record landed.
    let landed: Option<serde_json::Value> = rubix_core::read_record(handle.raw(), &target)
        .await
        .expect("read record")
        .map(|record| record.content);
    assert_eq!(landed, Some(serde_json::json!({ "temp": 21 })));

    // Exactly one audit row exists, carrying the correlation id apply returned.
    assert_eq!(count_audit_rows(&handle, "alice").await, 1);
    let mut audit_resp = handle
        .raw()
        .query("SELECT action, target, after, correlation_id FROM audit WHERE subject = $subject")
        .bind(("subject", "alice".to_owned()))
        .await
        .expect("read audit");
    let action: Option<String> = audit_resp.take("action").expect("action");
    let stored_corr: Option<String> = audit_resp.take("correlation_id").expect("corr");
    assert_eq!(action.as_deref(), Some("create"));
    assert_eq!(
        stored_corr.as_deref(),
        Some(applied.correlation_id.as_str())
    );
}

#[tokio::test]
async fn an_ungranted_command_is_denied_before_any_write() {
    let handle = open_gate_store("cmd_denied").await;
    let actor = operator("mallory");

    let target = Id::from_raw("rec-x");
    let command = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "temp": 99 })),
    );

    let err = apply(handle.raw(), &command, None)
        .await
        .expect_err("ungranted command must be denied");
    assert!(matches!(err, GateError::CommandDenied(_)));

    // No record and no audit row were written.
    let landed = rubix_core::read_record(handle.raw(), &target)
        .await
        .expect("read record");
    assert!(landed.is_none(), "no record on a denied command");
    assert_eq!(count_audit_rows(&handle, "mallory").await, 0);
}
