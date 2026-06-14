//! Entity-tag rows, Postgres backend. Mirrors [`super::super::entity_tags`].

use rubix_core::EntityTags;
use uuid::Uuid;

use super::super::{Result, Store};

pub(crate) fn replace_entity_tags(
    store: &Store,
    org: &str,
    kind: &str,
    entity_id: Uuid,
    tags: &EntityTags,
) -> Result<()> {
    let mut client = store.postgres_conn()?;
    let mut tx = client.transaction()?;
    tx.execute(
        "DELETE FROM entity_tags WHERE org = $1 AND kind = $2 AND entity_id = $3",
        &[&org, &kind, &entity_id.to_string()],
    )?;
    for (key, value) in &tags.0 {
        tx.execute(
            "INSERT INTO entity_tags (org, kind, entity_id, key, value) \
             VALUES ($1, $2, $3, $4, $5)",
            &[&org, &kind, &entity_id.to_string(), key, value],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn entity_tags(
    store: &Store,
    org: &str,
    kind: &str,
    entity_id: Uuid,
) -> Result<EntityTags> {
    let mut client = store.postgres_conn()?;
    let rows = client.query(
        "SELECT key, value FROM entity_tags \
         WHERE org = $1 AND kind = $2 AND entity_id = $3 ORDER BY key",
        &[&org, &kind, &entity_id.to_string()],
    )?;
    Ok(EntityTags(
        rows.iter()
            .map(|r| (r.get::<_, String>(0), r.get::<_, Option<String>>(1)))
            .collect(),
    ))
}

pub(crate) fn entities_with_tags(
    store: &Store,
    org: &str,
    kind: &str,
) -> Result<Vec<(Uuid, EntityTags)>> {
    let mut client = store.postgres_conn()?;
    let rows = client.query(
        "SELECT entity_id, key, value FROM entity_tags \
         WHERE org = $1 AND kind = $2 ORDER BY entity_id, key",
        &[&org, &kind],
    )?;
    let mut out: Vec<(Uuid, EntityTags)> = Vec::new();
    for r in &rows {
        let raw: String = r.get(0);
        let id = Uuid::parse_str(&raw)
            .map_err(|e| super::super::StoreError::Db(anyhow::anyhow!("bad entity_id: {e}")))?;
        let key: String = r.get(1);
        let value: Option<String> = r.get(2);
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

pub(crate) fn entity_tag_keys(store: &Store, org: &str, kind: &str) -> Result<Vec<String>> {
    let mut client = store.postgres_conn()?;
    let rows = client.query(
        "SELECT DISTINCT key FROM entity_tags WHERE org = $1 AND kind = $2 ORDER BY key",
        &[&org, &kind],
    )?;
    Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
}

pub(crate) fn sweep_entity_tags(store: &Store, kind: &str, entity_id: Uuid) -> Result<()> {
    store.postgres_conn()?.execute(
        "DELETE FROM entity_tags WHERE kind = $1 AND entity_id = $2",
        &[&kind, &entity_id.to_string()],
    )?;
    Ok(())
}
