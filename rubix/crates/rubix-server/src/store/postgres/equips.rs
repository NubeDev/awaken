//! Equip rows, Postgres backend. Mirrors [`super::super::equips`].

use rubix_core::Equip;
use uuid::Uuid;

use super::super::codec::{json_of, ts_of};
use super::super::equips::EQUIP_COLS;
use super::super::{Result, Store, StoreError};
use super::codec::{filter_tags, json_col, require, ts_col, uuid_of};

fn equip_of(row: &postgres::Row) -> Result<Equip> {
    Ok(Equip {
        id: uuid_of(row, 0)?,
        site_id: uuid_of(row, 1)?,
        path: row.get(2),
        display_name: row.get(3),
        tags: json_col(row, 4)?,
        created_at: ts_col(row, 5)?,
    })
}

pub(crate) fn create_equip(store: &Store, equip: &Equip) -> Result<()> {
    let mut client = store.postgres_conn()?;
    require(&mut *client, "sites", "site", equip.site_id)?;
    client.execute(
        "INSERT INTO equips (id, site_id, path, display_name, tags, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &equip.id.to_string(),
            &equip.site_id.to_string(),
            &equip.path,
            &equip.display_name,
            &json_of(&equip.tags),
            &ts_of(&equip.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_equips(
    store: &Store,
    site_id: Option<Uuid>,
    tags: &[String],
) -> Result<Vec<Equip>> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let sql = format!(
        "SELECT {EQUIP_COLS} FROM equips WHERE ($1::text IS NULL OR site_id = $1) ORDER BY path"
    );
    let rows = client.query(sql.as_str(), &[&site])?;
    let equips = rows.iter().map(equip_of).collect::<Result<Vec<_>>>()?;
    Ok(filter_tags(equips, tags, |e| &e.tags))
}

pub(crate) fn get_equip(store: &Store, id: Uuid) -> Result<Equip> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {EQUIP_COLS} FROM equips WHERE id = $1");
    let row = client
        .query_opt(sql.as_str(), &[&id.to_string()])?
        .ok_or(StoreError::NotFound("equip"))?;
    equip_of(&row)
}

pub(crate) fn update_equip(
    store: &Store,
    id: Uuid,
    display_name: Option<&str>,
    tags: Option<&rubix_core::TagSet>,
) -> Result<Equip> {
    let mut client = store.postgres_conn()?;
    let tags_json = tags.map(json_of);
    let row = client
        .query_opt(
            &format!(
                "UPDATE equips SET \
                 display_name = COALESCE($2, display_name), \
                 tags = COALESCE($3, tags) \
                 WHERE id = $1 RETURNING {EQUIP_COLS}"
            ),
            &[&id.to_string(), &display_name, &tags_json],
        )?
        .ok_or(StoreError::NotFound("equip"))?;
    equip_of(&row)
}

pub(crate) fn delete_equip(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM equips WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("equip"));
    }
    Ok(())
}
