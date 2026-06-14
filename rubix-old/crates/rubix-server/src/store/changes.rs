//! The change-ledger write/read path (docs/design/audit-and-undo.md "Recording",
//! "Audit read surface", "Undo/Redo"). [`Store`] is the sole write path for the
//! `changes` table; `record` appends one validated row, and a transaction-scoped
//! variant lets a mutation handler (WS-08) commit the change atomically with the
//! mutation it describes. Every id/timestamp is bound as a parameter (TEXT, shared
//! across both dialects); free-form snapshot text never reaches SQL unbound.
//!
//! Backend dispatch lives here; the SQLite body is inline, the Postgres body in
//! [`super::postgres::changes`].

use chrono::Utc;
use rubix_core::{Actor, Change, Op};
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_of, json_to, ts_of, ts_to};
use super::{Result, Store, StoreError};

pub(crate) const CHANGE_COLS: &str =
    "id, at, org, site_id, actor, kind, resource_id, op, before, after, group_id, correlation";

/// Filter for the audit read surface (`GET /api/v1/audit`). Every field is an
/// optional narrowing; `None` matches all. `org` is supplied separately and is
/// always enforced — a cross-org read is impossible by construction.
#[derive(Debug, Clone, Default)]
pub struct ChangeFilter {
    pub kind: Option<String>,
    pub resource_id: Option<Uuid>,
    pub actor_subject: Option<String>,
    pub op: Option<Op>,
    pub limit: i64,
}

impl ChangeFilter {
    /// A sane default page size when a caller does not cap explicitly.
    pub const DEFAULT_LIMIT: i64 = 200;
}

fn row_change(row: &Row<'_>) -> rusqlite::Result<Change> {
    let id = parse_uuid(&row.get::<_, String>(0)?)?;
    let site_id = match row.get::<_, Option<String>>(3)? {
        Some(s) => Some(parse_uuid(&s)?),
        None => None,
    };
    let actor: Actor = json_to(&row.get::<_, String>(4)?)?;
    let op = Op::parse(&row.get::<_, String>(7)?).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            "unknown change op".into(),
        )
    })?;
    let before = match row.get::<_, Option<String>>(8)? {
        Some(s) => Some(json_to(&s)?),
        None => None,
    };
    let after = match row.get::<_, Option<String>>(9)? {
        Some(s) => Some(json_to(&s)?),
        None => None,
    };
    Ok(Change {
        id,
        at: ts_to(&row.get::<_, String>(1)?)?,
        org: row.get(2)?,
        site_id,
        actor,
        kind: row.get(5)?,
        resource_id: parse_uuid(&row.get::<_, String>(6)?)?,
        op,
        before,
        after,
        group_id: parse_uuid(&row.get::<_, String>(10)?)?,
        correlation: row.get(11)?,
    })
}

