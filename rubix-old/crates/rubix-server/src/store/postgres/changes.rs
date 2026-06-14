//! Change-ledger rows, Postgres backend. Mirrors [`super::super::changes`]. Ids
//! and timestamps are TEXT (shared codecs); `before`/`after`/`actor` are JSON
//! TEXT; `epoch` is BIGINT. The same op/snapshot validation runs server-side
//! before any insert.

use chrono::{DateTime, Utc};
use rubix_core::{Actor, Change, Op};
use uuid::Uuid;

use super::super::changes::{ChangeFilter, UndoCursor, CHANGE_COLS};
use super::super::codec::{json_of, json_to, ts_of, ts_to};
use super::super::{Result, Store, StoreError};

fn change_of(row: &postgres::Row) -> Result<Change> {
    let id = parse(row.get::<_, String>(0))?;
    let at: DateTime<Utc> =
        ts_to(&row.get::<_, String>(1)).map_err(|e| db(format!("bad change at: {e}")))?;
    let site_id = row
        .get::<_, Option<String>>(3)
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| db(format!("bad change site_id: {e}")))?;
    let actor: Actor =
        json_to(&row.get::<_, String>(4)).map_err(|e| db(format!("bad change actor: {e}")))?;
    let op = Op::parse(&row.get::<_, String>(7)).ok_or_else(|| db("unknown change op".into()))?;
    let before = decode_opt_json(row.get::<_, Option<String>>(8))?;
    let after = decode_opt_json(row.get::<_, Option<String>>(9))?;
    Ok(Change {
        id,
        at,
        org: row.get(2),
        site_id,
        actor,
        kind: row.get(5),
        resource_id: parse(row.get::<_, String>(6))?,
        op,
        before,
        after,
        group_id: parse(row.get::<_, String>(10))?,
        correlation: row.get(11),
    })
}

fn decode_opt_json(raw: Option<String>) -> Result<Option<serde_json::Value>> {
    match raw {
        Some(s) => Ok(Some(
            json_to(&s).map_err(|e| db(format!("bad change snapshot: {e}")))?,
        )),
        None => Ok(None),
    }
}

fn parse(raw: String) -> Result<Uuid> {
    Uuid::parse_str(&raw).map_err(|e| db(format!("bad change uuid `{raw}`: {e}")))
}

fn db(msg: String) -> StoreError {
    StoreError::Db(anyhow::anyhow!(msg))
}

pub(crate) fn record_change(store: &Store, change: &Change) -> Result<()> {
    let mut client = store.postgres_conn()?;
    client.execute(
        &format!(
            "INSERT INTO changes ({CHANGE_COLS}) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
        ),
        &[
            &change.id.to_string(),
            &ts_of(&change.at),
            &change.org,
            &change.site_id.map(|s| s.to_string()),
            &json_of(&change.actor),
            &change.kind,
            &change.resource_id.to_string(),
            &change.op.as_str().to_string(),
            &change.before.as_ref().map(json_of),
            &change.after.as_ref().map(json_of),
            &change.group_id.to_string(),
            &change.correlation,
        ],
    )?;
    Ok(())
}

pub(crate) fn list_changes(
    store: &Store,
    org: &str,
    filter: &ChangeFilter,
) -> Result<Vec<Change>> {
    let mut client = store.postgres_conn()?;
    let limit = if filter.limit > 0 {
        filter.limit
    } else {
        ChangeFilter::DEFAULT_LIMIT
    };
    let sql = format!(
        "SELECT {CHANGE_COLS} FROM changes \
         WHERE org = $1 \
           AND ($2::text IS NULL OR kind = $2) \
           AND ($3::text IS NULL OR resource_id = $3) \
           AND ($4::text IS NULL OR op = $4) \
           AND ($5::text IS NULL OR (actor::json ->> 'subject') = $5) \
         ORDER BY at DESC, id DESC LIMIT $6"
    );
    let rows = client.query(
        sql.as_str(),
        &[
            &org,
            &filter.kind,
            &filter.resource_id.map(|r| r.to_string()),
            &filter.op.map(|o| o.as_str().to_string()),
            &filter.actor_subject,
            &limit,
        ],
    )?;
    rows.iter().map(change_of).collect()
}

pub(crate) fn changes_in_group(store: &Store, group_id: Uuid) -> Result<Vec<Change>> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {CHANGE_COLS} FROM changes WHERE group_id = $1 ORDER BY at DESC, id DESC"
    );
    let rows = client.query(sql.as_str(), &[&group_id.to_string()])?;
    rows.iter().map(change_of).collect()
}

pub(crate) fn newest_undoable_group(
    store: &Store,
    org: &str,
    subject: &str,
    undone: &[Uuid],
) -> Result<Option<Uuid>> {
    let mut client = store.postgres_conn()?;
    let rows = client.query(
        "SELECT group_id FROM changes \
         WHERE org = $1 AND (actor::json ->> 'subject') = $2 \
         ORDER BY at DESC, id DESC",
        &[&org, &subject],
    )?;
    for row in &rows {
        let g = parse(row.get::<_, String>(0))?;
        if !undone.contains(&g) {
            return Ok(Some(g));
        }
    }
    Ok(None)
}

pub(crate) fn undo_cursor(store: &Store, org: &str, subject: &str) -> Result<UndoCursor> {
    let mut client = store.postgres_conn()?;
    let row = client.query_opt(
        "SELECT redo_stack, epoch FROM undo_cursors WHERE org = $1 AND subject = $2",
        &[&org, &subject],
    )?;
    match row {
        Some(r) => {
            let stack: String = r.get(0);
            Ok(UndoCursor {
                redo_stack: json_to(&stack).map_err(|e| db(format!("bad redo_stack: {e}")))?,
                epoch: r.get(1),
            })
        }
        None => Ok(UndoCursor::default()),
    }
}

pub(crate) fn cas_undo_cursor(
    store: &Store,
    org: &str,
    subject: &str,
    expected_epoch: i64,
    next_stack_json: &str,
) -> Result<bool> {
    let mut client = store.postgres_conn()?;
    let n = client.execute(
        "INSERT INTO undo_cursors (org, subject, redo_stack, epoch) \
         VALUES ($1, $2, $3, $4 + 1) \
         ON CONFLICT (org, subject) DO UPDATE SET \
           redo_stack = excluded.redo_stack, epoch = undo_cursors.epoch + 1 \
         WHERE undo_cursors.epoch = $4",
        &[&org, &subject, &next_stack_json.to_string(), &expected_epoch],
    )?;
    Ok(n == 1)
}
