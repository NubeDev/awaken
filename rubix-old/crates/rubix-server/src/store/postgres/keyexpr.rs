//! Keyexpr resolution, Postgres backend. Mirrors [`super::super::keyexpr`].

use uuid::Uuid;

use super::super::keyexpr::{split_point_prefix, PointKey};
use super::super::{Result, Store, StoreError};
use super::codec::uuid_of;

pub(crate) fn point_by_keyexpr(store: &Store, prefix: &str) -> Result<Uuid> {
    let (org, site, equip_path, point) =
        split_point_prefix(prefix).ok_or(StoreError::NotFound("point"))?;
    let mut client = store.postgres_conn()?;
    let row = client
        .query_opt(
            "SELECT p.id FROM points p \
             JOIN equips e ON e.id = p.equip_id JOIN sites s ON s.id = e.site_id \
             WHERE s.org = $1 AND s.slug = $2 AND e.path = $3 AND p.slug = $4",
            &[&org, &site, &equip_path, &point],
        )?
        .ok_or(StoreError::NotFound("point"))?;
    uuid_of(&row, 0)
}

pub(crate) fn site_id_by_prefix(store: &Store, prefix: &str) -> Result<Uuid> {
    let parts: Vec<&str> = prefix.split('/').collect();
    if parts.len() != 2 {
        return Err(StoreError::NotFound("site"));
    }
    let mut client = store.postgres_conn()?;
    let row = client
        .query_opt(
            "SELECT id FROM sites WHERE org = $1 AND slug = $2",
            &[&parts[0], &parts[1]],
        )?
        .ok_or(StoreError::NotFound("site"))?;
    uuid_of(&row, 0)
}

pub(crate) fn owned_site_prefixes(store: &Store) -> Result<Vec<String>> {
    let mut client = store.postgres_conn()?;
    let rows = client.query("SELECT org, slug FROM sites ORDER BY org, slug", &[])?;
    Ok(rows
        .iter()
        .map(|row| format!("{}/{}", row.get::<_, String>(0), row.get::<_, String>(1)))
        .collect())
}

pub(crate) fn all_point_keys(store: &Store) -> Result<Vec<PointKey>> {
    let mut client = store.postgres_conn()?;
    let rows = client.query(
        "SELECT p.id, s.org, s.slug, e.path, p.slug FROM points p \
         JOIN equips e ON e.id = p.equip_id JOIN sites s ON s.id = e.site_id",
        &[],
    )?;
    rows.iter()
        .map(|row| {
            Ok(PointKey {
                id: uuid_of(row, 0)?,
                keyexpr: rubix_core::Point::keyexpr(
                    &row.get::<_, String>(1),
                    &row.get::<_, String>(2),
                    &row.get::<_, String>(3),
                    &row.get::<_, String>(4),
                ),
            })
        })
        .collect()
}
