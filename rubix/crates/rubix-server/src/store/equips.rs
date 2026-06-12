//! Equip rows: create, list, get, delete.

use rubix_core::Equip;
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use super::codec::{filter_tags, json_of, json_to, ts_of, ts_to};
use super::{Result, Store, StoreError};

fn row_equip(row: &Row<'_>) -> rusqlite::Result<Equip> {
    Ok(Equip {
        id: row.get(0)?,
        site_id: row.get(1)?,
        path: row.get(2)?,
        display_name: row.get(3)?,
        tags: json_to(&row.get::<_, String>(4)?)?,
        created_at: ts_to(&row.get::<_, String>(5)?)?,
    })
}

const EQUIP_COLS: &str = "id, site_id, path, display_name, tags, created_at";

impl Store {
    pub fn create_equip(&self, equip: &Equip) -> Result<()> {
        let conn = self.conn()?;
        Self::require_site(&conn, equip.site_id)?;
        conn.execute(
            "INSERT INTO equips (id, site_id, path, display_name, tags, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                equip.id,
                equip.site_id,
                equip.path,
                equip.display_name,
                json_of(&equip.tags),
                ts_of(&equip.created_at)
            ],
        )?;
        Ok(())
    }

    pub fn list_equips(&self, site_id: Option<Uuid>, tags: &[String]) -> Result<Vec<Equip>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {EQUIP_COLS} FROM equips WHERE (?1 IS NULL OR site_id = ?1) ORDER BY path"
        ))?;
        let rows = stmt.query_map(params![site_id], row_equip)?;
        let equips: Vec<Equip> = rows.collect::<rusqlite::Result<_>>()?;
        Ok(filter_tags(equips, tags, |e| &e.tags))
    }

    pub fn get_equip(&self, id: Uuid) -> Result<Equip> {
        self.conn()?
            .query_row(
                &format!("SELECT {EQUIP_COLS} FROM equips WHERE id = ?1"),
                params![id],
                row_equip,
            )
            .optional()?
            .ok_or(StoreError::NotFound("equip"))
    }

    pub fn delete_equip(&self, id: Uuid) -> Result<()> {
        let n = self
            .conn()?
            .execute("DELETE FROM equips WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("equip"));
        }
        Ok(())
    }

    pub(crate) fn require_equip(conn: &rusqlite::Connection, id: Uuid) -> Result<()> {
        let exists: Option<i64> = conn
            .query_row("SELECT 1 FROM equips WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()?;
        exists.map(|_| ()).ok_or(StoreError::NotFound("equip"))
    }
}
