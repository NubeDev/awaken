//! Run rows, Postgres backend. Mirrors [`super::super::runs`]. The
//! [`RunRecord`] status/origin validation lives in the domain types; the
//! suspended-settle guard is shared via the parent's `ensure_suspended`.

use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};

use super::super::codec::{json_of, json_to, ts_of};
use super::super::runs::{ensure_suspended, RUN_COLS};
use super::super::{Result, Store, StoreError};
use super::codec::ts_col;

fn run_of(row: &postgres::Row) -> Result<RunRecord> {
    let origin_raw: String = row.get(2);
    let status_raw: String = row.get(3);
    let pending_raw: Option<String> = row.get(6);
    let pending_write = match pending_raw {
        Some(json) => Some(
            json_to::<PendingWrite>(&json)
                .map_err(|e| StoreError::Db(anyhow::anyhow!("bad pending_write: {e}")))?,
        ),
        None => None,
    };
    Ok(RunRecord {
        id: row.get(0),
        thread_id: row.get(1),
        origin: RunOrigin::parse(&origin_raw)
            .ok_or_else(|| StoreError::Db(anyhow::anyhow!("unknown run origin `{origin_raw}`")))?,
        status: RunStatus::parse(&status_raw)
            .ok_or_else(|| StoreError::Db(anyhow::anyhow!("unknown run status `{status_raw}`")))?,
        response: row.get(4),
        steps: row.get::<_, i64>(5) as usize,
        pending_write,
        created_at: ts_col(row, 7)?,
        updated_at: ts_col(row, 8)?,
    })
}

pub(crate) fn create_run(store: &Store, run: &RunRecord) -> Result<()> {
    let pending = run.pending_write.as_ref().map(json_of);
    store.postgres_conn()?.execute(
        "INSERT INTO runs (id, thread_id, origin, status, response, steps, pending_write, \
         created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &[
            &run.id,
            &run.thread_id,
            &run.origin.to_string(),
            &run.status.to_string(),
            &run.response,
            &(run.steps as i64),
            &pending,
            &ts_of(&run.created_at),
            &ts_of(&run.updated_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_runs(store: &Store, status: Option<RunStatus>) -> Result<Vec<RunRecord>> {
    let mut client = store.postgres_conn()?;
    let status = status.map(|s| s.to_string());
    let sql = format!(
        "SELECT {RUN_COLS} FROM runs WHERE ($1::text IS NULL OR status = $1) \
         ORDER BY created_at DESC"
    );
    let rows = client.query(sql.as_str(), &[&status])?;
    rows.iter().map(run_of).collect()
}

pub(crate) fn get_run(store: &Store, id: &str) -> Result<RunRecord> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {RUN_COLS} FROM runs WHERE id = $1");
    let row = client
        .query_opt(sql.as_str(), &[&id])?
        .ok_or(StoreError::NotFound("run"))?;
    run_of(&row)
}

pub(crate) fn settle_suspended_run(
    store: &Store,
    id: &str,
    status: RunStatus,
) -> Result<RunRecord> {
    let mut client = store.postgres_conn()?;
    let mut tx = client.transaction()?;
    let sql = format!("SELECT {RUN_COLS} FROM runs WHERE id = $1");
    let row = tx
        .query_opt(sql.as_str(), &[&id])?
        .ok_or(StoreError::NotFound("run"))?;
    let run = run_of(&row)?;
    ensure_suspended(id, &run)?;
    tx.execute(
        "UPDATE runs SET status = $2, pending_write = NULL, updated_at = $3 WHERE id = $1",
        &[&id, &status.to_string(), &ts_of(&chrono::Utc::now())],
    )?;
    tx.commit()?;
    Ok(run)
}
