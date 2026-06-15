//! Shared test fixture: an in-memory store with the gate + audit schema defined,
//! plus a namespace admin to confer extension grants.
//!
//! Extension integration tests exercise real SurrealDB auth, row-level
//! permissions, and the WS-05 gate, so they run against a live kv-mem engine
//! (`rubix/STACK-DEISGN.md`, "Key decisions"). The fixture opens the durable
//! handle (which runs `init_schema`) and applies the gate and audit schema on
//! top, exactly as the gate's own tests do.

use rubix_core::{Id, Principal, PrincipalKind, Role, RuntimeConfig};
use rubix_store::StoreHandle;

/// Namespace every extension test runs against. Each test passes its own
/// database name to keep the in-memory datastores isolated.
pub const NS: &str = "rubix";

/// Open an in-memory store handle with the gate and audit schema applied.
pub async fn open_ext_store(database: &str) -> StoreHandle {
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

/// The namespace admin that confers extension grants (a human admin, not an
/// extension — grant administration is never an extension action).
///
/// Each test binary compiles this fixture independently, so a test that does not
/// confer grants leaves this unused; the attribute keeps that binary warning-free.
#[must_use]
#[allow(dead_code)]
pub fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}
