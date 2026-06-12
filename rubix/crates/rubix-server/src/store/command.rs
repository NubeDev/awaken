//! Point command path: priority-array writes, relinquish, sensor ingest.
//! Every change to the effective value lands in history. Backend dispatch;
//! SQLite body inline, Postgres body in [`super::postgres::command`].

use chrono::{DateTime, Utc};
use rubix_core::{HisSample, Point, PointValue};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_of, ts_of};
use super::point_row::{row_point, POINT_COLS};
use super::{Result, Store, StoreError};

impl Store {
    /// Set (`Some`) or relinquish (`None`) a priority slot, recompute the
    /// effective value, persist it as current, and log changes to history.
    pub fn command_point(
        &self,
        id: Uuid,
        priority: u8,
        value: Option<PointValue>,
        ts: DateTime<Utc>,
    ) -> Result<Point> {
        match &self.backend {
            Backend::Sqlite(_) => self.command_point_sqlite(id, priority, value, ts),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::command::command_point(self, id, priority, value, ts)
            }
        }
    }

    fn command_point_sqlite(
        &self,
        id: Uuid,
        priority: u8,
        value: Option<PointValue>,
        ts: DateTime<Utc>,
    ) -> Result<Point> {
        let mut conn = self.sqlite_conn()?;
        let tx = conn.transaction()?;
        let mut point = load_point(&tx, id)?;
        let changed = apply_command(&mut point, priority, value, ts)?;
        tx.execute(
            "UPDATE points SET priority_array = ?2, cur_value = ?3, cur_ts = ?4 WHERE id = ?1",
            params![
                id,
                json_of(&point.priority_array),
                point.cur_value.as_ref().map(json_of),
                ts_of(&ts)
            ],
        )?;
        if changed {
            if let Some(v) = &point.cur_value {
                tx.execute(
                    "INSERT OR REPLACE INTO his (point_id, ts, value) VALUES (?1, ?2, ?3)",
                    params![id, ts_of(&ts), json_of(v)],
                )?;
            }
        }
        tx.commit()?;
        Ok(point)
    }

    /// Record a sensor sample as the current value and log it to history.
    pub fn ingest_cur(&self, id: Uuid, sample: &HisSample) -> Result<Point> {
        match &self.backend {
            Backend::Sqlite(_) => self.ingest_cur_sqlite(id, sample),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::command::ingest_cur(self, id, sample),
        }
    }

    fn ingest_cur_sqlite(&self, id: Uuid, sample: &HisSample) -> Result<Point> {
        let mut conn = self.sqlite_conn()?;
        let tx = conn.transaction()?;
        let mut point = load_point(&tx, id)?;
        apply_ingest(&mut point, sample)?;
        tx.execute(
            "UPDATE points SET cur_value = ?2, cur_ts = ?3 WHERE id = ?1",
            params![id, json_of(&sample.value), ts_of(&sample.ts)],
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO his (point_id, ts, value) VALUES (?1, ?2, ?3)",
            params![id, ts_of(&sample.ts), json_of(&sample.value)],
        )?;
        tx.commit()?;
        Ok(point)
    }
}

/// Apply a priority-slot command to a loaded point in memory. Returns whether
/// the effective value changed (so the caller logs history). Shared by both
/// backends so the priority-array semantics live in one place.
pub(crate) fn apply_command(
    point: &mut Point,
    priority: u8,
    value: Option<PointValue>,
    ts: DateTime<Utc>,
) -> Result<bool> {
    if !point.kind.is_writable() {
        return Err(StoreError::Invalid(format!(
            "point `{}` is a sensor and cannot be commanded",
            point.slug
        )));
    }
    match value {
        Some(v) => point
            .priority_array
            .set(priority, v)
            .map_err(|e| StoreError::Invalid(e.to_string()))?,
        None => {
            point
                .priority_array
                .relinquish(priority)
                .map_err(|e| StoreError::Invalid(e.to_string()))?;
        }
    }
    let effective = point.priority_array.effective().map(|(_, v)| v.clone());
    let changed = effective != point.cur_value;
    point.cur_value = effective;
    point.cur_ts = Some(ts);
    Ok(changed)
}

/// Apply a sensor ingest to a loaded point in memory. Shared by both backends.
pub(crate) fn apply_ingest(point: &mut Point, sample: &HisSample) -> Result<()> {
    if point.kind.is_writable() {
        return Err(StoreError::Invalid(format!(
            "point `{}` is writable; use the write endpoint",
            point.slug
        )));
    }
    point.cur_value = Some(sample.value.clone());
    point.cur_ts = Some(sample.ts);
    Ok(())
}

fn load_point(tx: &rusqlite::Transaction<'_>, id: Uuid) -> Result<Point> {
    tx.query_row(
        &format!("SELECT {POINT_COLS} FROM points WHERE id = ?1"),
        params![id],
        row_point,
    )
    .optional()?
    .ok_or(StoreError::NotFound("point"))
}
