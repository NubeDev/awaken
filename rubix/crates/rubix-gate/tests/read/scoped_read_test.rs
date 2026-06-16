//! Integration: a scoped session reads only its own namespace's records.
//!
//! Proves contract #1/#2 — the SELECT runs on the gate-issued session and
//! SurrealDB row-level permissions filter the rows; the gate adds no app filter.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Record, Role, create_record};
use rubix_gate::{
    PrincipalToken, authenticate, issue_scoped_session, provision_principal,
    read_records_on_session,
};

use gate::open::open_gate_store;

#[tokio::test]
async fn scoped_session_reads_only_its_namespace_records() {
    let database = "scoped_read";
    let handle = open_gate_store(database).await;

    // Two tenants' data live in the same database; the principal is scoped to A.
    let in_a = Record::new("tenant-a", serde_json::json!({ "temp": 21 }));
    let also_a = Record::new("tenant-a", serde_json::json!({ "temp": 22 }));
    let in_b = Record::new("tenant-b", serde_json::json!({ "temp": 99 }));
    for record in [&in_a, &also_a, &in_b] {
        create_record(handle.raw(), record)
            .await
            .expect("seed record");
    }

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
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate");
    let session = issue_scoped_session(handle.raw(), "rubix", database, resolved, &token)
        .await
        .expect("issue scoped session");

    let visible = read_records_on_session(&session)
        .await
        .expect("scoped read");

    assert_eq!(
        visible.len(),
        2,
        "scoped session sees only tenant-a records"
    );
    assert!(visible.iter().all(|r| r.namespace == "tenant-a"));
    let ids: Vec<_> = visible.iter().map(|r| r.id.clone()).collect();
    assert!(ids.contains(&in_a.id));
    assert!(ids.contains(&also_a.id));
    assert!(!ids.contains(&in_b.id));
}
