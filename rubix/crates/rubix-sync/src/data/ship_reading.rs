//! Publish edge-partition readings over Zenoh and land them at the receiver.
//!
//! This is the sibling of [`ship`](super::ship) for the data plane's
//! high-volume primitive: the time-series [`Reading`]. Where a record is the
//! low-volume audited config/document primitive, a reading is a numeric `value` a
//! `series` produced at an instant — millions of them, never edited, never undone
//! (`rubix/docs/design/READINGS-TIMESERIES.md`, "Retention & edge↔cloud sync").
//! The shipping shape is identical to the record path because the conflict model
//! is identical: append-only, partitioned by edge identity (`rubix/docs/SCOPE.md`,
//! "Sync and conflict model"), so reconciliation at the cloud is **ordering +
//! dedup by id, not merge** — there is no multi-master conflict by construction.
//!
//! The reading id is **deterministic** from `(series, at)`
//! ([`reading_id`](rubix_core::reading_id)), so dedup-by-id makes a replayed
//! reading a no-op for free: a re-send lands the same id and is dropped, exactly
//! as a record re-send is. The wire and the receiver apply live together in this
//! file because they are two ends of one move: the [codec](encode_reading) the
//! publisher writes is the codec the receiver reads, kept together so they cannot
//! drift.
//!
//! A reading ships under a **distinct** key-space root ([`SYNC_READING_ROOT`])
//! from the record stream ([`SYNC_DATA_ROOT`](super::ship::SYNC_DATA_ROOT)) so the
//! two streams are table-discriminated on the wire — a subscriber can scope to the
//! reading stream alone, and a reading is never mistaken for a record.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use zenoh::Session;

use rubix_core::{Id, Reading, append_readings, read_reading};

use crate::error::{Result, SyncError};

use super::dedup::SeenSet;

/// The Zenoh key-space root the reading shipper publishes under.
///
/// A reading ships to `rubix/sync/data/reading/<edge-namespace>/<id>` — a
/// table-discriminated key-space (the `/reading` segment) so the reading stream is
/// distinguishable from the record stream (which uses
/// [`SYNC_DATA_ROOT`](super::ship::SYNC_DATA_ROOT), `rubix/sync/data`). Within the
/// reading root the stream is partitioned by the edge identity (the reading's
/// namespace), mirroring the ingest partitioning (contract #5).
pub const SYNC_READING_ROOT: &str = "rubix/sync/data/reading";

/// The Zenoh key a reading ships to: the reading root, the edge partition, the id.
#[must_use]
pub fn reading_ship_key(reading: &Reading) -> String {
    format!("{SYNC_READING_ROOT}/{}/{}", reading.namespace, reading.id)
}

/// Encode a reading for the wire as canonical JSON bytes.
///
/// The wire shape carries every persisted field (id + series + measurement and
/// receive timestamps + value + namespace + content), so the receiver
/// reconstructs the exact same reading — id-addressed and timestamp-preserving —
/// with no field invented or dropped.
///
/// # Errors
/// Returns [`SyncError::Codec`] if the reading cannot be serialized.
pub fn encode_reading(reading: &Reading) -> Result<Vec<u8>> {
    serde_json::to_vec(&WireReading::from(reading)).map_err(|e| SyncError::Codec(e.to_string()))
}

/// Decode a reading shipped over the wire.
///
/// # Errors
/// Returns [`SyncError::Codec`] if the bytes are not a valid wire reading.
pub fn decode_reading(bytes: &[u8]) -> Result<Reading> {
    let wire: WireReading =
        serde_json::from_slice(bytes).map_err(|e| SyncError::Codec(e.to_string()))?;
    Ok(wire.into_reading())
}

/// Publish one reading onto its sync key-space over an open Zenoh session.
///
/// The edge calls this for each reading to ship and for each unacked reading on
/// reconnect; the publish is at-least-once and the receiver's dedup makes the
/// *effect* exactly-once — and because the reading id is deterministic from
/// `(series, at)`, even a fresh receiver dedups a replay for free.
///
/// # Errors
/// Returns [`SyncError::Publish`] if the Zenoh put fails.
pub async fn publish_reading(session: &Session, reading: &Reading) -> Result<()> {
    let payload = encode_reading(reading)?;
    session
        .put(reading_ship_key(reading), payload)
        .await
        .map_err(|e| SyncError::Publish(e.to_string()))
}

/// Apply a shipped reading at the receiver, idempotently.
///
/// Marks the id in `seen` and lands the reading into the receiver store under its
/// own id (id-addressed, so the cloud row mirrors the edge row). A reading whose
/// id is already in `seen`, or already present in the store, is a no-op — the
/// idempotent-replay guarantee (`rubix/docs/SCOPE.md`). Because the id is derived
/// from `(series, at)`, two ships of the same sample carry the same id, so the
/// dedup is automatic. Returns `true` when the reading was genuinely applied,
/// `false` when it was deduped.
///
/// # Errors
/// Returns [`SyncError::Apply`] if the existence check or the write fails.
pub async fn apply_reading(db: &Surreal<Db>, seen: &mut SeenSet, reading: &Reading) -> Result<bool> {
    if !seen.is_new(&reading.id) {
        return Ok(false);
    }
    // Belt-and-suspenders: a fresh receiver (empty `seen`) that restarts must not
    // re-insert a reading that already landed in a prior run. The store is the
    // durable dedup of readings; `seen` is the in-memory fast path.
    if read_reading(db, &reading.id)
        .await
        .map_err(|e| SyncError::Apply(e.to_string()))?
        .is_some()
    {
        seen.mark(reading.id.clone());
        return Ok(false);
    }
    append_readings(db, std::slice::from_ref(reading))
        .await
        .map_err(|e| SyncError::Apply(e.to_string()))?;
    seen.mark(reading.id.clone());
    Ok(true)
}

