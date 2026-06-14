//! An in-memory store + a scoped session, the fixture the query tests read on.
//!
//! The query surface reads through a gate-issued scoped session, so the tests
//! need real SurrealDB auth and row-level permissions on kv-mem
//! (`rubix/STACK-DEISGN.md`, "Key decisions": kv-mem for tests). This fixture
//! opens the durable handle (running `init_schema`), applies the gate + audit
//! schema, provisions a principal, and issues its scoped session.

use rubix_core::{Id, Principal, PrincipalKind, RuntimeConfig, Role};
use rubix_gate::{
    PrincipalToken, ScopedSession, authenticate, issue_scoped_session, provision_principal,
};
use rubix_store::StoreHandle;

/// Namespace every query test runs against.
pub const NS: &str = "rubix";

/// Open an in-memory store with the gate + audit schema applied.
pub async fn open_query_store(database: &str) -> StoreHandle {
    let cfg = RuntimeConfig::in_memory(NS, database);
    let handle = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_gate::define_gate_schema(handle.raw())
        .await
        .expect("define gate schema");
    rubix_gate::define_audit_schema(handle.raw())
        .await
        .expect("define audit schema");
    handle
}

/// Provision `subject` in `namespace` with `role` and issue its scoped session.
pub async fn scoped_session_for(
    handle: &StoreHandle,
    database: &str,
    subject: &str,
    namespace: &str,
    role: Role,
) -> (Principal, ScopedSession) {
    let principal = Principal::new(Id::from_raw(subject), namespace, PrincipalKind::User, role);
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
