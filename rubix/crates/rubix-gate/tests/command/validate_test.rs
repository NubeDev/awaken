//! Integration: the gate's validate step enforces a collection's contract on the
//! one mutation chokepoint.
//!
//! Exercises `rubix/docs/design/BACKEND-COLLECTIONS.md` end to end against a live
//! kv-mem SurrealDB: a write whose `kind` matches a registered collection is
//! validated (valid content lands, invalid content is refused before any write),
//! an unknown `kind` is admitted while the namespace is fail-open and rejected
//! once it is strict, and a partial update is validated against the record it
//! produces, not the patch alone.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{COLLECTION_KIND, Id, NAMESPACE_SETTINGS_KIND, Principal, PrincipalKind, Record, Role, create_record, read_record};
use rubix_gate::{Capability, Change, Command, GateError, apply, create_grant};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn operator(subject: &str) -> Principal {
    Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Operator)
}

/// Register the `site` collection (key+name required, area number) in `NS`.
async fn register_site_collection(handle: &rubix_store::StoreHandle) {
    let def = Record::new(
        NS,
        serde_json::json!({
            "kind": COLLECTION_KIND,
            "name": "site",
            "schema": [
                { "name": "key",  "type": "text",   "required": true },
                { "name": "name", "type": "text",   "required": true },
                { "name": "area", "type": "number" }
            ]
        }),
    );
    create_record(handle.raw(), &def).await.expect("register collection");
}

async fn grant(handle: &rubix_store::StoreHandle, actor: &Principal) {
    create_grant(handle.raw(), &admin(), actor, Capability::RuleInvoke)
        .await
        .expect("grant");
}

#[tokio::test]
async fn valid_content_for_a_registered_collection_lands() {
    let handle = open_gate_store("validate_valid").await;
    register_site_collection(&handle).await;
    let actor = operator("alice");
    grant(&handle, &actor).await;

    let target = Id::from_raw("site-1");
    let command = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "kind": "site", "key": "s1", "name": "HQ", "area": 1200 })),
    );
    apply(handle.raw(), &command, None).await.expect("apply valid");

    assert!(read_record(handle.raw(), &target).await.expect("read").is_some());
}

#[tokio::test]
async fn invalid_content_is_refused_before_any_write() {
    let handle = open_gate_store("validate_invalid").await;
    register_site_collection(&handle).await;
    let actor = operator("alice");
    grant(&handle, &actor).await;

    let target = Id::from_raw("site-bad");
    // Missing required `name`, and `area` is the wrong type.
    let command = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "kind": "site", "key": "s1", "area": "huge" })),
    );
    let err = apply(handle.raw(), &command, None).await.expect_err("must reject");
    assert!(matches!(err, GateError::Validation(_)), "got {err:?}");

    // Fail-closed: no record written for a rejected command.
    assert!(read_record(handle.raw(), &target).await.expect("read").is_none());
}

#[tokio::test]
async fn an_unknown_kind_is_admitted_when_fail_open() {
    let handle = open_gate_store("validate_failopen").await;
    let actor = operator("alice");
    grant(&handle, &actor).await;

    let target = Id::from_raw("misc-1");
    let command = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "kind": "gadget", "whatever": true })),
    );
    apply(handle.raw(), &command, None).await.expect("fail-open admits");
    assert!(read_record(handle.raw(), &target).await.expect("read").is_some());
}

#[tokio::test]
async fn an_unknown_kind_is_rejected_under_strict_mode() {
    let handle = open_gate_store("validate_strict").await;
    // Flip the namespace to strict.
    let settings = Record::new(
        NS,
        serde_json::json!({ "kind": NAMESPACE_SETTINGS_KIND, "strict": true }),
    );
    create_record(handle.raw(), &settings).await.expect("strict on");

    let actor = operator("alice");
    grant(&handle, &actor).await;

    let target = Id::from_raw("misc-2");
    let command = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "kind": "gadget", "whatever": true })),
    );
    let err = apply(handle.raw(), &command, None).await.expect_err("strict rejects");
    assert!(matches!(err, GateError::Validation(_)), "got {err:?}");
    assert!(read_record(handle.raw(), &target).await.expect("read").is_none());
}

#[tokio::test]
async fn a_partial_update_validates_against_the_merged_record() {
    let handle = open_gate_store("validate_update_merge").await;
    register_site_collection(&handle).await;
    let actor = operator("alice");
    grant(&handle, &actor).await;

    // Seed a valid site through the gate.
    let target = Id::from_raw("site-merge");
    let create = Command::new(
        actor.clone(),
        Capability::RuleInvoke,
        target.clone(),
        Change::Create(serde_json::json!({ "kind": "site", "key": "s1", "name": "HQ" })),
    );
    apply(handle.raw(), &create, None).await.expect("create");

    // A patch that omits required fields still validates — the merged record keeps
    // `kind`, `key`, and `name`.
    let patch = Command::new(
        actor,
        Capability::RuleInvoke,
        target.clone(),
        Change::Update(serde_json::json!({ "area": 900 })),
    );
    apply(handle.raw(), &patch, None).await.expect("partial update validates");

    let content = read_record(handle.raw(), &target)
        .await
        .expect("read")
        .expect("present")
        .content;
    assert_eq!(content.get("area").and_then(|v| v.as_i64()), Some(900));
    assert_eq!(content.get("name").and_then(|v| v.as_str()), Some("HQ"));
}
