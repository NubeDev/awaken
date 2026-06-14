//! History rows, Postgres backend. Mirrors [`super::super::his`].

use chrono::{DateTime, Utc};
use rubix_core::HisSample;
use uuid::Uuid;

use super::super::codec::{json_of, ts_of};
use super::super::{Result, Store};
use super::codec::{json_col, require, ts_col};

pub(crate) fn his_insert(store: &Store, id: Uuid, samples: &[HisSample]) -> Result<usize> {
    let mut client = store.postgres_conn()?;
    let mut tx = client.transaction()?;
    require(&mut tx, "points", "point", id)?;
    let mut n = 0;
    for s in samples {
        n += tx.execute(
            "INSERT INTO his (point_id, ts, value) VALUES ($1, $2, $3) \
             ON CONFLICT (point_id, ts) DO UPDATE SET value = EXCLUDED.value",
            &[&id.to_string(), &ts_of(&s.ts), &json_of(&s.value)],
        )?;
    }
    tx.commit()?;
    Ok(n as usize)
}

pub(crate) fn his_query(
    store: &Store,
    id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: usize,
) -> Result<Vec<HisSample>> {
    let mut client = store.postgres_conn()?;
    require(&mut *client, "points", "point", id)?;
    let start_t = start.as_ref().map(ts_of);
    let end_t = end.as_ref().map(ts_of);
    let rows = client.query(
        "SELECT ts, value FROM his WHERE point_id = $1 \
         AND ($2::text IS NULL OR ts >= $2) AND ($3::text IS NULL OR ts < $3) \
         ORDER BY ts LIMIT $4",
        &[&id.to_string(), &start_t, &end_t, &(limit as i64)],
    )?;
    rows.iter()
        .map(|row| {
            Ok(HisSample {
                ts: ts_col(row, 0)?,
                value: json_col(row, 1)?,
            })
        })
        .collect()
}
