//! Shared test fixture: an in-memory store with the `trace` table defined.
//!
//! The durable trace verbs (persist, retain, assemble) run on the root/owner
//! store handle, the only session past the append-only `trace` table permissions
//! (`rubix/STACK-DEISGN.md`, "Key decisions": kv-mem for tests). This fixture
//! opens the handle and applies the trace schema so the integration tests write
//! and read real spans over a live engine.

use rubix_store::StoreHandle;

/// Namespace the SurrealDB datastore is bootstrapped under. Spans carry their own
/// tenant namespace field, which the row-level read permission scopes on.
pub const NS: &str = "rubix";

/// Open an in-memory store handle with the `trace` table schema applied.
pub async fn open_trace_store(database: &str) -> StoreHandle {
    let cfg = rubix_core::RuntimeConfig::in_memory(NS, database);
    let handle = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_trace::define_trace_schema(handle.raw())
        .await
        .expect("define trace schema");
    handle
}