/// Apply a batch of shipped readings in deterministic order, deduping each.
///
/// Orders the batch then applies each through [`apply_reading`], so an
/// out-of-order or replayed batch lands the same set in the same order. Returns
/// the number of readings genuinely applied (duplicates do not count).
///
/// Ordering is inline here rather than through
/// [`in_apply_order`](super::order::in_apply_order): that helper is `Record`-typed
/// (it sorts by a record's `created`/id), and a `Reading` is a deliberately
/// distinct domain type, not a `Record`. The order is the same shape — primary key
/// is the receive instant (`created`), with the globally-unique, deterministic id
/// as a stable tiebreak — so two receivers that saw the same set converge to the
/// same applied sequence, independent of arrival order.
///
/// # Errors
/// Returns [`SyncError::Apply`] on the first reading that fails to land.
pub async fn apply_reading_batch(
    db: &Surreal<Db>,
    seen: &mut SeenSet,
    mut batch: Vec<Reading>,
) -> Result<usize> {
    batch.sort_by(|a, b| {
        a.created
            .cmp(&b.created)
            .then_with(|| a.id.as_str().cmp(b.id.as_str()))
    });
    let mut applied = 0;
    for reading in batch {
        if apply_reading(db, seen, &reading).await? {
            applied += 1;
        }
    }
    Ok(applied)
}

/// The wire projection of a reading: the reading fields, serde-encodable.
///
/// The store-facing row keys on a SurrealDB `RecordId` and links `series` as a
/// `record`; the wire carries plain string ids so the shape is engine-neutral and
/// reconstructs the exact same [`Reading`] — every field the edge persisted, none
/// invented or dropped.
#[derive(serde::Serialize, serde::Deserialize)]
struct WireReading {
    id: String,
    series: String,
    at: surrealdb::types::Datetime,
    value: f64,
    namespace: String,
    created: surrealdb::types::Datetime,
    content: serde_json::Value,
}

impl From<&Reading> for WireReading {
    fn from(reading: &Reading) -> Self {
        Self {
            id: reading.id.to_string(),
            series: reading.series.clone(),
            at: reading.at,
            value: reading.value,
            namespace: reading.namespace.clone(),
            created: reading.created,
            content: reading.content.clone(),
        }
    }
}

impl WireReading {
    fn into_reading(self) -> Reading {
        // Reconstruct the same `Reading` the store round-trips, so a shipped
        // reading is byte-for-byte the reading the edge persisted.
        Reading {
            id: Id::from_raw(self.id),
            series: self.series,
            at: self.at,
            value: self.value,
            namespace: self.namespace,
            created: self.created,
            content: self.content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SYNC_READING_ROOT, apply_reading, decode_reading, encode_reading, reading_ship_key,
    };
    use crate::data::dedup::SeenSet;
    use rubix_core::{Reading, RuntimeConfig};
    use rubix_store::StoreHandle;
    use surrealdb::types::Datetime;

    fn reading(ns: &str, series: &str, secs: i64, value: f64) -> Reading {
        let at = Datetime::from_timestamp(secs, 0).expect("valid instant");
        Reading::new(ns, series, at, value, serde_json::json!({ "q": "good" }))
    }

    #[test]
    fn a_reading_round_trips_through_the_wire_codec() {
        let original = reading("edge-7", "reg-1", 1_000, 21.5);
        let bytes = encode_reading(&original).expect("encode");
        let decoded = decode_reading(&bytes).expect("decode");
        assert_eq!(decoded, original, "wire codec preserves the reading exactly");
    }

    #[test]
    fn reading_ship_key_partitions_by_edge_namespace_and_id() {
        let r = reading("edge-7", "reg-1", 1_000, 21.5);
        let key = reading_ship_key(&r);
        assert_eq!(key, format!("{SYNC_READING_ROOT}/edge-7/{}", r.id));
        assert!(key.starts_with("rubix/sync/data/reading/edge-7/"));
    }

    #[tokio::test]
    async fn a_replayed_reading_lands_exactly_once() {
        // Apply a reading twice through the same receiver state: the first apply
        // genuinely lands it, the second is deduped — no duplicate row. This is the
        // idempotent-replay guarantee the deterministic `(series, at)` id buys.
        let cfg = RuntimeConfig::in_memory("rubix", "ship_reading_dedup");
        let store = StoreHandle::open(&cfg).await.expect("open in-memory store");

        let r = reading("edge-7", "reg-1", 1_000, 21.5);
        let mut seen = SeenSet::new();

        let first = apply_reading(store.raw(), &mut seen, &r)
            .await
            .expect("first apply");
        assert!(first, "first apply genuinely lands the reading");
        assert_eq!(seen.len(), 1);

        let second = apply_reading(store.raw(), &mut seen, &r)
            .await
            .expect("replay apply");
        assert!(!second, "a replayed reading applies nothing new");
        assert_eq!(seen.len(), 1, "dedup keeps exactly one entry per id");
    }
}
