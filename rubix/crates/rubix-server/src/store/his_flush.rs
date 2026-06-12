//! Move aged `his` rows out of the SQLite hot tier so the Parquet cold tier
//! can hold them. The read and delete run in one transaction, so a row is never
//! lost between tiers: it is read, handed to the caller to persist, then deleted
//! only after the caller confirms the cold-tier write succeeded.

use chrono::{DateTime, Utc};
use rubix_query::HisRow;
use uuid::Uuid;

use super::codec::{ts_of, ts_to};
use super::{Result, Store};

/// A batch of aged rows read from the hot tier, pending a cold-tier write.
///
/// The caller writes [`rows`](Self::rows) to Parquet, then calls
/// [`Store::his_delete_aged`] with the same `cutoff` to drop them from SQLite.
pub struct AgedHis {
    pub rows: Vec<HisRow>,
}

impl Store {
    /// Read every `his` row strictly older than `cutoff`, as cold-tier rows.
    ///
    /// `point_id` is stored as a UUID blob (rusqlite's `uuid` feature); it is
    /// rendered to the canonical string form so the cold-tier `point_id` matches
    /// the hot-tier provider, which stringifies the same blob. `value` is carried
    /// as its stored JSON text, identical to the cold-tier encoding, so the row
    /// round-trips through Parquet unchanged.
    pub fn his_aged(&self, cutoff: DateTime<Utc>) -> Result<AgedHis> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT point_id, ts, value FROM his WHERE ts < ?1 ORDER BY point_id, ts",
        )?;
        let cutoff_text = ts_of(&cutoff);
        let rows = stmt
            .query_map([cutoff_text], |row| {
                Ok(HisRow {
                    point_id: row.get::<_, Uuid>(0)?.to_string(),
                    ts: ts_to(&row.get::<_, String>(1)?)?,
                    value: row.get::<_, String>(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(AgedHis { rows })
    }

    /// Delete every `his` row strictly older than `cutoff`. Called only after
    /// the matching [`his_aged`](Self::his_aged) batch is durable in Parquet.
    /// Returns the number of rows removed.
    pub fn his_delete_aged(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        let conn = self.conn()?;
        let n = conn.execute("DELETE FROM his WHERE ts < ?1", [ts_of(&cutoff)])?;
        Ok(n)
    }
}
