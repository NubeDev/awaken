//! Shared test fixture: an in-memory store with the gate schema defined.
//!
//! Integration tests exercise real SurrealDB auth and row-level permissions, so
//! they need a live engine (`rubix/STACK-DEISGN.md`, "Key decisions": kv-mem for
//! tests). The fixture opens the durable handle (which runs `init_schema`), then
//! applies the gate's access method and record permissions on top.

use rubix_core::RuntimeConfig;
use rubix_store::StoreHandle;

/// Namespace every gate test runs against. Each test passes its own database
/// name to keep the in-memory datastores isolated.
pub const NS: &str = "rubix";

/// Open an in-memory store handle with the gate and audit schema applied.
pub async fn open_gate_store(database: &str) -> StoreHandle {
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
