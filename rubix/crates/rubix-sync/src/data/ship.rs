//! Publish edge-partition records over Zenoh and land them at the receiver.
//!
//! The data plane ships append-only records edge→cloud (`rubix/docs/SCOPE.md`,
//! "Sync and conflict model"). SurrealDB has no multi-master replication, so this
//! is an application-level shipper: the edge encodes each record and publishes it
//! to a sync key-space keyed by its edge identity; the cloud receives, applies in
//! [order](super::order), and dedups by id ([`SeenSet`](super::dedup::SeenSet)) so
//! a replayed record is a no-op. Because the data plane is partitioned by edge
//! identity (contract #5), two edges never write the same record — reconciliation
//! is ordering + dedup, never a merge, so there is no multi-master conflict here.
//!
//! The Zenoh wire and the receiver apply are both in this file because they are
//! two ends of one move: the [codec](encode_record) the publisher writes is the
//! codec the receiver reads, kept together so they cannot drift.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use zenoh::Session;

use rubix_core::{Id, Record, create_record, read_record};

use crate::error::{Result, SyncError};

use super::dedup::SeenSet;
use super::order::in_apply_order;

/// The Zenoh key-space root the data-plane shipper publishes under.
///
/// A record ships to `rubix/sync/data/<edge-namespace>/<id>` — partitioned by the
/// edge identity (the record's namespace) so a subscriber can scope to one edge's
/// stream, mirroring the ingest partitioning (contract #5).
pub const SYNC_DATA_ROOT: &str = "rubix/sync/data";

/// The Zenoh key a record ships to: the data root, the edge partition, the id.
#[must_use]
pub fn ship_key(record: &Record) -> String {
    format!("{SYNC_DATA_ROOT}/{}/{}", record.namespace, record.id)
}

/// Encode a record for the wire as canonical JSON bytes.
///
/// The wire shape is the persisted [`RecordRow`] form (id + namespace + content +
/// timestamps), so the receiver reconstructs the exact same record — id-addressed
/// and timestamp-preserving — with no field invented or dropped.
///
/// # Errors
/// Returns [`SyncError::Codec`] if the record cannot be serialized.
pub fn encode_record(record: &Record) -> Result<Vec<u8>> {
    serde_json::to_vec(&WireRecord::from(record)).map_err(|e| SyncError::Codec(e.to_string()))
}

/// Decode a record shipped over the wire.
///
/// # Errors
/// Returns [`SyncError::Codec`] if the bytes are not a valid wire record.
pub fn decode_record(bytes: &[u8]) -> Result<Record> {
    let wire: WireRecord =
        serde_json::from_slice(bytes).map_err(|e| SyncError::Codec(e.to_string()))?;
    Ok(wire.into_record())
}

/// Publish one record onto its sync key-space over an open Zenoh session.
///
/// The edge calls this for each record to ship and for each unacked record on
/// reconnect ([`replay`](super::replay)); the publish is at-least-once and the
/// receiver's dedup makes the *effect* exactly-once.
///
/// # Errors
/// Returns [`SyncError::Publish`] if the Zenoh put fails.
pub async fn publish_record(session: &Session, record: &Record) -> Result<()> {
    let payload = encode_record(record)?;
    session
        .put(ship_key(record), payload)
        .await
        .map_err(|e| SyncError::Publish(e.to_string()))
}

/// Apply a shipped record at the receiver, idempotently.
///
/// Marks the id in `seen` and lands the record into the receiver store under its
/// own id (id-addressed, so the cloud row mirrors the edge row). A record whose id
/// is already in `seen`, or already present in the store, is a no-op — the
/// idempotent-replay guarantee (`rubix/docs/SCOPE.md`). Returns `true` when the
/// record was genuinely applied, `false` when it was deduped.
///
/// # Errors
/// Returns [`SyncError::Apply`] if the existence check or the write fails.
pub async fn apply_record(db: &Surreal<Db>, seen: &mut SeenSet, record: &Record) -> Result<bool> {
    if !seen.is_new(&record.id) {
        return Ok(false);
    }
    // Belt-and-suspenders: a fresh receiver (empty `seen`) that restarts must not
    // re-insert a record that already landed in a prior run. The store is the
    // durable dedup of record; `seen` is the in-memory fast path.
    if read_record(db, &record.id)
        .await
        .map_err(|e| SyncError::Apply(e.to_string()))?
        .is_some()
    {
        seen.mark(record.id.clone());
        return Ok(false);
    }
    create_record(db, record)
        .await
        .map_err(|e| SyncError::Apply(e.to_string()))?;
    seen.mark(record.id.clone());
    Ok(true)
}

/// Apply a batch of shipped records in deterministic order, deduping each.
///
/// Orders the batch ([`in_apply_order`]) then applies each through
/// [`apply_record`], so an out-of-order or replayed batch lands the same set in
/// the same order. Returns the number of records genuinely applied (duplicates do
/// not count).
///
/// # Errors
/// Returns [`SyncError::Apply`] on the first record that fails to land.
pub async fn apply_batch(db: &Surreal<Db>, seen: &mut SeenSet, batch: Vec<Record>) -> Result<usize> {
    let mut applied = 0;
    for record in in_apply_order(batch) {
        if apply_record(db, seen, &record).await? {
            applied += 1;
        }
    }
    Ok(applied)
}

/// The wire projection of a record: the record fields, serde-encodable.
///
/// The store-facing row keys on a SurrealDB `RecordId`; the wire carries a plain
/// string id so the shape is engine-neutral and reconstructs the exact same
/// [`Record`] — every field the edge persisted, none invented or dropped.
#[derive(serde::Serialize, serde::Deserialize)]
struct WireRecord {
    id: String,
    namespace: String,
    content: serde_json::Value,
    created: surrealdb::types::Datetime,
    updated: surrealdb::types::Datetime,
}

impl From<&Record> for WireRecord {
    fn from(record: &Record) -> Self {
        Self {
            id: record.id.to_string(),
            namespace: record.namespace.clone(),
            content: record.content.clone(),
            created: record.created,
            updated: record.updated,
        }
    }
}

impl WireRecord {
    fn into_record(self) -> Record {
        // Reconstruct the same `Record` the store round-trips, so a shipped record
        // is byte-for-byte the record the edge persisted.
        Record {
            id: Id::from_raw(self.id),
            namespace: self.namespace,
            content: self.content,
            created: self.created,
            updated: self.updated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_record, encode_record, ship_key};
    use rubix_core::{Id, Record};

    fn record(id: &str, ns: &str) -> Record {
        let mut r = Record::new(ns, serde_json::json!({ "temp": 21.5 }));
        r.id = Id::from_raw(id);
        r
    }

    #[test]
    fn a_record_round_trips_through_the_wire_codec() {
        let original = record("rec-1", "edge-7");
        let bytes = encode_record(&original).expect("encode");
        let decoded = decode_record(&bytes).expect("decode");
        assert_eq!(decoded, original, "wire codec preserves the record exactly");
    }

    #[test]
    fn ship_key_partitions_by_edge_namespace_and_id() {
        let key = ship_key(&record("rec-1", "edge-7"));
        assert_eq!(key, "rubix/sync/data/edge-7/rec-1");
    }
}
