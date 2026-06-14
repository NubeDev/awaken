//! Shared test fixture: an in-memory store with the gate schema, plus a
//! scoped-session issuer.
//!
//! The data-change plane runs on a gate-issued scoped session, so the bus tests
//! need the real gate auth + row-level permissions over a live engine
//! (`rubix/STACK-DEISGN.md`, "Key decisions": kv-mem for tests). This fixture
//! opens the durable handle, applies the gate + audit schema, and offers a
//! helper that provisions a principal and signs a scoped session in.

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    PrincipalToken, ScopedSession, authenticate, issue_scoped_session, provision_principal,
};
use rubix_store::StoreHandle;

/// Namespace the SurrealDB datastore is bootstrapped under. Records carry their
/// own tenant namespace field, which the row-level read permission scopes on.
pub const NS: &str = "rubix";

/// Open an in-memory store handle with the gate and audit schema applied.
pub async fn open_bus_store(database: &str) -> StoreHandle {
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

/// Provision a viewer principal in `tenant` and issue its scoped session.
pub async fn scoped_session_for(
    handle: &StoreHandle,
    database: &str,
    subject: &str,
    tenant: &str,
) -> ScopedSession {
    let principal = Principal::new(
        Id::from_raw(subject),
        tenant,
        PrincipalKind::User,
        Role::Viewer,
    );
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision principal");
    let token = PrincipalToken::new(subject, "pw");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate");
    issue_scoped_session(handle.raw(), NS, database, resolved, &token)
        .await
        .expect("issue scoped session")
}
