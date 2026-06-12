//! Keyexpr ↔ point resolution for the zenoh data plane. Backend dispatch;
//! SQLite body inline, Postgres body in [`super::postgres::keyexpr`].

use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use super::backend::Backend;
use super::{Result, Store, StoreError};

/// A point's identity plus its zenoh keyexpr prefix.
#[derive(Debug, Clone)]
pub struct PointKey {
    pub id: Uuid,
    /// `{org}/{site}/{equip-path}/{point}`.
    pub keyexpr: String,
}

/// Split a `{org}/{site}/{equip-path}/{point}` prefix into its parts. The
/// equip-path may contain slashes (nested equips), so org and site are the
/// first two segments and the point slug is the last. Shared by both backends.
pub(crate) fn split_point_prefix(prefix: &str) -> Option<(String, String, String, String)> {
    let parts: Vec<&str> = prefix.split('/').collect();
    if parts.len() < 4 {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2..parts.len() - 1].join("/"),
        parts[parts.len() - 1].to_string(),
    ))
}

impl Store {
    /// Resolve a `{org}/{site}/{equip-path}/{point}` prefix to a point id.
    pub fn point_by_keyexpr(&self, prefix: &str) -> Result<Uuid> {
        match &self.backend {
            Backend::Sqlite(_) => self.point_by_keyexpr_sqlite(prefix),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::keyexpr::point_by_keyexpr(self, prefix),
        }
    }

    fn point_by_keyexpr_sqlite(&self, prefix: &str) -> Result<Uuid> {
        let (org, site, equip_path, point) =
            split_point_prefix(prefix).ok_or(StoreError::NotFound("point"))?;
        self.sqlite_conn()?
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
        match &self.backend {
            Backend::Sqlite(_) => self.site_id_by_prefix_sqlite(prefix),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::keyexpr::site_id_by_prefix(self, prefix),
        }
    }

    fn site_id_by_prefix_sqlite(&self, prefix: &str) -> Result<Uuid> {
        let parts: Vec<&str> = prefix.split('/').collect();
        if parts.len() != 2 {
            return Err(StoreError::NotFound("site"));
        }
        self.sqlite_conn()?
            .query_row(
                "SELECT id FROM sites WHERE org = ?1 AND slug = ?2",
                params![parts[0], parts[1]],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound("site"))
    }

    /// Distinct `{org}/{site}` prefixes this node owns — one per site in the
    /// store. The bus scopes its `write`/`his` queryables to these.
    pub fn owned_site_prefixes(&self) -> Result<Vec<String>> {
        match &self.backend {
            Backend::Sqlite(_) => self.owned_site_prefixes_sqlite(),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::keyexpr::owned_site_prefixes(self),
        }
    }

    fn owned_site_prefixes_sqlite(&self) -> Result<Vec<String>> {
        let conn = self.sqlite_conn()?;
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
        match &self.backend {
            Backend::Sqlite(_) => self.all_point_keys_sqlite(),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::keyexpr::all_point_keys(self),
        }
    }

    fn all_point_keys_sqlite(&self) -> Result<Vec<PointKey>> {
        let conn = self.sqlite_conn()?;
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
