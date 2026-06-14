//! Integration: a non-system principal cannot UPDATE or DELETE an audit row.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Audit log"):
//! audit immutability is enforced by SurrealDB table permissions, not app code.
//! A principal on a gate-issued scoped session (the same session reads run on)
//! issues an UPDATE and a DELETE against an audit row; the engine's
//! `FOR update, delete NONE` permission refuses both, so the row's content is
//! unchanged and the row still exists.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Capability, Change, Command, PrincipalToken, apply, authenticate, create_grant,
    issue_scoped_session, provision_principal,
};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn operator(subject: &str) -> Principal {
    Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Operator)
}

#[tokio::test]
async fn a_scoped_principal_cannot_mutate_or_delete_an_audit_row() {
    let database = "audit_immutable";
    let handle = open_gate_store(database).await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    // Produce an audit row via a real command.
    let command = Command::new(
        actor.clone(),
        Capability::RuleInvoke,
        Id::from_raw("rec-1"),
        Change::Create(serde_json::json!({ "temp": 21 })),
    );
    apply(handle.raw(), &command, None).await.expect("apply");

    // Provision the principal and issue its scoped session (a non-system caller).
    provision_principal(handle.raw(), &actor, "pw")
        .await
        .expect("provision");
    let token = PrincipalToken::new("alice", "pw");
    let resolved = authenticate(handle.raw(), &token).await.expect("authenticate");
    let session = issue_scoped_session(handle.raw(), NS, database, resolved, &token)
        .await
        .expect("issue session");

    // Attempt to tamper: UPDATE then DELETE every audit row on the scoped session.
    session
        .connection()
        .query("UPDATE audit SET action = 'tampered'")
        .query("DELETE audit")
        .await
        .expect("queries run")
        .check()
        .expect("permission filtering does not error");

    // The original audit row is intact on the owner handle.
    let mut response = handle
        .raw()
        .query("SELECT VALUE action FROM audit WHERE subject = 'alice'")
        .await
        .expect("read audit");
    let actions: Vec<String> = response.take(0).expect("decode actions");
    assert_eq!(actions, vec!["create".to_owned()], "audit row is immutable");
}
