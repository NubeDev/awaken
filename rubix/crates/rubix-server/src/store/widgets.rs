//! Pinned dashboard widget rows: create and list. Backend dispatch; SQLite body
//! inline, Postgres body in [`super::postgres::widgets`].

use rubix_core::{Widget, WidgetKind};
use rusqlite::params;
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_to, ts_of, ts_to};
use super::{Result, Store, StoreError};

fn row_widget(row: &rusqlite::Row<'_>) -> rusqlite::Result<Widget> {
    Ok(Widget {
        id: row.get(0)?,
        dashboard_id: row.get(1)?,
        site_id: row.get(2)?,
        kind: json_to::<WidgetKind>(&format!("\"{}\"", row.get::<_, String>(3)?))?,
        title: row.get(4)?,
        target: row.get(5)?,
        query: row.get(6)?,
        created_at: ts_to(&row.get::<_, String>(7)?)?,
    })
}

pub(crate) const WIDGET_COLS: &str =
    "id, dashboard_id, site_id, kind, title, target, query, created_at";

impl Store {
    pub fn create_widget(&self, widget: &Widget) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_widget_sqlite(widget),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::widgets::create_widget(self, widget),
        }
    }

    fn create_widget_sqlite(&self, widget: &Widget) -> Result<()> {
        let conn = self.sqlite_conn()?;
        Self::require_dashboard(&conn, widget.dashboard_id)?;
        Self::require_site(&conn, widget.site_id)?;
        conn.execute(
            "INSERT INTO widgets \
                 (id, dashboard_id, site_id, kind, title, target, query, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                widget.id,
                widget.dashboard_id,
                widget.site_id,
                kind_token(widget.kind),
                widget.title,
                widget.target,
                widget.query,
                ts_of(&widget.created_at),
            ],
        )?;
        Ok(())
    }

    /// List widgets, optionally filtered by site and/or dashboard. With both
    /// `None` it returns every widget (the agent/overview view).
    pub fn list_widgets(
        &self,
        site_id: Option<Uuid>,
        dashboard_id: Option<Uuid>,
    ) -> Result<Vec<Widget>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_widgets_sqlite(site_id, dashboard_id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::widgets::list_widgets(self, site_id, dashboard_id)
            }
        }
    }

    fn list_widgets_sqlite(
        &self,
        site_id: Option<Uuid>,
        dashboard_id: Option<Uuid>,
    ) -> Result<Vec<Widget>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {WIDGET_COLS} FROM widgets \
             WHERE (?1 IS NULL OR site_id = ?1) \
               AND (?2 IS NULL OR dashboard_id = ?2) ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map(params![site_id, dashboard_id], row_widget)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_widget(&self, id: Uuid) -> Result<Widget> {
        match &self.backend {
            Backend::Sqlite(_) => self.get_widget_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::widgets::get_widget(self, id),
        }
    }

    fn get_widget_sqlite(&self, id: Uuid) -> Result<Widget> {
        use rusqlite::OptionalExtension;
        self.sqlite_conn()?
            .query_row(
                &format!("SELECT {WIDGET_COLS} FROM widgets WHERE id = ?1"),
                params![id],
                row_widget,
            )
            .optional()?
            .ok_or(StoreError::NotFound("widget"))
    }

    pub fn delete_widget(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.delete_widget_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::widgets::delete_widget(self, id),
        }
    }

    fn delete_widget_sqlite(&self, id: Uuid) -> Result<()> {
        let n = self
            .sqlite_conn()?
            .execute("DELETE FROM widgets WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("widget"));
        }
        Ok(())
    }
}

/// Bare snake_case token for the `kind` column (the serde repr without quotes).
pub(crate) fn kind_token(kind: WidgetKind) -> String {
    serde_json::to_string(&kind)
        .expect("WidgetKind serializes")
        .trim_matches('"')
        .to_string()
}
