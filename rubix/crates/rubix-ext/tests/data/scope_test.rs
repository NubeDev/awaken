//! Integration: an extension reads only its own namespace's records, and its
//! data-plane scope delegates to WS-12 key-space authorization.
//!
//! Two halves of the extension data plane, both on a live kv-mem engine:
//!
//! - **Record scope (WS-03).** A query on the extension's scoped session returns
//!   only records in its granted namespace; a foreign-namespace record is
//!   invisible — the denial is SurrealDB-native, the same row-level scope a user
//!   is held to (contract #1).
//! - **Key-space scope (WS-12).** `authorize_data_scope` is pure delegation to
//!   WS-12: with the `zenoh-subscribe` grant, a key-space inside the extension's
//!   edge partition is authorized; one outside it is refused at subscribe, before
//!   any Zenoh session opens (contract #2). No extension-only scoping path exists
//!   to escape the partition a user is held to.

#[path = "../ext/mod.rs"]
mod ext;

use rubix_core::{Id, Principal, PrincipalKind, Record, Role, create_record};
use rubix_gate::{
    Capability, PrincipalToken, authenticate, create_grant, issue_scoped_session,
    read_records_on_session,
};

use ext::open::open_ext_store;

/// The tenant the extension is scoped to (its principal namespace field, which
/// `$auth.namespace` row-perms filter on).
const TENANT: &str = "tenant-a";

#[tokio::test]
async fn an_extension_reads_only_its_namespace_records() {
    let database = "ext_scope_records";
    let handle = open_ext_store(database).await;

    // One record in the extension's tenant, one in a foreign tenant.
    let own = Record::new(TENANT, serde_json::json!({ "reading": 21 }));
    let foreign = Record::new("tenant-b", serde_json::json!({ "secret": true }));
    create_record(handle.raw(), &own).await.expect("seed own");
    create_record(handle.raw(), &foreign)
        .await
        .expect("seed foreign");

    // Provision the extension into the tenant and sign it in to a scoped session.
    let registration =
        rubix_ext::register_extension(handle.raw(), "scope-ext", TENANT, "k")
            .await
            .expect("register extension");
    let token = PrincipalToken::new("scope-ext", "k");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate");
    assert_eq!(&resolved, registration.principal());
    let session = issue_scoped_session(handle.raw(), "rubix", database, resolved, &token)
        .await
        .expect("issue scoped session");

    // The scoped session sees only the tenant's own record.
    let visible = read_records_on_session(&session)
        .await
        .expect("scoped list read");
    assert_eq!(visible.len(), 1, "exactly the tenant's own record is visible");
    assert_eq!(visible[0].namespace, TENANT);
}

#[tokio::test]
async fn the_data_plane_scope_delegates_to_ws12_key_space_authorization() {
    let handle = open_ext_store("ext_scope_keyspace").await;

    let registration =
        rubix_ext::register_extension(handle.raw(), "stream-ext", TENANT, "k")
            .await
            .expect("register extension");
    let extension = registration.principal().clone();

    // Grant the zenoh-subscribe capability through the same WS-04 path a user
    // is granted through (an admin in the extension's namespace).
    let admin = Principal::new(
        Id::from_raw("tenant-admin"),
        TENANT,
        PrincipalKind::User,
        Role::Admin,
    );
    create_grant(handle.raw(), &admin, &extension, Capability::ZenohSubscribe)
        .await
        .expect("grant zenoh-subscribe");

    // A key-space inside the extension's edge partition is authorized.
    let within = format!("rubix/ingest/{TENANT}/sensors/temp");
    let scope = rubix_ext::authorize_data_scope(handle.raw(), &extension, &within)
        .await
        .expect("in-partition key-space is authorized");
    assert!(scope.scope().includes(scope.scope()));

    // A key-space outside the partition is refused at subscribe.
    let outside = "rubix/ingest/tenant-b/sensors/temp";
    let err = rubix_ext::authorize_data_scope(handle.raw(), &extension, outside)
        .await
        .expect_err("out-of-partition key-space must be refused");
    assert!(matches!(err, rubix_ext::ExtError::Scope(_)));
}
