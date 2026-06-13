//! Widget rows, Postgres backend. Mirrors [`super::super::widgets`].

use rubix_core::Widget;
use uuid::Uuid;

use super::super::codec::ts_of;
use super::super::widgets::kind_token;
use super::super::{Result, Store, StoreError};
use super::codec::{require, token_enum, ts_col, uuid_of};

const WIDGET_COLS: &str = "id, dashboard_id, site_id, kind, title, target, query, created_at";

fn widget_of(row: &postgres::Row) -> Result<Widget> {
    Ok(Widget {
        id: uuid_of(row, 0)?,
        dashboard_id: uuid_of(row, 1)?,
        site_id: uuid_of(row, 2)?,
        kind: token_enum(row, 3)?,
        title: row.get(4),
        target: row.get(5),
        query: row.get(6),
        created_at: ts_col(row, 7)?,
    })
}

pub(crate) fn create_widget(store: &Store, widget: &Widget) -> Result<()> {
    let mut client = store.postgres_conn()?;
    require(&mut *client, "dashboards", "dashboard", widget.dashboard_id)?;
    require(&mut *client, "sites", "site", widget.site_id)?;
    client.execute(
        "INSERT INTO widgets \
             (id, dashboard_id, site_id, kind, title, target, query, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &widget.id.to_string(),
            &widget.dashboard_id.to_string(),
            &widget.site_id.to_string(),
            &kind_token(widget.kind),
            &widget.title,
            &widget.target,
            &widget.query,
            &ts_of(&widget.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn get_widget(store: &Store, id: Uuid) -> Result<Widget> {
    let mut client = store.postgres_conn()?;
    let row = client
        .query_opt(
            &format!("SELECT {WIDGET_COLS} FROM widgets WHERE id = $1"),
            &[&id.to_string()],
        )?
        .ok_or(StoreError::NotFound("widget"))?;
    widget_of(&row)
}

pub(crate) fn delete_widget(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM widgets WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("widget"));
    }
    Ok(())
}

pub(crate) fn list_widgets(
    store: &Store,
    site_id: Option<Uuid>,
    dashboard_id: Option<Uuid>,
) -> Result<Vec<Widget>> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let dashboard = dashboard_id.map(|d| d.to_string());
    let rows = client.query(
        &format!(
            "SELECT {WIDGET_COLS} FROM widgets \
             WHERE ($1::text IS NULL OR site_id = $1) \
               AND ($2::text IS NULL OR dashboard_id = $2) ORDER BY created_at DESC"
        ),
        &[&site, &dashboard],
    )?;
    rows.iter().map(widget_of).collect()
}