fn parse_uuid(raw: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

impl Store {
    /// Append one change row. The sole change write path. Validates op/snapshot
    /// consistency first (a `before`-less update is rejected here, the type-level
    /// half of the coverage guard) and opens its own transaction. Mutation
    /// handlers that need atomicity with their own write use
    /// [`record_in_sqlite_tx`](Self::record_in_sqlite_tx) instead (WS-08).
    pub fn record_change(&self, change: &Change) -> Result<()> {
        change
            .validate()
            .map_err(|e| StoreError::Invalid(e.to_string()))?;
        match &self.backend {
            Backend::Sqlite(_) => {
                let mut conn = self.sqlite_conn()?;
                let tx = conn.transaction()?;
                Self::insert_change_sqlite(&tx, change)?;
                tx.commit()?;
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::changes::record_change(self, change),
        }
    }

    /// Append a change inside a caller-owned SQLite transaction, so it commits
    /// atomically with the mutation that produced it (docs/design/audit-and-undo.md
    /// "Recording"). Validates first.
    // The substrate's transactional record path; WS-08 wires it into each mutation
    // handler. Exercised by the changes unit tests in the meantime.
    #[allow(dead_code)]
    pub(crate) fn record_in_sqlite_tx(
        tx: &rusqlite::Transaction<'_>,
        change: &Change,
    ) -> Result<()> {
        change
            .validate()
            .map_err(|e| StoreError::Invalid(e.to_string()))?;
        Self::insert_change_sqlite(tx, change)
    }

    fn insert_change_sqlite(tx: &rusqlite::Transaction<'_>, change: &Change) -> Result<()> {
        let before = change.before.as_ref().map(json_of);
        let after = change.after.as_ref().map(json_of);
        tx.execute(
            &format!(
                "INSERT INTO changes ({CHANGE_COLS}) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
            ),
            params![
                change.id.to_string(),
                ts_of(&change.at),
                change.org,
                change.site_id.map(|s| s.to_string()),
                json_of(&change.actor),
                change.kind,
                change.resource_id.to_string(),
                change.op.as_str(),
                before,
                after,
                change.group_id.to_string(),
                change.correlation,
            ],
        )?;
        Ok(())
    }

    /// Query the audit ledger, newest-first, org-scoped and filtered. Powers
    /// `GET /api/v1/audit` (WS-08) and the coverage guard.
    pub fn list_changes(&self, org: &str, filter: &ChangeFilter) -> Result<Vec<Change>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_changes_sqlite(org, filter),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::changes::list_changes(self, org, filter),
        }
    }

    fn list_changes_sqlite(&self, org: &str, filter: &ChangeFilter) -> Result<Vec<Change>> {
        let conn = self.sqlite_conn()?;
        // Every predicate binds; the optional filters use a `?N IS NULL OR col = ?N`
        // sentinel so one prepared statement covers every combination.
        let limit = if filter.limit > 0 {
            filter.limit
        } else {
            ChangeFilter::DEFAULT_LIMIT
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT {CHANGE_COLS} FROM changes \
             WHERE org = ?1 \
               AND (?2 IS NULL OR kind = ?2) \
               AND (?3 IS NULL OR resource_id = ?3) \
               AND (?4 IS NULL OR op = ?4) \
               AND (?5 IS NULL OR json_extract(actor, '$.subject') = ?5) \
             ORDER BY at DESC, id DESC LIMIT ?6"
        ))?;
        let rows = stmt.query_map(
            params![
                org,
                filter.kind,
                filter.resource_id.map(|r| r.to_string()),
                filter.op.map(|o| o.as_str()),
                filter.actor_subject,
                limit,
            ],
            row_change,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The full timeline of one resource, newest-first (powers `GET
    /// /audit/{kind}/{id}`). Org-scoped.
    pub fn resource_changes(&self, org: &str, kind: &str, id: Uuid) -> Result<Vec<Change>> {
        self.list_changes(
            org,
            &ChangeFilter {
                kind: Some(kind.to_string()),
                resource_id: Some(id),
                limit: ChangeFilter::DEFAULT_LIMIT,
                ..Default::default()
            },
        )
    }

    /// Every change row of one `group_id`, newest-first within the group. The unit
    /// the reverser replays as one undo step.
    pub fn changes_in_group(&self, group_id: Uuid) -> Result<Vec<Change>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let mut stmt = conn.prepare(&format!(
                    "SELECT {CHANGE_COLS} FROM changes WHERE group_id = ?1 \
                     ORDER BY at DESC, id DESC"
                ))?;
                let rows = stmt.query_map(params![group_id.to_string()], row_change)?;
                Ok(rows.collect::<rusqlite::Result<_>>()?)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::changes::changes_in_group(self, group_id),
        }
    }

    /// The most-recent change group the actor made that has not been undone — the
    /// next group `POST /undo` pops. `None` when the actor has nothing to undo.
    /// Org-scoped and per-actor (it never sees another actor's groups).
    pub fn newest_undoable_group(&self, org: &str, subject: &str) -> Result<Option<Uuid>> {
        let undone = self.undo_cursor(org, subject)?.redo_stack;
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                // Newest group first; skip any already on the redo stack (undone).
                let mut stmt = conn.prepare(
                    "SELECT group_id FROM changes \
                     WHERE org = ?1 AND json_extract(actor, '$.subject') = ?2 \
                     ORDER BY at DESC, id DESC",
                )?;
                let mut rows = stmt.query(params![org, subject])?;
                while let Some(row) = rows.next()? {
                    let g = parse_uuid(&row.get::<_, String>(0)?)?;
                    if !undone.contains(&g) {
                        return Ok(Some(g));
                    }
                }
                Ok(None)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::changes::newest_undoable_group(self, org, subject, &undone)
            }
        }
    }
}

