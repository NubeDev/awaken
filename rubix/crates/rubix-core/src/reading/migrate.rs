//! One-shot migration: legacy `kind:"history"` records → the `reading` plane.
//!
//! Before the readings/time-series plane existed, captured history was stored as
//! ordinary `kind:"history"` records in the `record` table
//! (`rubix/docs/design/READINGS-TIMESERIES.md`, "Migration from `kind:"history"`").
//! This moves any such records once into the data plane: read each, map
//! `content.ts → at`, `content.register → series`, `content.value → value`, append
//! to `reading`, then delete the migrated record — leaving the `record` table
//! holding only config/document records, so the plane split is clean.
//!
//! It is **safe to re-run**. The append is keyed by the deterministic `(series,
//! at)` id, so a re-append (or a crash mid-migration) re-lands the same rows
//! idempotently; the delete of an already-gone record is a no-op. It is also
//! **defensive**: a record that is not well-formed history (missing/!typed `ts`,
//! `register`, or `value`) is left in place and counted as skipped, never appended
//! as a junk reading and never deleted. For NHP the cheaper path is a re-seed (the
//! synthetic history has no production value); this command is the safe path for
//! any *real* captured history.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::Datetime;

use crate::error::Result;
use crate::id::Id;
use crate::record::{Record, delete_record, list_records_filtered};

use super::{Reading, append_readings};

/// The `content.kind` legacy captured-history records carried before the plane.
const HISTORY_KIND: &str = "history";

/// The outcome of a [`migrate_history_to_readings`] run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HistoryMigration {
    /// Readings appended into the data plane (one per migrated record).
    pub migrated: u64,
    /// Legacy `kind:"history"` records deleted after their reading was appended.
    pub deleted: u64,
    /// Records left in place because they were not well-formed history.
    pub skipped: u64,
}

/// Migrate every visible `kind:"history"` record into the `reading` plane,
/// returning what happened.
///
/// Append-then-delete per migrated record: the reading is written first (idempotent
/// by `(series, at)` id) and only a successfully-mapped record is deleted, so a
/// crash leaves the source record in place to be re-migrated on the next run.
/// Malformed records are skipped, not appended or deleted. Runs on whatever handle
/// is passed — the root/owner handle for a maintenance migration, off the gate.
///
/// # Errors
/// Returns [`Error::Store`](crate::Error::Store) if a read, append, or delete fails.
pub async fn migrate_history_to_readings(db: &Surreal<Db>) -> Result<HistoryMigration> {
    let records = list_records_filtered(db, Some(HISTORY_KIND), &[]).await?;

    let mut report = HistoryMigration::default();
    let mut mapped: Vec<(Id, Reading)> = Vec::new();
    for record in &records {
        match history_reading(record) {
            Some(reading) => mapped.push((record.id.clone(), reading)),
            None => report.skipped += 1,
        }
    }

    if mapped.is_empty() {
        return Ok(report);
    }

    let readings: Vec<Reading> = mapped.iter().map(|(_, reading)| reading.clone()).collect();
    append_readings(db, &readings).await?;
    report.migrated = readings.len() as u64;

    // Only delete after the batch landed, so an append failure above leaves every
    // source record intact for a clean re-run.
    for (record_id, _) in &mapped {
        delete_record(db, record_id).await?;
        report.deleted += 1;
    }

    Ok(report)
}

/// Map one `kind:"history"` record to a [`Reading`], or `None` if it is not
/// well-formed history.
///
/// `series` is `content.register`, `at` is `content.ts` (RFC 3339), `value` is
/// `content.value` — the documented mapping. The namespace comes from the record,
/// never the content. Content stays lean (`{}`): display metadata lives on the
/// register record the `series` points at, not copied onto every sample.
fn history_reading(record: &Record) -> Option<Reading> {
    let content = &record.content;
    let series = content.get("register")?.as_str()?;
    let at: Datetime = content.get("ts")?.as_str()?.parse().ok()?;
    let value = content.get("value")?.as_f64()?;
    Some(Reading::new(
        &record.namespace,
        series,
        at,
        value,
        serde_json::json!({}),
    ))
}
