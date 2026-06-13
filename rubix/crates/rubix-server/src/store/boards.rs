//! Board rows: create a new version, list latest-per-slug, get by slug, delete.
//!
//! Boards are versioned: `create_board` always inserts a fresh `(slug,
//! version)`. `latest_boards` / `get_board` return the highest version for a
//! slug — the active definition the scheduler runs. Backend dispatch; SQLite
//! body inline, Postgres body in [`super::postgres::boards`].

use rusqlite::{params, OptionalExtension, Row};

use super::backend::Backend;
use super::codec::{json_of, json_to, ts_of, ts_to};
use crate::scheduler::BoardRecord;

use super::{Result, Store, StoreError};

pub(crate) const BOARD_COLS: &str =
    "id, slug, version, display_name, enabled, trigger, graph, created_at";

fn row_board(row: &Row<'_>) -> rusqlite::Result<BoardRecord> {
    Ok(BoardRecord {
        id: row.get(0)?,
        slug: row.get(1)?,
        version: row.get(2)?,
        display_name: row.get(3)?,
        enabled: row.get(4)?,
        trigger: json_to(&row.get::<_, String>(5)?)?,
        graph: json_to(&row.get::<_, String>(6)?)?,
        created_at: ts_to(&row.get::<_, String>(7)?)?,
    })
}

impl Store {
    /// Insert a board version. The caller assigns `version`; the `(slug,
    /// version)` UNIQUE constraint rejects a duplicate.
    pub fn create_board(&self, board: &BoardRecord) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_board_sqlite(board),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::create_board(self, board),
        }
    }

    fn create_board_sqlite(&self, board: &BoardRecord) -> Result<()> {
        self.sqlite_conn()?.execute(
            "INSERT INTO boards (id, slug, version, display_name, enabled, trigger, graph, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                board.id,
                board.slug,
                board.version,
                board.display_name,
                board.enabled,
                json_of(&board.trigger),
                json_of(&board.graph),
                ts_of(&board.created_at)
            ],
        )?;
        Ok(())
    }

    /// The next version number to assign for `slug` (max existing + 1, or 1).
    pub fn next_board_version(&self, slug: &str) -> Result<i64> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let max: Option<i64> = self.sqlite_conn()?.query_row(
                    "SELECT MAX(version) FROM boards WHERE slug = ?1",
                    params![slug],
                    |r| r.get(0),
                )?;
                Ok(max.unwrap_or(0) + 1)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::next_board_version(self, slug),
        }
    }

    /// Latest version of every distinct board slug, newest slug first.
    pub fn latest_boards(&self) -> Result<Vec<BoardRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.latest_boards_sqlite(),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::latest_boards(self),
        }
    }

    fn latest_boards_sqlite(&self) -> Result<Vec<BoardRecord>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {BOARD_COLS} FROM boards b WHERE version = \
             (SELECT MAX(version) FROM boards WHERE slug = b.slug) ORDER BY slug"
        ))?;
        let rows = stmt.query_map([], row_board)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Latest version of one slug.
    pub fn get_board(&self, slug: &str) -> Result<BoardRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self.get_board_sqlite(slug),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::get_board(self, slug),
        }
    }

    fn get_board_sqlite(&self, slug: &str) -> Result<BoardRecord> {
        self.sqlite_conn()?
            .query_row(
                &format!(
                    "SELECT {BOARD_COLS} FROM boards WHERE slug = ?1 ORDER BY version DESC LIMIT 1"
                ),
                params![slug],
                row_board,
            )
            .optional()?
            .ok_or(StoreError::NotFound("board"))
    }

    /// Delete every version of a slug. Returns NotFound if none existed.
    pub fn delete_board(&self, slug: &str) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self
                    .sqlite_conn()?
                    .execute("DELETE FROM boards WHERE slug = ?1", params![slug])?;
                if n == 0 {
                    return Err(StoreError::NotFound("board"));
                }
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::delete_board(self, slug),
        }
    }

    /// Patch mutable metadata (`display_name`, `enabled`) on the latest version
    /// of a board slug. `slug`/`trigger`/`graph` define the version and are not
    /// edited in place — republishing those is a new `create_board` version.
    pub fn update_board(
        &self,
        slug: &str,
        display_name: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<BoardRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self.update_board_sqlite(slug, display_name, enabled),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::boards::update_board(self, slug, display_name, enabled)
            }
        }
    }

    fn update_board_sqlite(
        &self,
        slug: &str,
        display_name: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<BoardRecord> {
        let conn = self.sqlite_conn()?;
        let n = conn.execute(
            "UPDATE boards SET \
             display_name = COALESCE(?2, display_name), \
             enabled = COALESCE(?3, enabled) \
             WHERE slug = ?1 AND version = (SELECT MAX(version) FROM boards WHERE slug = ?1)",
            params![slug, display_name, enabled],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound("board"));
        }
        self.get_board_sqlite(slug)
    }
}
