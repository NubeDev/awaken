//! Integration: a principal cannot read another namespace's readings.
//!
//! The `reading` table is the data plane, but its read scope is enforced exactly
//! like `record`'s: the gate overwrites `FOR select WHERE namespace =
//! $auth.namespace` onto it (`rubix/docs/design/READINGS-TIMESERIES.md`,
//! "Scoped permissions are not optional"). A reading appended into tenant-b is
//! fully visible on the root handle but invisible to a tenant-a scoped session —
//! the denial is SurrealDB-native, never an app filter (contract #2). Without the
//! overwrite a new table is invisible-or-leaky, so this guards the correctness
//! that the perms, not the table's absence, are doing the scoping.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Reading, Role, append_readings, read_readings_window};
use rubix_gate::{
    PrincipalToken, authenticate, issue_scoped_session, provision_principal,
    read_readings_on_session,
};
use surrealdb::types::Datetime;

use gate::open::open_gate_store;

fn at(secs: i64) -> Datetime {
    Datetime::from_timestamp(secs, 0).expect("valid instant")
}

#[tokio::test]
async fn principal_cannot_read_a_foreign_namespace_reading() {
    let database = "reading_scope";
    let handle = open_gate_store(database).await;

    // A reading owned by tenant-b, appended on the owner handle (the data-plane
    // write never crosses a scoped session).
    let foreign = Reading::new("tenant-b", "reg-b", at(1_000), 42.0, serde_json::json!({}));
    append_readings(handle.raw(), std::slice::from_ref(&foreign))
        .await
        .expect("append foreign reading");

    // The root handle sees it — confirms it exists and is well-formed.
    let on_root = read_readings_window(handle.raw(), "reg-b", &at(0), &at(10_000))
        .await
        .expect("root windowed read");
    assert_eq!(on_root.len(), 1, "reading exists on the root session");

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

    // The same windowed read on tenant-a's session returns nothing: SurrealDB's
    // row-level permission excludes the foreign-namespace reading.
    let denied = read_readings_on_session(&session, "reg-b", &at(0), &at(10_000))
        .await
        .expect("scoped windowed read");
    assert!(
        denied.is_empty(),
        "foreign-namespace reading must be invisible to the scoped session",
    );
}
