//! Board rows, Postgres backend. Mirrors [`super::super::boards`]. Boards carry
//! an `org` + optional `site_id` scope; `site_id IS NOT DISTINCT FROM $n` does
//! the NULL-aware comparison the SQLite `IS` does.

use uuid::Uuid;

use super::super::boards::BOARD_COLS;
use super::super::codec::{json_of, ts_of};
use super::super::{Result, Store, StoreError};
use super::codec::{json_col, ts_col, uuid_of};
use crate::scheduler::BoardRecord;

fn board_of(row: &postgres::Row) -> Result<BoardRecord> {
    let site_id = row
        .get::<_, Option<String>>(2)
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| StoreError::Db(anyhow::anyhow!("bad board site_id uuid: {e}")))?;
    Ok(BoardRecord {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        site_id,
        slug: row.get(3),
        version: row.get(4),
        display_name: row.get(5),
        enabled: row.get(6),
        trigger: json_col(row, 7)?,
        graph: json_col(row, 8)?,
        created_at: ts_col(row, 9)?,
    })
}

pub(crate) fn create_board(store: &Store, board: &BoardRecord) -> Result<()> {
    let mut client = store.postgres_conn()?;
    if let Some(site_id) = board.site_id {
        super::codec::require(&mut *client, "sites", "site", site_id)?;
    }
    let site_id = board.site_id.map(|s| s.to_string());
    client.execute(
        "INSERT INTO boards \
             (id, org, site_id, slug, version, display_name, enabled, trigger, graph, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        &[
            &board.id.to_string(),
            &board.org,
            &site_id,
            &board.slug,
            &board.version,
            &board.display_name,
            &board.enabled,
            &json_of(&board.trigger),
            &json_of(&board.graph),
            &ts_of(&board.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn next_board_version(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    slug: &str,
) -> Result<i64> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let row = client.query_one(
        "SELECT MAX(version) FROM boards \
         WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND slug = $3",
        &[&org, &site, &slug],
    )?;
    let max: Option<i64> = row.get(0);
    Ok(max.unwrap_or(0) + 1)
}

pub(crate) fn latest_boards_all(store: &Store) -> Result<Vec<BoardRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {BOARD_COLS} FROM boards b WHERE version = (\
             SELECT MAX(version) FROM boards \
             WHERE org = b.org AND site_id IS NOT DISTINCT FROM b.site_id AND slug = b.slug) \
         ORDER BY b.slug"
    );
    let rows = client.query(sql.as_str(), &[])?;
    rows.iter().map(board_of).collect()
}

pub(crate) fn latest_boards(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> Result<Vec<BoardRecord>> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let sql = format!(
        "SELECT {BOARD_COLS} FROM boards b WHERE version = (\
             SELECT MAX(version) FROM boards \
             WHERE org = b.org AND site_id IS NOT DISTINCT FROM b.site_id AND slug = b.slug) \
         AND b.org = $1 AND ($2::text IS NULL OR b.site_id = $2) \
         ORDER BY b.slug"
    );
    let rows = client.query(sql.as_str(), &[&org, &site])?;
    rows.iter().map(board_of).collect()
}

pub(crate) fn get_board(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    slug: &str,
) -> Result<BoardRecord> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let sql = format!(
        "SELECT {BOARD_COLS} FROM boards \
         WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND slug = $3 \
         ORDER BY version DESC LIMIT 1"
    );
    let row = client
        .query_opt(sql.as_str(), &[&org, &site, &slug])?
        .ok_or(StoreError::NotFound("board"))?;
    board_of(&row)
}

pub(crate) fn get_board_by_id(store: &Store, id: Uuid) -> Result<BoardRecord> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {BOARD_COLS} FROM boards WHERE id = $1");
    let row = client
        .query_opt(sql.as_str(), &[&id.to_string()])?
        .ok_or(StoreError::NotFound("board"))?;
    board_of(&row)
}

pub(crate) fn update_board(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    slug: &str,
    display_name: Option<&str>,
    enabled: Option<bool>,
) -> Result<BoardRecord> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let row = client
        .query_opt(
            &format!(
                "UPDATE boards SET \
                 display_name = COALESCE($4, display_name), \
                 enabled = COALESCE($5, enabled) \
                 WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND slug = $3 \
                   AND version = (SELECT MAX(version) FROM boards \
                       WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND slug = $3) \
                 RETURNING {BOARD_COLS}"
            ),
            &[&org, &site, &slug, &display_name, &enabled],
        )?
        .ok_or(StoreError::NotFound("board"))?;
    board_of(&row)
}

pub(crate) fn delete_board(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    slug: &str,
) -> Result<()> {
    let site = site_id.map(|s| s.to_string());
    let n = store.postgres_conn()?.execute(
        "DELETE FROM boards WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND slug = $3",
        &[&org, &site, &slug],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("board"));
    }
    Ok(())
}
