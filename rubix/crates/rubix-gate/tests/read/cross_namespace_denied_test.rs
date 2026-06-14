//! Integration: a principal in namespace A cannot read a record in namespace B.
//!
//! The denial is SurrealDB-native: a direct by-id select of a foreign record on
//! the scoped session resolves to `None` because the engine's row-level
//! permission excludes it — not because the gate filtered it (contract #2,
//! `rubix/STACK-DEISGN.md`). The same record is fully readable on the root
//! handle, isolating the difference to the session scope.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Record, Role, create_record, read_record};
use rubix_gate::{
    PrincipalToken, authenticate, issue_scoped_session, provision_principal,
    read_record_on_session, read_records_on_session,
};

use gate::open::open_gate_store;

#[tokio::test]
async fn principal_cannot_read_a_foreign_namespace_record() {
    let database = "cross_ns";
    let handle = open_gate_store(database).await;

    let foreign = Record::new("tenant-b", serde_json::json!({ "secret": true }));
    create_record(handle.raw(), &foreign)
        .await
        .expect("seed foreign record");

    // The root handle can see the record — confirms it exists and is well-formed.
    let on_root = read_record(handle.raw(), &foreign.id)
        .await
        .expect("root read");
    assert!(on_root.is_some(), "record exists on the root session");

    // A principal scoped to tenant-a signs in.
    let principal = Principal::new(
        Id::from_raw("alice"),
        "tenant-a",
        PrincipalKind::User,
        Role::Viewer,
    );
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision");
    let token = PrincipalToken::new("alice", "pw");
    let resolved = authenticate(handle.raw(), &token).await.expect("authenticate");
    let session = issue_scoped_session(handle.raw(), "rubix", database, resolved, &token)
        .await
        .expect("issue scoped session");

    // By-id read of the foreign record is denied (resolves to None) by SurrealDB.
    let denied = read_record_on_session(&session, &foreign.id)
        .await
        .expect("scoped by-id read");
    assert!(
        denied.is_none(),
        "foreign-namespace record must be invisible to the scoped session",
    );

    // The list read returns nothing — tenant-a has no records of its own.
    let visible = read_records_on_session(&session)
        .await
        .expect("scoped list read");
    assert!(
        visible.is_empty(),
        "scoped session sees no foreign records via the list path either",
    );
}
