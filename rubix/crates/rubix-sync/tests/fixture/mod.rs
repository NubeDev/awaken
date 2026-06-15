#![allow(dead_code)] // Shared fixture: each test binary uses only part of it.

//! Two in-memory stores simulating an edge node and a cloud deployment.
//!
//! The WS-15 integration tests model edge↔cloud sync on kv-mem: the data plane
//! ships append-only records from the edge store into the cloud store, deduping
//! idempotently on replay; the config plane reconciles a contested definition.
//! Each store is its own in-memory datastore (`RuntimeConfig::in_memory`), so the
//! edge and cloud partitions are genuinely separate engines, exactly as a real
//! deployment splits them — not two namespaces sharing one process datastore.

use rubix_core::{Id, Record, RuntimeConfig};
use rubix_store::StoreHandle;

/// The edge node's namespace (its edge identity, the data-plane partition key).
pub const EDGE_NS: &str = "edge-7";
/// The cloud deployment's namespace.
pub const CLOUD_NS: &str = "cloud";

/// Open a fresh in-memory store for one side, isolated by `database`.
pub async fn open_store(database: &str) -> StoreHandle {
    let cfg = RuntimeConfig::in_memory("rubix", database);
    StoreHandle::open(&cfg).await.expect("open in-memory store")
}

/// Build an append-only data record with an explicit id and edge-partition
/// namespace, carrying `content`.
#[must_use]
pub fn data_record(id: &str, content: serde_json::Value) -> Record {
    let mut record = Record::new(EDGE_NS, content);
    record.id = Id::from_raw(id);
    record
}
