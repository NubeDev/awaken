//! Keyexpr ↔ point resolution for the zenoh data plane.

use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use super::{Result, Store, StoreError};

/// A point's identity plus its zenoh keyexpr prefix.
#[derive(Debug, Clone)]
pub struct PointKey {
    pub id: Uuid,
    /// `{org}/{site}/{equip-path}/{point}`.
    pub keyexpr: String,
}

impl Store {
    /// Resolve a `{org}/{site}/{equip-path}/{point}` prefix to a point id.
    /// The equip-path may contain slashes (nested equips), so the org and
    /// site are the first two segments and the point slug is the last.
    pub fn point_by_keyexpr(&self, prefix: &str) -> Result<Uuid> {
        let parts: Vec<&str> = prefix.split('/').collect();
        if parts.len() < 4 {
            return Err(StoreError::NotFound("point"));
        }
        let org = parts[0];
        let site = parts[1];
        let point = parts[parts.len() - 1];
        let equip_path = parts[2..parts.len() - 1].join("/");
        self.conn()?
            .query_row(
                "SELECT p.id FROM points p \
                 JOIN equips e ON e.id = p.equip_id JOIN sites s ON s.id = e.site_id \
                 WHERE s.org = ?1 AND s.slug = ?2 AND e.path = ?3 AND p.slug = ?4",
                params![org, site, equip_path, point],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound("point"))
    }

    /// Resolve an `{org}/{site}` prefix to a site id. Used by the `emit_spark`
    /// board node, which names its site the same way it names points.
    pub fn site_id_by_prefix(&self, prefix: &str) -> Result<Uuid> {
        let parts: Vec<&str> = prefix.split('/').collect();
        if parts.len() != 2 {
            return Err(StoreError::NotFound("site"));
        }
        self.conn()?
            .query_row(
                "SELECT id FROM sites WHERE org = ?1 AND slug = ?2",
                params![parts[0], parts[1]],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound("site"))
    }

    /// Distinct `{org}/{site}` prefixes this node owns — one per site in the
    /// store. The bus scopes its `write`/`his` queryables to these so a node in
    /// a multi-node mesh only answers for sites it actually holds, instead of
    /// declaring global `**/write` and replying "not found" for foreign keys.
    pub fn owned_site_prefixes(&self) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT org, slug FROM sites ORDER BY org, slug")?;
        let rows = stmt.query_map([], |row| {
            Ok(format!(
                "{}/{}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// All points with their keyexpr prefixes, for declaring the data plane.
    pub fn all_point_keys(&self) -> Result<Vec<PointKey>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, s.org, s.slug, e.path, p.slug FROM points p \
             JOIN equips e ON e.id = p.equip_id JOIN sites s ON s.id = e.site_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PointKey {
                id: row.get(0)?,
                keyexpr: rubix_core::Point::keyexpr(
                    &row.get::<_, String>(1)?,
                    &row.get::<_, String>(2)?,
                    &row.get::<_, String>(3)?,
                    &row.get::<_, String>(4)?,
                ),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
