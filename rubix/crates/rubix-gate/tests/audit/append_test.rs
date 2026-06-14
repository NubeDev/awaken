//! Integration: each mutating command appends one audit row with full detail.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Audit log"):
//! the audit row records principal, namespace, action, target, before/after, and
//! the correlation id. A create then a delete on the same target produce two
//! rows whose action and before/after summaries reflect the two mutations.

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
async fn create_then_delete_append_two_audit_rows() {
    let handle = open_gate_store("audit_append").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("rec-a");
    let content = serde_json::json!({ "temp": 21 });

    let create = Command::new(
        actor.clone(),
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(content.clone()),
    );
    apply(handle.raw(), &create, None).await.expect("create");

    let delete = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Delete,
    );
    apply(handle.raw(), &delete, None).await.expect("delete");

    // Two rows, ordered by action, carry the expected namespace and summaries.
    let mut response = handle
        .raw()
        .query(
            "SELECT namespace, action, target, before, after FROM audit \
             WHERE target = $target ORDER BY action",
        )
        .bind(("target", target.to_string()))
        .await
        .expect("read audit");
    let actions: Vec<String> = response.take("action").expect("actions");
    let namespaces: Vec<String> = response.take("namespace").expect("namespaces");
    assert_eq!(actions, vec!["create".to_owned(), "delete".to_owned()]);
    assert!(namespaces.iter().all(|ns| ns == NS));

    // The delete's before-image is the content the create wrote.
    let mut delete_resp = handle
        .raw()
        .query("SELECT before FROM audit WHERE target = $target AND action = 'delete'")
        .bind(("target", target.to_string()))
        .await
        .expect("read delete audit");
    let before: Option<serde_json::Value> = delete_resp.take("before").expect("before");
    assert_eq!(before, Some(content));
}
