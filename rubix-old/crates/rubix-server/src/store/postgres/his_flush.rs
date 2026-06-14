//! Aged-history flush, Postgres backend. Mirrors [`super::super::his_flush`].
//! `point_id` is already canonical TEXT here, so it maps straight to the
//! cold-tier string.

use chrono::{DateTime, Utc};
use rubix_query::HisRow;

use super::super::codec::ts_of;
use super::super::his_flush::AgedHis;
use super::super::{Result, Store};
use super::codec::ts_col;

pub(crate) fn his_aged(store: &Store, cutoff: DateTime<Utc>) -> Result<AgedHis> {
    let mut client = store.postgres_conn()?;
    let rows = client.query(
        "SELECT point_id, ts, value FROM his WHERE ts < $1 ORDER BY point_id, ts",
        &[&ts_of(&cutoff)],
    )?;
    let rows = rows
        .iter()
        .map(|row| {
            Ok(HisRow {
                point_id: row.get::<_, String>(0),
                ts: ts_col(row, 1)?,
                value: row.get::<_, String>(2),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(AgedHis { rows })
}

pub(crate) fn his_delete_aged(store: &Store, cutoff: DateTime<Utc>) -> Result<usize> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM his WHERE ts < $1", &[&ts_of(&cutoff)])?;
    Ok(n as usize)
}
