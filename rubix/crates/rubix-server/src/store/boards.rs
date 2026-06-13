//! Board rows: versioned flows scoped to an `org` and optionally a `site`.
//!
//! A flow is owned by an `org` and either site-scoped (`site_id` set) or
//! org-level (`site_id` NULL, applying across the org) — the same model as
//! [`rubix_core::Dashboard`]. Boards are versioned: `create_board` inserts a
//! fresh `(org, site_id, slug, version)`; `latest_board`/`get_board` return the
//! highest version for a slug within its scope — the active definition.
//!
//! Resolution: API CRUD addresses a board by `(org, site_id, slug)`; the
//! scheduler re-fetches by globally-unique `id` ([`Store::get_board_by_id`]),
//! so its hot loops never need scope context. Backend dispatch; SQLite body
//! inline, Postgres body in [`super::postgres::boards`].

use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_of, json_to, ts_of, ts_to};
use crate::scheduler::BoardRecord;

use super::{Result, Store, StoreError};

pub(crate) const BOARD_COLS: &str =
    "id, org, site_id, slug, version, display_name, enabled, trigger, graph, created_at";

fn row_board(row: &Row<'_>) -> rusqlite::Result<BoardRecord> {
    Ok(BoardRecord {
        id: row.get(0)?,
        org: row.get(1)?,
        site_id: row.get(2)?,
        slug: row.get(3)?,
        version: row.get(4)?,
        display_name: row.get(5)?,
        enabled: row.get(6)?,
        trigger: json_to(&row.get::<_, String>(7)?)?,
        graph: json_to(&row.get::<_, String>(8)?)?,
        created_at: ts_to(&row.get::<_, String>(9)?)?,
    })
}

