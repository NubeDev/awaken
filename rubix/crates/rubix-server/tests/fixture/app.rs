//! Boot the transport over an in-memory store with the gate schema applied and a
//! granted principal provisioned — the fixture the HTTP/WS/OpenAPI tests run on.
//!
//! The WS-16 integration tests exercise the real route table on kv-mem
//! (`rubix/docs/sessions/WS-16.md`): mutations must cross the WS-05 gate (with an
//! audit row written) and reads must run on the WS-03 scoped session. This
//! fixture opens the store, declares the gate + audit schema, provisions an
//! operator principal, grants it the record-write capability (the gate's
//! `IngestPublish`, the capability the transport routes a record mutation
//! through), and returns the assembled `Router` plus the principal's credential
//! headers.

use axum::Router;
use rubix_core::{Id, Principal, PrincipalKind, Role, RuntimeConfig};
use rubix_gate::{Capability, PrincipalToken, authenticate, create_grant, provision_principal};
use rubix_server::{AppState, router};
use rubix_store::StoreHandle;

/// Namespace every server test runs against.
pub const NS: &str = "rubix";
/// The provisioned principal's subject.
pub const SUBJECT: &str = "operator";
/// The provisioned principal's secret.
pub const SECRET: &str = "pw";

/// A booted transport plus the handle and credential headers a test drives it
/// with.
pub struct TestApp {
    /// The assembled transport router.
    pub app: Router,
    /// The store owner handle, for asserting on persisted rows (e.g. audit).
    ///
    /// Not every test binary reads the handle (the fixture is shared via
    /// `#[path]`), so the unused-field lint is allowed here rather than per test.
    #[allow(dead_code)]
    pub store: StoreHandle,
}

/// Boot the transport with the gate schema applied and a granted principal.
///
/// `capabilities` are granted to the principal by an admin in `NS` (the gate's
/// no-escalation rule), so a test can choose which actions the principal may
/// perform. `database` keeps each test's kv-mem instance isolated.
pub async fn boot(database: &str, capabilities: &[Capability]) -> TestApp {
    let cfg = RuntimeConfig::in_memory(NS, database);
    let store = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_gate::define_gate_schema(store.raw())
        .await
        .expect("define gate schema");
    rubix_gate::define_audit_schema(store.raw())
        .await
        .expect("define audit schema");

    let principal = Principal::new(Id::from_raw(SUBJECT), NS, PrincipalKind::User, Role::Operator);
    provision_principal(store.raw(), &principal, SECRET)
        .await
        .expect("provision principal");

    let admin = Principal::new(Id::from_raw("admin"), NS, PrincipalKind::User, Role::Admin);
    for capability in capabilities {
        create_grant(store.raw(), &admin, &principal, *capability)
            .await
            .expect("grant capability");
    }

    // Confirm the credential resolves before any test relies on it.
    authenticate(store.raw(), &PrincipalToken::new(SUBJECT, SECRET))
        .await
        .expect("authenticate provisioned principal");

    let app = router(AppState::new(store.clone(), NS, database));
    TestApp { app, store }
}
