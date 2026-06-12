//! Widget rows, Postgres backend. Mirrors [`super::super::widgets`].

use rubix_core::Widget;
use uuid::Uuid;

use super::super::codec::ts_of;
use super::super::widgets::kind_token;
use super::super::{Result, Store};
use super::codec::{require, token_enum, ts_col, uuid_of};

fn widget_of(row: &postgres::Row) -> Result<Widget> {
    Ok(Widget {
        id: uuid_of(row, 0)?,
        site_id: uuid_of(row, 1)?,
        kind: token_enum(row, 2)?,
        title: row.get(3),
        target: row.get(4),
        created_at: ts_col(row, 5)?,
    })
}

pub(crate) fn create_widget(store: &Store, widget: &Widget) -> Result<()> {
    let mut client = store.postgres_conn()?;
    require(&mut *client, "sites", "site", widget.site_id)?;
    client.execute(
        "INSERT INTO widgets (id, site_id, kind, title, target, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &widget.id.to_string(),
            &widget.site_id.to_string(),
            &kind_token(widget.kind),
            &widget.title,
            &widget.target,
            &ts_of(&widget.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_widgets(store: &Store, site_id: Option<Uuid>) -> Result<Vec<Widget>> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let rows = client.query(
        "SELECT id, site_id, kind, title, target, created_at FROM widgets \
         WHERE ($1::text IS NULL OR site_id = $1) ORDER BY created_at DESC",
        &[&site],
    )?;
    rows.iter().map(widget_of).collect()
}