impl Store {
    /// Insert a board version. The caller assigns `version`; the per-scope
    /// unique index rejects a duplicate `(org, site_id, slug, version)`.
    pub fn create_board(&self, board: &BoardRecord) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_board_sqlite(board),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::create_board(self, board),
        }
    }

    fn create_board_sqlite(&self, board: &BoardRecord) -> Result<()> {
        let conn = self.sqlite_conn()?;
        if let Some(site_id) = board.site_id {
            Self::require_site(&conn, site_id)?;
        }
        conn.execute(
            "INSERT INTO boards \
                 (id, org, site_id, slug, version, display_name, enabled, trigger, graph, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                board.id,
                board.org,
                board.site_id,
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

    /// The next version to assign for a slug within its scope (max + 1, or 1).
    pub fn next_board_version(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        slug: &str,
    ) -> Result<i64> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let max: Option<i64> = self.sqlite_conn()?.query_row(
                    "SELECT MAX(version) FROM boards \
                     WHERE org = ?1 AND site_id IS ?2 AND slug = ?3",
                    params![org, site_id, slug],
                    |r| r.get(0),
                )?;
                Ok(max.unwrap_or(0) + 1)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::boards::next_board_version(self, org, site_id, slug)
            }
        }
    }

    /// Latest version of every board across ALL tenants — the scheduler-boot
    /// view (it schedules every org's enabled boards).
    pub fn latest_boards_all(&self) -> Result<Vec<BoardRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.latest_boards_filtered_sqlite(None, None, false),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::latest_boards_all(self),
        }
    }

    /// Latest version of every board in a scope: `org` required, `site_id`
    /// filters to one site's flows when `Some`. With `site_id` `None` it returns
    /// the org's flows at every scope (org-level + all sites) — the API list.
    pub fn latest_boards(
        &self,
        org: &str,
        site_id: Option<Uuid>,
    ) -> Result<Vec<BoardRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                self.latest_boards_filtered_sqlite(Some(org), site_id, false)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::latest_boards(self, org, site_id),
        }
    }

    /// Shared latest-per-scope query. `org`/`site_id` filter when `Some`;
    /// `strict_site` makes a `Some(site)` filter exact (used by the API site
    /// filter) — here `site_id = ?` already is exact, so the flag is reserved
    /// for symmetry and currently unused at the SQL level.
    fn latest_boards_filtered_sqlite(
        &self,
        org: Option<&str>,
        site_id: Option<Uuid>,
        _strict_site: bool,
    ) -> Result<Vec<BoardRecord>> {
        let conn = self.sqlite_conn()?;
        // The latest version is per scope `(org, site_id, slug)`. Compare site
        // with IS so NULL groups correctly.
        let mut stmt = conn.prepare(&format!(
            "SELECT {BOARD_COLS} FROM boards b WHERE version = (\
                 SELECT MAX(version) FROM boards \
                 WHERE org = b.org AND site_id IS b.site_id AND slug = b.slug) \
             AND (?1 IS NULL OR b.org = ?1) \
             AND (?2 IS NULL OR b.site_id = ?2) \
             ORDER BY b.slug"
        ))?;
        let rows = stmt.query_map(params![org, site_id], row_board)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Latest version of one slug within its scope.
    pub fn get_board(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        slug: &str,
    ) -> Result<BoardRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self.get_board_sqlite(org, site_id, slug),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::get_board(self, org, site_id, slug),
        }
    }

    fn get_board_sqlite(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        slug: &str,
    ) -> Result<BoardRecord> {
        self.sqlite_conn()?
            .query_row(
                &format!(
                    "SELECT {BOARD_COLS} FROM boards \
                     WHERE org = ?1 AND site_id IS ?2 AND slug = ?3 \
                     ORDER BY version DESC LIMIT 1"
                ),
                params![org, site_id, slug],
                row_board,
            )
            .optional()?
            .ok_or(StoreError::NotFound("board"))
    }

    /// Fetch a board by its globally-unique row id (a single version). The
    /// scheduler's loops use this so they never need scope context to re-read a
    /// board when its trigger fires.
    pub fn get_board_by_id(&self, id: Uuid) -> Result<BoardRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self
                .sqlite_conn()?
                .query_row(
                    &format!("SELECT {BOARD_COLS} FROM boards WHERE id = ?1"),
                    params![id],
                    row_board,
                )
                .optional()?
                .ok_or(StoreError::NotFound("board")),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::get_board_by_id(self, id),
        }
    }

    /// Delete every version of a slug within its scope. NotFound if none existed.
    pub fn delete_board(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        slug: &str,
    ) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self.sqlite_conn()?.execute(
                    "DELETE FROM boards WHERE org = ?1 AND site_id IS ?2 AND slug = ?3",
                    params![org, site_id, slug],
                )?;
                if n == 0 {
                    return Err(StoreError::NotFound("board"));
                }
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::delete_board(self, org, site_id, slug),
        }
    }

    /// Patch mutable metadata (`display_name`, `enabled`) on the latest version
    /// of a board slug within its scope. `slug`/`trigger`/`graph` define the
    /// version and are not edited in place — republishing is a new version.
    pub fn update_board(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        slug: &str,
        display_name: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<BoardRecord> {
        match &self.backend {
            Backend::Sqlite(_) => {
                self.update_board_sqlite(org, site_id, slug, display_name, enabled)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::boards::update_board(
                self,
                org,
                site_id,
                slug,
                display_name,
                enabled,
            ),
        }
    }

    fn update_board_sqlite(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        slug: &str,
        display_name: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<BoardRecord> {
        let conn = self.sqlite_conn()?;
        let n = conn.execute(
            "UPDATE boards SET \
             display_name = COALESCE(?4, display_name), \
             enabled = COALESCE(?5, enabled) \
             WHERE org = ?1 AND site_id IS ?2 AND slug = ?3 AND version = (\
                 SELECT MAX(version) FROM boards \
                 WHERE org = ?1 AND site_id IS ?2 AND slug = ?3)",
            params![org, site_id, slug, display_name, enabled],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound("board"));
        }
        self.get_board_sqlite(org, site_id, slug)
    }
}
