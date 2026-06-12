//! Postgres row decoding shared by the resource bodies. Mirrors the SQLite
//! [`super::super::codec`] text encoding: ids are canonical UUID strings,
//! timestamps RFC 3339, tag/JSON columns serde text. Existence checks for the
//! foreign-key parents live here too (the SQLite path uses `require_*` against
//! a `rusqlite::Connection`, which the Postgres client cannot share).

use rubix_core::{Point, PointKind, Site, TagSet};
use uuid::Uuid;

use super::super::codec::{json_to, ts_to};
use super::super::{Result, StoreError};

/// Parse a TEXT id column into a [`Uuid`].
pub(super) fn uuid_of(row: &postgres::Row, idx: usize) -> Result<Uuid> {
    let raw: String = row.get(idx);
    Uuid::parse_str(&raw).map_err(|e| StoreError::Db(anyhow::anyhow!("bad uuid `{raw}`: {e}")))
}

/// Decode a JSON TEXT column.
pub(super) fn json_col<T: serde::de::DeserializeOwned>(
    row: &postgres::Row,
    idx: usize,
) -> Result<T> {
    let raw: String = row.get(idx);
    json_to(&raw).map_err(|e| StoreError::Db(anyhow::anyhow!("bad json column: {e}")))
}

/// Decode an RFC 3339 TEXT timestamp column.
pub(super) fn ts_col(row: &postgres::Row, idx: usize) -> Result<chrono::DateTime<chrono::Utc>> {
    let raw: String = row.get(idx);
    ts_to(&raw).map_err(|e| StoreError::Db(anyhow::anyhow!("bad timestamp column: {e}")))
}

/// Decode a snake_case enum stored as a bare token (the serde repr without
/// quotes), matching the SQLite `kind`/`severity`/widget-`kind` columns.
pub(super) fn token_enum<T: serde::de::DeserializeOwned>(
    row: &postgres::Row,
    idx: usize,
) -> Result<T> {
    let raw: String = row.get(idx);
    json_to(&format!("\"{raw}\"")).map_err(|e| StoreError::Db(anyhow::anyhow!("bad enum: {e}")))
}

/// Map a [`Site`] row (`SITE_COLS` order).
pub(super) fn site_of(row: &postgres::Row) -> Result<Site> {
    Ok(Site {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        slug: row.get(2),
        display_name: row.get(3),
        tags: json_col(row, 4)?,
        created_at: ts_col(row, 5)?,
    })
}

/// Map a [`Point`] row (`POINT_COLS` order).
pub(super) fn point_of(row: &postgres::Row) -> Result<Point> {
    let kind: PointKind = token_enum(row, 4)?;
    let cur_value = match row.get::<_, Option<String>>(8) {
        Some(s) => Some(json_to(&s).map_err(|e| StoreError::Db(anyhow::anyhow!("bad cur: {e}")))?),
        None => None,
    };
    let cur_ts = match row.get::<_, Option<String>>(9) {
        Some(s) => Some(ts_to(&s).map_err(|e| StoreError::Db(anyhow::anyhow!("bad cur_ts: {e}")))?),
        None => None,
    };
    Ok(Point {
        id: uuid_of(row, 0)?,
        equip_id: uuid_of(row, 1)?,
        slug: row.get(2),
        display_name: row.get(3),
        kind,
        unit: row.get(5),
        tags: json_col(row, 6)?,
        priority_array: json_col(row, 7)?,
        cur_value,
        cur_ts,
        created_at: ts_col(row, 10)?,
    })
}

/// In-Rust tag filtering, identical to the SQLite path.
pub(super) fn filter_tags<T>(
    items: Vec<T>,
    tags: &[String],
    get: impl Fn(&T) -> &TagSet,
) -> Vec<T> {
    if tags.is_empty() {
        return items;
    }
    items
        .into_iter()
        .filter(|item| get(item).has_all(tags.iter().map(String::as_str)))
        .collect()
}

/// Confirm a parent row exists, returning [`StoreError::NotFound`] otherwise.
/// Generic over `Client`/`Transaction` via [`postgres::GenericClient`].
pub(super) fn require<C: postgres::GenericClient>(
    client: &mut C,
    table: &str,
    what: &'static str,
    id: Uuid,
) -> Result<()> {
    let sql = format!("SELECT 1 FROM {table} WHERE id = $1");
    let n = client.query(sql.as_str(), &[&id.to_string()])?;
    if n.is_empty() {
        return Err(StoreError::NotFound(what));
    }
    Ok(())
}
