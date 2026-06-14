//! Open a store, issue a scoped session, and administer grants for the tests.

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Capability, PrincipalToken, ScopedSession, authenticate, create_grant, issue_scoped_session,
    provision_principal,
};
use rubix_store::StoreHandle;

/// Namespace every datasource test runs against.
pub const NS: &str = "rubix";

/// Open an in-memory store with the gate + audit schema applied.
pub async fn open_datasource_store(database: &str) -> StoreHandle {
    let cfg = rubix_core::RuntimeConfig::in_memory(NS, database);
    let handle = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_gate::define_gate_schema(handle.raw())
        .await
        .expect("define gate schema");
    rubix_gate::define_audit_schema(handle.raw())
        .await
        .expect("define audit schema");
    handle
}

/// An admin in `NS` that may administer grants for same-namespace principals.
pub fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

/// Provision `subject` in `NS` with `role` and issue its scoped session.
pub async fn scoped_session_for(
    handle: &StoreHandle,
    database: &str,
    subject: &str,
    role: Role,
) -> (Principal, ScopedSession) {
    let principal = Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, role);
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision principal");
    let token = PrincipalToken::new(subject, "pw");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate");
    let session = issue_scoped_session(handle.raw(), NS, database, resolved.clone(), &token)
        .await
        .expect("issue scoped session");
    (resolved, session)
}

/// Grant `capability` to `principal` through the admin.
pub async fn grant(handle: &StoreHandle, principal: &Principal, capability: Capability) {
    create_grant(handle.raw(), &admin(), principal, capability)
        .await
        .expect("create grant");
}
