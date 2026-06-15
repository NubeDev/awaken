//! Shared test fixture: an in-memory store with the gate + audit schema defined,
//! plus a namespace admin to confer agent grants.
//!
//! Agent integration tests exercise real SurrealDB auth, row-level permissions,
//! and the WS-05 gate, so they run against a live kv-mem engine
//! (`rubix/STACK-DEISGN.md`, "Key decisions"). The fixture opens the durable
//! handle (which runs `init_schema`) and applies the gate and audit schema on
//! top, exactly as the gate's and extensions' own tests do.

use rubix_core::{Id, Principal, PrincipalKind, Role, RuntimeConfig};
use rubix_store::StoreHandle;

/// Namespace every agent test runs against. Each test passes its own database
/// name to keep the in-memory datastores isolated.
pub const NS: &str = "rubix";

/// Open an in-memory store handle with the gate and audit schema applied.
pub async fn open_agent_store(database: &str) -> StoreHandle {
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

/// The namespace admin that confers agent grants (a human admin, not the agent —
/// grant administration is never an agent action, so the agent cannot escalate
/// its own authority).
///
/// Each test binary compiles this fixture independently, so a binary that does
/// not use it stays warning-free under the attribute.
#[must_use]
#[allow(dead_code)]
pub fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}
