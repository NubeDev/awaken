//! The persisted shape of a [`Reading`] at the SurrealDB boundary.
//!
//! SurrealDB owns the reserved `id` field as a `RecordId` and stores `series` as
//! a `record` link (the DDL declares `series ON reading TYPE record`,
//! `rubix/docs/design/READINGS-TIMESERIES.md`); the domain [`Reading`] carries
//! plain string ids on both. This row type maps between the two so the domain
//! stays free of store-specific identity while the write still keys on the
//! deterministic reading id and links `series` to its register record.

use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue, ToSql};

use crate::id::Id;

use super::{READING_TABLE, Reading};

/// The `record` table a `series` link points at (the series-defining register).
const SERIES_TABLE: &str = "record";

/// SurrealDB-facing reading: the reserved `id` thing, the `series` link, and the
/// reading fields.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct ReadingRow {
    pub(crate) id: RecordId,
    pub(crate) series: RecordId,
    pub(crate) at: Datetime,
    pub(crate) value: f64,
    pub(crate) namespace: String,
    pub(crate) created: Datetime,
    pub(crate) content: serde_json::Value,
}

impl ReadingRow {
    /// Project a domain [`Reading`] into its persisted row.
    pub(crate) fn from_reading(reading: &Reading) -> Self {
        Self {
            id: RecordId::new(READING_TABLE, reading.id.as_str()),
            series: RecordId::new(SERIES_TABLE, reading.series.as_str()),
            at: reading.at,
            value: reading.value,
            namespace: reading.namespace.clone(),
            created: reading.created,
            content: reading.content.clone(),
        }
    }

    /// Reconstruct the domain [`Reading`] from a persisted row.
    pub(crate) fn into_reading(self) -> Reading {
        Reading {
            id: Id::from_raw(key_string(&self.id)),
            series: key_string(&self.series),
            at: self.at,
            value: self.value,
            namespace: self.namespace,
            created: self.created,
            content: self.content,
        }
    }
}

/// The raw string form of a record id's key (the part after `table:`).
///
/// Ids on both `id` and `series` are minted from string [`Id`]s, so the key is
/// always a string; other key shapes are coerced through their SurrealQL form for
/// completeness. Decoding through the typed `RecordId` (not the JSON `table:key`
/// rendering) yields the bare key without SurrealDB's bracket-quoting, so the
/// domain `series` matches a register's bare id for a direct `series == id` join.
fn key_string(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => other.to_sql(),
    }
}
