//! Site rows, Postgres backend. Mirrors [`super::super::sites`].

use rubix_core::Site;
use uuid::Uuid;

use super::super::codec::{json_of, ts_of};
use super::super::sites::SITE_COLS;
use super::super::{Result, Store, StoreError};
use super::codec::site_of;

pub(crate) fn create_site(store: &Store, site: &Site) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO sites (id, org, slug, display_name, tags, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &site.id.to_string(),
            &site.org,
            &site.slug,
            &site.display_name,
            &json_of(&site.tags),
            &ts_of(&site.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_sites(store: &Store, org: Option<&str>) -> Result<Vec<Site>> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {SITE_COLS} FROM sites WHERE ($1::text IS NULL OR org = $1) ORDER BY org, slug"
    );
    let rows = client.query(sql.as_str(), &[&org])?;
    rows.iter().map(site_of).collect()
}

pub(crate) fn get_site(store: &Store, id: Uuid) -> Result<Site> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {SITE_COLS} FROM sites WHERE id = $1");
    let row = client
        .query_opt(sql.as_str(), &[&id.to_string()])?
        .ok_or(StoreError::NotFound("site"))?;
    site_of(&row)
}

pub(crate) fn delete_site(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM sites WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("site"));
    }
    Ok(())
}
