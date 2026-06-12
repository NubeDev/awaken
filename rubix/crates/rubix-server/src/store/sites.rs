//! Site rows: create, list, get, delete. Backend dispatch lives here; the
//! SQLite body is inline, the Postgres body in [`super::postgres::sites`].

use rubix_core::Site;
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_of, json_to, ts_of, ts_to};
use super::{Result, Store, StoreError};

fn row_site(row: &Row<'_>) -> rusqlite::Result<Site> {
    Ok(Site {
        id: row.get(0)?,
        org: row.get(1)?,
        slug: row.get(2)?,
        display_name: row.get(3)?,
        tags: json_to(&row.get::<_, String>(4)?)?,
        created_at: ts_to(&row.get::<_, String>(5)?)?,
    })
}

pub(crate) const SITE_COLS: &str = "id, org, slug, display_name, tags, created_at";

impl Store {
    pub fn create_site(&self, site: &Site) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_site_sqlite(site),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::sites::create_site(self, site),
        }
    }

    fn create_site_sqlite(&self, site: &Site) -> Result<()> {
        self.sqlite_conn()?.execute(
            "INSERT INTO sites (id, org, slug, display_name, tags, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                site.id,
                site.org,
                site.slug,
                site.display_name,
                json_of(&site.tags),
                ts_of(&site.created_at)
            ],
        )?;
        Ok(())
    }

    pub fn list_sites(&self, org: Option<&str>) -> Result<Vec<Site>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_sites_sqlite(org),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::sites::list_sites(self, org),
        }
    }

    fn list_sites_sqlite(&self, org: Option<&str>) -> Result<Vec<Site>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SITE_COLS} FROM sites WHERE (?1 IS NULL OR org = ?1) ORDER BY org, slug"
        ))?;
        let rows = stmt.query_map(params![org], row_site)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_site(&self, id: Uuid) -> Result<Site> {
        match &self.backend {
            Backend::Sqlite(_) => self.get_site_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::sites::get_site(self, id),
        }
    }

    fn get_site_sqlite(&self, id: Uuid) -> Result<Site> {
        self.sqlite_conn()?
            .query_row(
                &format!("SELECT {SITE_COLS} FROM sites WHERE id = ?1"),
                params![id],
                row_site,
            )
            .optional()?
            .ok_or(StoreError::NotFound("site"))
    }

    pub fn delete_site(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.delete_site_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::sites::delete_site(self, id),
        }
    }

    fn delete_site_sqlite(&self, id: Uuid) -> Result<()> {
        let n = self
            .sqlite_conn()?
            .execute("DELETE FROM sites WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("site"));
        }
        Ok(())
    }

    pub(crate) fn require_site(conn: &rusqlite::Connection, id: Uuid) -> Result<()> {
        let exists: Option<i64> = conn
            .query_row("SELECT 1 FROM sites WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()?;
        exists.map(|_| ()).ok_or(StoreError::NotFound("site"))
    }
}
