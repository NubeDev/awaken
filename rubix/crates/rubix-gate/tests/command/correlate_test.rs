//! Integration: the correlation id is carried onto the audit row.
//!
//! Contract #3 (`rubix/STACK-DEISGN.md`): the correlation id minted/carried at
//! the gate is stamped onto the audit record — the thread a reader pivots on. A
//! command applied with a carried id stamps that exact id; a command applied
//! without one mints a fresh id and stamps that.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{CorrelationId, Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, Change, Command, apply, create_grant};

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

async fn stored_correlation_id(handle: &rubix_store::StoreHandle, target: &str) -> Option<String> {
    let mut response = handle
        .raw()
        .query("SELECT correlation_id FROM audit WHERE target = $target")
        .bind(("target", target.to_owned()))
        .await
        .expect("read audit");
    response.take("correlation_id").expect("decode corr")
}

#[tokio::test]
async fn a_carried_correlation_id_is_stamped_onto_the_audit_row() {
    let handle = open_gate_store("cmd_corr_carry").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("rec-carry");
    let command = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "k": "v" })),
    );
    let carried = CorrelationId::carry("corr-upstream");

    let applied = apply(handle.raw(), &command, Some(carried.clone()))
        .await
        .expect("apply");

    assert_eq!(applied.correlation_id, carried);
    assert_eq!(
        stored_correlation_id(&handle, "rec-carry").await.as_deref(),
        Some("corr-upstream")
    );
}

#[tokio::test]
async fn a_principal_command_mints_one_id_and_stamps_it() {
    let handle = open_gate_store("cmd_corr_mint").await;
    let actor = operator("alice");
    create_grant(handle.raw(), &admin(), &actor, Capability::RuleInvoke)
        .await
        .expect("grant");

    let target = Id::from_raw("rec-mint");
    let command = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "k": "v" })),
    );

    let applied = apply(handle.raw(), &command, None).await.expect("apply");

    assert_eq!(
        stored_correlation_id(&handle, "rec-mint").await.as_deref(),
        Some(applied.correlation_id.as_str()),
        "the minted id is the one stamped onto audit"
    );
}
