//! Point rows, Postgres backend. Mirrors [`super::super::points`].

use rubix_core::Point;
use uuid::Uuid;

use super::super::codec::{json_of, ts_of};
use super::super::point_row::{kind_str, POINT_COLS};
use super::super::{Result, Store, StoreError};
use super::codec::{filter_tags, point_of, require};

pub(crate) fn create_point(store: &Store, point: &Point) -> Result<()> {
    let mut client = store.postgres_conn()?;
    require(&mut *client, "equips", "equip", point.equip_id)?;
    let cur_value = point.cur_value.as_ref().map(json_of);
    let cur_ts = point.cur_ts.as_ref().map(ts_of);
    client.execute(
        &format!(
            "INSERT INTO points ({POINT_COLS}) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)"
        ),
        &[
            &point.id.to_string(),
            &point.equip_id.to_string(),
            &point.slug,
            &point.display_name,
            &kind_str(point.kind).to_string(),
            &point.unit,
            &json_of(&point.tags),
            &json_of(&point.priority_array),
            &cur_value,
            &cur_ts,
            &ts_of(&point.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_points(
    store: &Store,
    equip_id: Option<Uuid>,
    site_id: Option<Uuid>,
    tags: &[String],
) -> Result<Vec<Point>> {
    let mut client = store.postgres_conn()?;
    let equip = equip_id.map(|e| e.to_string());
    let site = site_id.map(|s| s.to_string());
    let sql = format!(
        "SELECT {POINT_COLS} FROM points \
         WHERE ($1::text IS NULL OR equip_id = $1) \
           AND ($2::text IS NULL OR equip_id IN (SELECT id FROM equips WHERE site_id = $2)) \
         ORDER BY slug"
    );
    let rows = client.query(sql.as_str(), &[&equip, &site])?;
    let points = rows.iter().map(point_of).collect::<Result<Vec<_>>>()?;
    Ok(filter_tags(points, tags, |p| &p.tags))
}

pub(crate) fn get_point(store: &Store, id: Uuid) -> Result<Point> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {POINT_COLS} FROM points WHERE id = $1");
    let row = client
        .query_opt(sql.as_str(), &[&id.to_string()])?
        .ok_or(StoreError::NotFound("point"))?;
    point_of(&row)
}

pub(crate) fn delete_point(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM points WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("point"));
    }
    Ok(())
}

pub(crate) fn point_keyexpr(store: &Store, id: Uuid) -> Result<String> {
    let mut client = store.postgres_conn()?;
    let row = client
        .query_opt(
            "SELECT s.org, s.slug, e.path, p.slug FROM points p \
             JOIN equips e ON e.id = p.equip_id JOIN sites s ON s.id = e.site_id \
             WHERE p.id = $1",
            &[&id.to_string()],
        )?
        .ok_or(StoreError::NotFound("point"))?;
    Ok(Point::keyexpr(
        &row.get::<_, String>(0),
        &row.get::<_, String>(1),
        &row.get::<_, String>(2),
        &row.get::<_, String>(3),
    ))
}
