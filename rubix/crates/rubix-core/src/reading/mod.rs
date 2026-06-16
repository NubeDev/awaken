//! The time-series reading — the data plane's append-only primitive.
//!
//! A reading is **not** a [`Record`](crate::Record). Where a record is the
//! low-volume, audited, undoable config/document primitive, a reading is the
//! high-volume data-plane primitive: a numeric `value` a `series` produced at an
//! instant (`at`), of which there are millions, never edited, never undone, and
//! only ever queried "this series, this window, bucketed"
//! (`rubix/docs/design/READINGS-TIMESERIES.md`). It lives in its own `reading`
//! table with `(namespace, series, at)` indexes, written append-only off the
//! command gate.
//!
//! The shape is deliberately lean — `series`, `at`, `value` plus the `namespace`
//! edge partition, a receive-time `created`, and a free-form `content` for the
//! rare quality flag. Display metadata (`unit`, `quantity`, …) lives on the
//! **series record** (the register/point the `series` id points at), not copied
//! onto every sample. This one type is shared across ingest, the bulk-append
//! endpoint, sync, migration, and the query projection so the shape cannot drift
//! across the seams that touch it (`READINGS-TIMESERIES.md`, "One shared
//! `Reading` domain type").

mod append;
mod list;
mod migrate;
mod row;
mod sweep;

pub use append::append_readings;
pub use list::{list_readings, read_reading, read_readings_window};
pub use migrate::{HistoryMigration, migrate_history_to_readings};
pub use sweep::sweep_readings_before;

pub(crate) use row::ReadingRow;

use surrealdb::types::Datetime;
use uuid::Uuid;

use crate::id::Id;

/// The SurrealDB table every reading lives in (the data plane, append-only).
pub(crate) const READING_TABLE: &str = "reading";

/// The DNS-style UUIDv5 namespace the deterministic reading id derives under.
///
/// A fixed, arbitrary constant: it only needs to be stable across runs and
/// distinct from any real UUIDv5 namespace, so two edges (or a re-seed) deriving
/// an id for the same `(series, at)` always agree. Minted once, never changed —
/// changing it would re-key every reading and break idempotent re-append.
const READING_ID_NAMESPACE: Uuid = Uuid::from_u128(0x5242_5849_5245_4144_494e_4753_0000_0001);

/// Derive the deterministic row id for a `(series, at)` pair.
///
/// `(series, at)` is the natural key of a reading, so the row id is derived from
/// it rather than minted fresh: a re-append or a sync-replay of the same sample
/// lands on the same id and is therefore an idempotent no-op
/// (`READINGS-TIMESERIES.md`, "Append identity is deterministic"). The hash is
/// **UUIDv5** (SHA-1) over the canonical `"{series}|{at}"` string, where `at` is
/// rendered in its canonical RFC3339 form — so `…:00Z` and `…:00.000Z` (the same
/// instant) collapse to one id. Sync dedup keys by id ([`SeenSet`] in
/// `rubix-sync`), so a deterministic id makes a replayed reading a no-op for free.
#[must_use]
pub fn reading_id(series: &str, at: &Datetime) -> Id {
    let name = format!("{series}|{at}");
    Id::from_raw(Uuid::new_v5(&READING_ID_NAMESPACE, name.as_bytes()).to_string())
}

/// One time-series sample: a numeric `value` a `series` produced at `at`.
///
/// `id` is **derived** from `(series, at)` (see [`reading_id`]) so it is not an
/// independent field a caller can set inconsistently — [`Reading::new`] computes
/// it. `series` is the bare id of the series-defining record (a `history:true`
/// register in NHP); the engine stores it as a `record` link, but the domain
/// keeps it a plain string so the time-series primitive carries no store-specific
/// identity (`rubix/docs/SCOPE.md`, principle 4). `at` is **measurement** time
/// (when the world produced the value); `created` is **receive** time (when we
/// persisted it) — the query layer buckets on `at`, never `created`, which closes
/// the trend-collapse bug (`READINGS-TIMESERIES.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// Deterministic id derived from `(series, at)`.
    pub id: Id,
    /// The bare id of the series-defining record this sample belongs to.
    pub series: String,
    /// The measurement instant — when the world produced the value (UTC).
    pub at: Datetime,
    /// The numeric sample.
    pub value: f64,
    /// The namespace (edge partition) this reading belongs to.
    pub namespace: String,
    /// When the reading was received/persisted (UTC) — never used as `at`.
    pub created: Datetime,
    /// Free-form extras (quality flags, source key) — empty for the common sample.
    pub content: serde_json::Value,
}

impl Reading {
    /// Build a reading, deriving its deterministic id and stamping receive time.
    ///
    /// `at` is the caller-supplied measurement instant (the sample's own
    /// timestamp, or arrival time when the source does not stamp one). `created`
    /// is set to now — receive time — so it can never stand in for `at`. The id is
    /// derived from `(series, at)` so a re-append of the same sample is idempotent.
    #[must_use]
    pub fn new(
        namespace: impl Into<String>,
        series: impl Into<String>,
        at: Datetime,
        value: f64,
        content: serde_json::Value,
    ) -> Self {
        let series = series.into();
        let id = reading_id(&series, &at);
        Self {
            id,
            series,
            at,
            value,
            namespace: namespace.into(),
            created: Datetime::now(),
            content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Reading, ReadingRow, reading_id};
    use surrealdb::types::Datetime;

    fn at(secs: i64) -> Datetime {
        Datetime::from_timestamp(secs, 0).expect("valid instant")
    }

    #[test]
    fn the_same_series_and_instant_derive_the_same_id() {
        // Idempotency hinges on this: a re-append of one sample must land the same
        // row id, so the second write is a no-op rather than a duplicate.
        let a = reading_id("reg-1", &at(1_000));
        let b = reading_id("reg-1", &at(1_000));
        assert_eq!(a, b);
    }

    #[test]
    fn a_different_series_or_instant_derives_a_different_id() {
        let base = reading_id("reg-1", &at(1_000));
        assert_ne!(base, reading_id("reg-2", &at(1_000)));
        assert_ne!(base, reading_id("reg-1", &at(2_000)));
    }

    #[test]
    fn the_same_instant_in_two_notations_derives_one_id() {
        // `…:00Z` and `…:00.000Z` are the same instant; the canonical RFC3339
        // rendering collapses them so a re-seed in either notation is idempotent.
        let terse: Datetime = "2026-06-14T10:00:00Z".parse().expect("rfc3339");
        let padded: Datetime = "2026-06-14T10:00:00.000Z".parse().expect("rfc3339");
        assert_eq!(reading_id("reg-1", &terse), reading_id("reg-1", &padded));
    }

    #[test]
    fn new_derives_its_id_and_stamps_receive_time_apart_from_at() {
        let measured = at(1_000);
        let reading = Reading::new("rubix", "reg-1", measured, 21.5, serde_json::json!({}));
        assert_eq!(reading.id, reading_id("reg-1", &measured));
        assert_eq!(reading.at, measured);
        // `created` is receive time, distinct from the measurement instant.
        assert_ne!(reading.created, measured);
    }

    #[test]
    fn reading_round_trips_through_the_persisted_row() {
        let reading = Reading::new("rubix", "reg-1", at(1_000), 21.5, serde_json::json!({}));
        let row = ReadingRow::from_reading(&reading);
        assert_eq!(row.into_reading(), reading);
    }
}
