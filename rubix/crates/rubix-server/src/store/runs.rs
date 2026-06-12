//! Agent run rows: persist a run on creation, update its status, and read it
//! back for the operator surface. A suspended run carries the [`PendingWrite`]
//! the `write_point` tool held for approval (JSON in `pending_write`); resume
//! and cancel transition the row out of `suspended`.

use rusqlite::{params, OptionalExtension, Row};

use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};

use super::codec::{json_of, json_to, ts_of, ts_to};
use super::{Result, Store, StoreError};

const RUN_COLS: &str =
    "id, thread_id, origin, status, response, steps, pending_write, created_at, updated_at";

fn row_run(row: &Row<'_>) -> rusqlite::Result<RunRecord> {
    let origin_raw: String = row.get(2)?;
    let status_raw: String = row.get(3)?;
    let pending_raw: Option<String> = row.get(6)?;
    let pending_write = match pending_raw {
        Some(json) => Some(json_to::<PendingWrite>(&json)?),
        None => None,
    };
    Ok(RunRecord {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        origin: RunOrigin::parse(&origin_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                format!("unknown run origin `{origin_raw}`").into(),
            )
        })?,
        status: RunStatus::parse(&status_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                format!("unknown run status `{status_raw}`").into(),
            )
        })?,
        response: row.get(4)?,
        steps: row.get::<_, i64>(5)? as usize,
        pending_write,
        created_at: ts_to(&row.get::<_, String>(7)?)?,
        updated_at: ts_to(&row.get::<_, String>(8)?)?,
    })
}

impl Store {
    /// Persist a freshly-finished run. `pending_write` is set only for a run
    /// that suspended in the escalation band.
    pub fn create_run(&self, run: &RunRecord) -> Result<()> {
        self.conn()?.execute(
            "INSERT INTO runs (id, thread_id, origin, status, response, steps, pending_write, \
             created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                run.id,
                run.thread_id,
                run.origin.to_string(),
                run.status.to_string(),
                run.response,
                run.steps as i64,
                run.pending_write.as_ref().map(json_of),
                ts_of(&run.created_at),
                ts_of(&run.updated_at)
            ],
        )?;
        Ok(())
    }

    /// All runs, newest first, optionally filtered by status.
    pub fn list_runs(&self, status: Option<RunStatus>) -> Result<Vec<RunRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {RUN_COLS} FROM runs WHERE (?1 IS NULL OR status = ?1) \
             ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map(params![status.map(|s| s.to_string())], row_run)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// One run by id.
    pub fn get_run(&self, id: &str) -> Result<RunRecord> {
        self.conn()?
            .query_row(
                &format!("SELECT {RUN_COLS} FROM runs WHERE id = ?1"),
                params![id],
                row_run,
            )
            .optional()?
            .ok_or(StoreError::NotFound("run"))
    }

    /// Transition a run out of `suspended`, clearing its held write. Returns the
    /// run as it was *before* the update (so the caller can act on the pending
    /// write). Fails if the run is absent or no longer suspended, so a
    /// double-resume cannot apply the held write twice.
    pub fn settle_suspended_run(&self, id: &str, status: RunStatus) -> Result<RunRecord> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let run: RunRecord = tx
            .query_row(
                &format!("SELECT {RUN_COLS} FROM runs WHERE id = ?1"),
                params![id],
                row_run,
            )
            .optional()?
            .ok_or(StoreError::NotFound("run"))?;
        if run.status != RunStatus::Suspended {
            return Err(StoreError::Conflict(format!(
                "run `{id}` is `{}`, not suspended",
                run.status
            )));
        }
        tx.execute(
            "UPDATE runs SET status = ?2, pending_write = NULL, updated_at = ?3 WHERE id = ?1",
            params![id, status.to_string(), ts_of(&chrono::Utc::now())],
        )?;
        tx.commit()?;
        Ok(run)
    }
}
