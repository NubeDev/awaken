//! Pinned dashboard widget rows: create and list. Backend dispatch; SQLite body
//! inline, Postgres body in [`super::postgres::widgets`].

use rubix_core::{Widget, WidgetKind};
use rusqlite::params;
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_to, ts_of, ts_to};
use super::{Result, Store};

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
        Self::require_site(&conn, widget.site_id)?;
        conn.execute(
            "INSERT INTO widgets (id, site_id, kind, title, target, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                widget.id,
                widget.site_id,
                kind_token(widget.kind),
                widget.title,
                widget.target,
                ts_of(&widget.created_at),
            ],
        )?;
        Ok(())
    }

    pub fn list_widgets(&self, site_id: Option<Uuid>) -> Result<Vec<Widget>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_widgets_sqlite(site_id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::widgets::list_widgets(self, site_id),
        }
    }

    fn list_widgets_sqlite(&self, site_id: Option<Uuid>) -> Result<Vec<Widget>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, site_id, kind, title, target, created_at FROM widgets \
             WHERE (?1 IS NULL OR site_id = ?1) ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![site_id], |row| {
            Ok(Widget {
                id: row.get(0)?,
                site_id: row.get(1)?,
                kind: json_to::<WidgetKind>(&format!("\"{}\"", row.get::<_, String>(2)?))?,
                title: row.get(3)?,
                target: row.get(4)?,
                created_at: ts_to(&row.get::<_, String>(5)?)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// Bare snake_case token for the `kind` column (the serde repr without quotes).
pub(crate) fn kind_token(kind: WidgetKind) -> String {
    serde_json::to_string(&kind)
        .expect("WidgetKind serializes")
        .trim_matches('"')
        .to_string()
}
