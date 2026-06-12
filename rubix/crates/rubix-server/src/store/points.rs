//! Point rows: create, list, get, delete, keyexpr lookup.

use rubix_core::Point;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use super::codec::{filter_tags, json_of, ts_of};
use super::point_row::{kind_str, row_point, POINT_COLS};
use super::{Result, Store, StoreError};

impl Store {
    pub fn create_point(&self, point: &Point) -> Result<()> {
        let conn = self.conn()?;
        Self::require_equip(&conn, point.equip_id)?;
        conn.execute(
            &format!(
                "INSERT INTO points ({POINT_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"
            ),
            params![
                point.id,
                point.equip_id,
                point.slug,
                point.display_name,
                kind_str(point.kind),
                point.unit,
                json_of(&point.tags),
                json_of(&point.priority_array),
                point.cur_value.as_ref().map(json_of),
                point.cur_ts.as_ref().map(ts_of),
                ts_of(&point.created_at)
            ],
        )?;
        Ok(())
    }

    pub fn list_points(
        &self,
        equip_id: Option<Uuid>,
        site_id: Option<Uuid>,
        tags: &[String],
    ) -> Result<Vec<Point>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {POINT_COLS} FROM points \
             WHERE (?1 IS NULL OR equip_id = ?1) \
               AND (?2 IS NULL OR equip_id IN (SELECT id FROM equips WHERE site_id = ?2)) \
             ORDER BY slug"
        ))?;
        let rows = stmt.query_map(params![equip_id, site_id], row_point)?;
        let points: Vec<Point> = rows.collect::<rusqlite::Result<_>>()?;
        Ok(filter_tags(points, tags, |p| &p.tags))
    }

    pub fn get_point(&self, id: Uuid) -> Result<Point> {
        self.conn()?
            .query_row(
                &format!("SELECT {POINT_COLS} FROM points WHERE id = ?1"),
                params![id],
                row_point,
            )
            .optional()?
            .ok_or(StoreError::NotFound("point"))
    }

    pub fn delete_point(&self, id: Uuid) -> Result<()> {
        let n = self
            .conn()?
            .execute("DELETE FROM points WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("point"));
        }
        Ok(())
    }

    /// `{org}/{site}/{equip-path}/{point}` identity for a point.
    pub fn point_keyexpr(&self, id: Uuid) -> Result<String> {
        self.conn()?
            .query_row(
                "SELECT s.org, s.slug, e.path, p.slug FROM points p \
                 JOIN equips e ON e.id = p.equip_id JOIN sites s ON s.id = e.site_id \
                 WHERE p.id = ?1",
                params![id],
                |row| {
                    Ok(Point::keyexpr(
                        &row.get::<_, String>(0)?,
                        &row.get::<_, String>(1)?,
                        &row.get::<_, String>(2)?,
                        &row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or(StoreError::NotFound("point"))
    }

    pub(crate) fn require_point(conn: &rusqlite::Connection, id: Uuid) -> Result<()> {
        let exists: Option<i64> = conn
            .query_row("SELECT 1 FROM points WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()?;
        exists.map(|_| ()).ok_or(StoreError::NotFound("point"))
    }
}
