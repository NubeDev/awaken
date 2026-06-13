//! Dashboard rows: create, list (by org, optionally by site), get, patch,
//! delete. A dashboard groups widgets; it is owned by an `org` and either
//! site-scoped or an org overview (`site_id` NULL). Backend dispatch lives here;
//! the SQLite body is inline, the Postgres body in [`super::postgres::dashboards`].

use rubix_core::Dashboard;
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{ts_of, ts_to};
use super::{Result, Store, StoreError};

fn row_dashboard(row: &Row<'_>) -> rusqlite::Result<Dashboard> {
    Ok(Dashboard {
        id: row.get(0)?,
        org: row.get(1)?,
        site_id: row.get(2)?,
        slug: row.get(3)?,
        title: row.get(4)?,
        created_at: ts_to(&row.get::<_, String>(5)?)?,
    })
}

pub(crate) const DASHBOARD_COLS: &str = "id, org, site_id, slug, title, created_at";

impl Store {
    pub fn create_dashboard(&self, dashboard: &Dashboard) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_dashboard_sqlite(dashboard),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::dashboards::create_dashboard(self, dashboard),
        }
    }

    fn create_dashboard_sqlite(&self, dashboard: &Dashboard) -> Result<()> {
        let conn = self.sqlite_conn()?;
        if let Some(site_id) = dashboard.site_id {
            Self::require_site(&conn, site_id)?;
        }
        conn.execute(
            "INSERT INTO dashboards (id, org, site_id, slug, title, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                dashboard.id,
                dashboard.org,
                dashboard.site_id,
                dashboard.slug,
                dashboard.title,
                ts_of(&dashboard.created_at),
            ],
        )?;
        Ok(())
    }

    /// Dashboards under an org. `site_id` filters to one site's boards when
    /// `Some`; `None` returns every board the org owns (overviews + all sites).
    pub fn list_dashboards(&self, org: &str, site_id: Option<Uuid>) -> Result<Vec<Dashboard>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_dashboards_sqlite(org, site_id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::dashboards::list_dashboards(self, org, site_id)
            }
        }
    }

    fn list_dashboards_sqlite(&self, org: &str, site_id: Option<Uuid>) -> Result<Vec<Dashboard>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {DASHBOARD_COLS} FROM dashboards \
             WHERE org = ?1 AND (?2 IS NULL OR site_id = ?2) \
             ORDER BY site_id IS NOT NULL, slug"
        ))?;
        let rows = stmt.query_map(params![org, site_id], row_dashboard)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_dashboard(&self, id: Uuid) -> Result<Dashboard> {
        match &self.backend {
            Backend::Sqlite(_) => self.get_dashboard_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::dashboards::get_dashboard(self, id),
        }
    }

    fn get_dashboard_sqlite(&self, id: Uuid) -> Result<Dashboard> {
        self.sqlite_conn()?
            .query_row(
                &format!("SELECT {DASHBOARD_COLS} FROM dashboards WHERE id = ?1"),
                params![id],
                row_dashboard,
            )
            .optional()?
            .ok_or(StoreError::NotFound("dashboard"))
    }

    /// Patch the mutable metadata of a dashboard (`title`). `org`/`site_id`/
    /// `slug` are identity and immutable. Returns the updated row.
    pub fn update_dashboard(&self, id: Uuid, title: Option<&str>) -> Result<Dashboard> {
        match &self.backend {
            Backend::Sqlite(_) => self.update_dashboard_sqlite(id, title),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::dashboards::update_dashboard(self, id, title),
        }
    }

    fn update_dashboard_sqlite(&self, id: Uuid, title: Option<&str>) -> Result<Dashboard> {
        let conn = self.sqlite_conn()?;
        let n = conn.execute(
            "UPDATE dashboards SET title = COALESCE(?2, title) WHERE id = ?1",
            params![id, title],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound("dashboard"));
        }
        conn.query_row(
            &format!("SELECT {DASHBOARD_COLS} FROM dashboards WHERE id = ?1"),
            params![id],
            row_dashboard,
        )
        .map_err(Into::into)
    }

    pub fn delete_dashboard(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.delete_dashboard_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::dashboards::delete_dashboard(self, id),
        }
    }

    fn delete_dashboard_sqlite(&self, id: Uuid) -> Result<()> {
        let n = self
            .sqlite_conn()?
            .execute("DELETE FROM dashboards WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("dashboard"));
        }
        Ok(())
    }

    /// Find or create a site's `default` dashboard and return its id. The agent
    /// `pin_widget` path and any "pin to this site" flow land here so a widget
    /// always has a home board without the caller choosing one.
    pub fn default_dashboard_for_site(&self, site_id: Uuid) -> Result<Uuid> {
        let site = self.get_site(site_id)?;
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let existing: Option<Uuid> = conn
                    .query_row(
                        "SELECT id FROM dashboards WHERE site_id = ?1 AND slug = 'default'",
                        params![site_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if let Some(id) = existing {
                    return Ok(id);
                }
                let dashboard = Dashboard {
                    id: Uuid::new_v4(),
                    org: site.org,
                    site_id: Some(site_id),
                    slug: "default".into(),
                    title: "Default".into(),
                    created_at: chrono::Utc::now(),
                };
                self.create_dashboard(&dashboard)?;
                Ok(dashboard.id)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::dashboards::default_dashboard_for_site(self, &site)
            }
        }
    }

    pub(crate) fn require_dashboard(conn: &rusqlite::Connection, id: Uuid) -> Result<()> {
        let exists: Option<i64> = conn
            .query_row("SELECT 1 FROM dashboards WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()?;
        exists.map(|_| ()).ok_or(StoreError::NotFound("dashboard"))
    }
}