/// A per-actor undo cursor row (docs/design/audit-and-undo.md "Undo/Redo"):
/// `redo_stack` is the LIFO of undone `group_id`s, `epoch` the CAS guard.
#[derive(Debug, Clone, Default)]
pub struct UndoCursor {
    pub redo_stack: Vec<Uuid>,
    pub epoch: i64,
}

impl Store {
    /// Read the actor's cursor, defaulting to an empty stack at epoch 0 when the
    /// actor has never undone anything.
    pub fn undo_cursor(&self, org: &str, subject: &str) -> Result<UndoCursor> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let row = conn
                    .query_row(
                        "SELECT redo_stack, epoch FROM undo_cursors \
                         WHERE org = ?1 AND subject = ?2",
                        params![org, subject],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
                    )
                    .optional()?;
                match row {
                    Some((stack, epoch)) => Ok(UndoCursor {
                        redo_stack: json_to(&stack)?,
                        epoch,
                    }),
                    None => Ok(UndoCursor::default()),
                }
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::changes::undo_cursor(self, org, subject),
        }
    }

    /// Compare-and-set the actor's cursor: write `next` only when the stored epoch
    /// still equals `expected_epoch`, bumping the epoch on success. A racing undo
    /// that already advanced the epoch makes this a no-op, returning `false` so the
    /// caller retries or reports a conflict (the design's double-pop guard).
    pub fn cas_undo_cursor(
        &self,
        org: &str,
        subject: &str,
        expected_epoch: i64,
        next_stack: &[Uuid],
    ) -> Result<bool> {
        let stack_json = json_of(&next_stack.to_vec());
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                // UPSERT keyed on (org, subject): insert the first cursor row, or
                // update only when the epoch matches. The WHERE on UPDATE is the CAS.
                let n = conn.execute(
                    "INSERT INTO undo_cursors (org, subject, redo_stack, epoch) \
                     VALUES (?1, ?2, ?3, ?4 + 1) \
                     ON CONFLICT (org, subject) DO UPDATE SET \
                       redo_stack = excluded.redo_stack, epoch = undo_cursors.epoch + 1 \
                     WHERE undo_cursors.epoch = ?4",
                    params![org, subject, stack_json, expected_epoch],
                )?;
                Ok(n == 1)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::changes::cas_undo_cursor(
                self,
                org,
                subject,
                expected_epoch,
                &stack_json,
            ),
        }
    }

    /// Record the moment a fresh edit clears the actor's redo stack (standard undo
    /// semantics: a new change invalidates any previously-undone groups). A no-op
    /// when nothing was undone. Called by WS-08's recorder after a user edit.
    pub fn clear_redo_stack(&self, org: &str, subject: &str) -> Result<()> {
        let cursor = self.undo_cursor(org, subject)?;
        if cursor.redo_stack.is_empty() {
            return Ok(());
        }
        // CAS so a concurrent undo is not silently lost; one retry on contention.
        for _ in 0..2 {
            let cursor = self.undo_cursor(org, subject)?;
            if cursor.redo_stack.is_empty() {
                return Ok(());
            }
            if self.cas_undo_cursor(org, subject, cursor.epoch, &[])? {
                return Ok(());
            }
        }
        Ok(())
    }
}

/// A fresh `group_id` for a logical operation. One operation's rows (e.g. a cascade
/// delete) share this so they undo as one step (docs/design/audit-and-undo.md
/// "group_id groups a transaction").
pub fn new_group_id() -> Uuid {
    Uuid::new_v4()
}

/// A monotonic-ish `(at, id)` pair for a fresh change row. `at` is the UTC commit
/// instant; `id` is unique. Recording sites use this so ordering is stable.
pub fn new_change_id() -> (Uuid, chrono::DateTime<Utc>) {
    (Uuid::new_v4(), Utc::now())
}

#[cfg(test)]
mod tests {
    use rubix_core::Change;
    use uuid::Uuid;

    use crate::store::Store;

