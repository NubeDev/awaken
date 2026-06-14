//! Point command path, Postgres backend. Mirrors [`super::super::command`];
//! the priority-array and ingest semantics are shared via the parent's
//! `apply_command` / `apply_ingest`.

use chrono::{DateTime, Utc};
use rubix_core::{HisSample, Point, PointValue};
use uuid::Uuid;

use super::super::codec::{json_of, ts_of};
use super::super::command::{apply_command, apply_ingest};
use super::super::point_row::POINT_COLS;
use super::super::{Result, Store, StoreError};
use super::codec::point_of;

pub(crate) fn command_point(
    store: &Store,
    id: Uuid,
    priority: u8,
    value: Option<PointValue>,
    ts: DateTime<Utc>,
) -> Result<Point> {
    let mut client = store.postgres_conn()?;
    let mut tx = client.transaction()?;
    let mut point = load_point(&mut tx, id)?;
    let changed = apply_command(&mut point, priority, value, ts)?;
    tx.execute(
        "UPDATE points SET priority_array = $2, cur_value = $3, cur_ts = $4 WHERE id = $1",
        &[
            &id.to_string(),
            &json_of(&point.priority_array),
            &point.cur_value.as_ref().map(json_of),
            &ts_of(&ts),
        ],
    )?;
    if changed {
        if let Some(v) = &point.cur_value {
            upsert_his(&mut tx, id, ts, &json_of(v))?;
        }
    }
    tx.commit()?;
    Ok(point)
}

pub(crate) fn ingest_cur(store: &Store, id: Uuid, sample: &HisSample) -> Result<Point> {
    let mut client = store.postgres_conn()?;
    let mut tx = client.transaction()?;
    let mut point = load_point(&mut tx, id)?;
    apply_ingest(&mut point, sample)?;
    tx.execute(
        "UPDATE points SET cur_value = $2, cur_ts = $3 WHERE id = $1",
        &[&id.to_string(), &json_of(&sample.value), &ts_of(&sample.ts)],
    )?;
    upsert_his(&mut tx, id, sample.ts, &json_of(&sample.value))?;
    tx.commit()?;
    Ok(point)
}

/// `INSERT OR REPLACE` equivalent on `(point_id, ts)`.
fn upsert_his(
    tx: &mut postgres::Transaction<'_>,
    id: Uuid,
    ts: DateTime<Utc>,
    value: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO his (point_id, ts, value) VALUES ($1, $2, $3) \
         ON CONFLICT (point_id, ts) DO UPDATE SET value = EXCLUDED.value",
        &[&id.to_string(), &ts_of(&ts), &value],
    )?;
    Ok(())
}

fn load_point(tx: &mut postgres::Transaction<'_>, id: Uuid) -> Result<Point> {
    let sql = format!("SELECT {POINT_COLS} FROM points WHERE id = $1");
    let row = tx
        .query_opt(sql.as_str(), &[&id.to_string()])?
        .ok_or(StoreError::NotFound("point"))?;
    point_of(&row)
}
