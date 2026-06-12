//! History rows: batch insert and range query.

use chrono::{DateTime, Utc};
use rubix_core::HisSample;
use rusqlite::params;
use uuid::Uuid;

use super::codec::{json_of, json_to, ts_of, ts_to};
use super::{Result, Store};

impl Store {
    pub fn his_insert(&self, id: Uuid, samples: &[HisSample]) -> Result<usize> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        Self::require_point(&tx, id)?;
        let mut n = 0;
        {
            let mut stmt =
                tx.prepare("INSERT OR REPLACE INTO his (point_id, ts, value) VALUES (?1, ?2, ?3)")?;
            for s in samples {
                n += stmt.execute(params![id, ts_of(&s.ts), json_of(&s.value)])?;
            }
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn his_query(
        &self,
        id: Uuid,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<HisSample>> {
        let conn = self.conn()?;
        Self::require_point(&conn, id)?;
        let mut stmt = conn.prepare(
            "SELECT ts, value FROM his WHERE point_id = ?1 \
             AND (?2 IS NULL OR ts >= ?2) AND (?3 IS NULL OR ts < ?3) \
             ORDER BY ts LIMIT ?4",
        )?;
        let rows = stmt.query_map(
            params![
                id,
                start.as_ref().map(ts_of),
                end.as_ref().map(ts_of),
                limit as i64
            ],
            |row| {
                Ok(HisSample {
                    ts: ts_to(&row.get::<_, String>(0)?)?,
                    value: json_to(&row.get::<_, String>(1)?)?,
                })
            },
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
