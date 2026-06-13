//! Entity-tag rows (docs/design/page-context-and-nav.md §3): org-scoped key/value
//! tags on a domain entity, addressed by `(org, kind, entity_id)`. The full set
//! is replaced wholesale (the editor owns it). Every value reaches SQL as a bound
//! parameter — the injection boundary holds even for a `value` of `'); DROP …`.
//! Backend dispatch; SQLite body inline, Postgres in [`super::postgres::entity_tags`].

use rubix_core::EntityTags;
use rusqlite::params;
use uuid::Uuid;

use super::backend::Backend;
use super::{Result, Store};

/// One distinct tag key in use for a kind within an org — the `GET /tags/keys`
/// surface (authoring autocomplete).
impl Store {
    /// Replace the full tag set on `(org, kind, entity_id)`. Deletes the prior
    /// rows and inserts the new ones in one transaction, so a `PUT` is atomic and
    /// an empty set clears the entity. `value` is bound, never interpolated.
    pub fn replace_entity_tags(
        &self,
        org: &str,
        kind: &str,
        entity_id: Uuid,
        tags: &EntityTags,
    ) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.replace_entity_tags_sqlite(org, kind, entity_id, tags),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::entity_tags::replace_entity_tags(self, org, kind, entity_id, tags)
            }
        }
    }

    fn replace_entity_tags_sqlite(
        &self,
        org: &str,
        kind: &str,
        entity_id: Uuid,
        tags: &EntityTags,
    ) -> Result<()> {
        let mut conn = self.sqlite_conn()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM entity_tags WHERE org = ?1 AND kind = ?2 AND entity_id = ?3",
            params![org, kind, entity_id],
        )?;
        for (key, value) in &tags.0 {
            tx.execute(
                "INSERT INTO entity_tags (org, kind, entity_id, key, value) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![org, kind, entity_id, key, value],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The full tag set on `(org, kind, entity_id)`; empty when the entity carries
    /// no tags.
    pub fn entity_tags(&self, org: &str, kind: &str, entity_id: Uuid) -> Result<EntityTags> {
        match &self.backend {
            Backend::Sqlite(_) => self.entity_tags_sqlite(org, kind, entity_id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::entity_tags::entity_tags(self, org, kind, entity_id)
            }
        }
    }

    fn entity_tags_sqlite(&self, org: &str, kind: &str, entity_id: Uuid) -> Result<EntityTags> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key, value FROM entity_tags \
             WHERE org = ?1 AND kind = ?2 AND entity_id = ?3 ORDER BY key",
        )?;
        let rows = stmt.query_map(params![org, kind, entity_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?;
        Ok(EntityTags(rows.collect::<rusqlite::Result<_>>()?))
    }

    /// Reverse lookup: every entity of `kind` in `org` that carries a tag, with
    /// its full set. Drives `GET /tags/entities/{kind}` (which boards hold a given
    /// tag). Keyed by entity id string.
    pub fn entities_with_tags(
        &self,
        org: &str,
        kind: &str,
    ) -> Result<Vec<(Uuid, EntityTags)>> {
        match &self.backend {
            Backend::Sqlite(_) => self.entities_with_tags_sqlite(org, kind),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::entity_tags::entities_with_tags(self, org, kind)
            }
        }
    }

    fn entities_with_tags_sqlite(
        &self,
        org: &str,
        kind: &str,
    ) -> Result<Vec<(Uuid, EntityTags)>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(
            "SELECT entity_id, key, value FROM entity_tags \
             WHERE org = ?1 AND kind = ?2 ORDER BY entity_id, key",
        )?;
        let rows = stmt.query_map(params![org, kind], |r| {
            Ok((
                r.get::<_, Uuid>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        let mut out: Vec<(Uuid, EntityTags)> = Vec::new();
        for row in rows {
            let (id, key, value) = row?;
            match out.last_mut() {
                Some((last, tags)) if *last == id => {
                    tags.0.insert(key, value);
                }
                _ => {
                    let mut tags = EntityTags::default();
                    tags.0.insert(key, value);
                    out.push((id, tags));
                }
            }
        }
        Ok(out)
    }

    /// Distinct tag keys in use for `kind` within `org` (authoring autocomplete).
    pub fn entity_tag_keys(&self, org: &str, kind: &str) -> Result<Vec<String>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT key FROM entity_tags \
                     WHERE org = ?1 AND kind = ?2 ORDER BY key",
                )?;
                let rows = stmt.query_map(params![org, kind], |r| r.get::<_, String>(0))?;
                Ok(rows.collect::<rusqlite::Result<_>>()?)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::entity_tags::entity_tag_keys(self, org, kind),
        }
    }

    /// Sweep every tag on `(kind, entity_id)` when the entity is deleted. Called
    /// from the entity's own delete handler (the same place cascades happen);
    /// `org` is not needed since the entity id is globally unique.
    pub fn sweep_entity_tags(&self, kind: &str, entity_id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                self.sqlite_conn()?.execute(
                    "DELETE FROM entity_tags WHERE kind = ?1 AND entity_id = ?2",
                    params![kind, entity_id],
                )?;
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::entity_tags::sweep_entity_tags(self, kind, entity_id)
            }
        }
    }
}