    fn store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("changes.db")).unwrap();
        (store, dir)
    }

    fn create(org: &str, subject: &str, kind: &str, group: Uuid) -> Change {
        let (id, at) = super::new_change_id();
        Change::create(
            id,
            at,
            org,
            None,
            rubix_core::Actor::User { subject: subject.into() },
            kind,
            Uuid::new_v4(),
            serde_json::json!({"x": 1}),
            group,
            None,
        )
    }

    #[test]
    fn record_in_tx_commits_atomically_with_the_caller() {
        let (store, _dir) = store();
        let change = create("kfc", "sub-1", "dashboard", super::new_group_id());
        // A handler-owned transaction: the change lands only when the tx commits.
        let mut conn = store.sqlite_conn().unwrap();
        let tx = conn.transaction().unwrap();
        Store::record_in_sqlite_tx(&tx, &change).unwrap();
        tx.commit().unwrap();
        drop(conn);

        let rows = store
            .list_changes("kfc", &super::ChangeFilter::default())
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, change.id);
    }

    #[test]
    fn record_in_tx_rolls_back_with_the_caller() {
        let (store, _dir) = store();
        let change = create("kfc", "sub-1", "dashboard", super::new_group_id());
        let mut conn = store.sqlite_conn().unwrap();
        let tx = conn.transaction().unwrap();
        Store::record_in_sqlite_tx(&tx, &change).unwrap();
        // The caller's mutation failed: the whole tx rolls back, change included.
        drop(tx);
        drop(conn);
        assert!(store
            .list_changes("kfc", &super::ChangeFilter::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn list_changes_is_org_scoped() {
        let (store, _dir) = store();
        store
            .record_change(&create("kfc", "sub-1", "dashboard", super::new_group_id()))
            .unwrap();
        store
            .record_change(&create("acme", "sub-2", "dashboard", super::new_group_id()))
            .unwrap();
        assert_eq!(store.list_changes("kfc", &super::ChangeFilter::default()).unwrap().len(), 1);
        assert_eq!(store.list_changes("acme", &super::ChangeFilter::default()).unwrap().len(), 1);
        // A cross-org read sees nothing — org isolation is by construction.
        assert!(store
            .list_changes("ghost", &super::ChangeFilter::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn cursor_cas_guards_against_double_pop() {
        let (store, _dir) = store();
        let g1 = super::new_group_id();
        // First CAS from epoch 0 wins and bumps the epoch.
        assert!(store.cas_undo_cursor("kfc", "sub-1", 0, &[g1]).unwrap());
        let cursor = store.undo_cursor("kfc", "sub-1").unwrap();
        assert_eq!(cursor.redo_stack, vec![g1]);
        assert_eq!(cursor.epoch, 1);
        // A stale CAS (still expecting epoch 0) is refused — no double-pop.
        assert!(!store.cas_undo_cursor("kfc", "sub-1", 0, &[]).unwrap());
        // The correct epoch succeeds.
        assert!(store.cas_undo_cursor("kfc", "sub-1", 1, &[]).unwrap());
    }

    #[test]
    fn newest_undoable_group_skips_already_undone_and_is_per_actor() {
        let (store, _dir) = store();
        let g1 = super::new_group_id();
        let g2 = super::new_group_id();
        store.record_change(&create("kfc", "sub-1", "dashboard", g1)).unwrap();
        store.record_change(&create("kfc", "sub-1", "dashboard", g2)).unwrap();
        // Another actor's group must never surface for sub-1.
        store
            .record_change(&create("kfc", "other", "dashboard", super::new_group_id()))
            .unwrap();

        // Newest undoable is g2.
        assert_eq!(store.newest_undoable_group("kfc", "sub-1").unwrap(), Some(g2));
        // Mark g2 undone; now g1 is next.
        assert!(store.cas_undo_cursor("kfc", "sub-1", 0, &[g2]).unwrap());
        assert_eq!(store.newest_undoable_group("kfc", "sub-1").unwrap(), Some(g1));
        // Mark both undone; nothing left.
        assert!(store.cas_undo_cursor("kfc", "sub-1", 1, &[g2, g1]).unwrap());
        assert_eq!(store.newest_undoable_group("kfc", "sub-1").unwrap(), None);
    }
}
