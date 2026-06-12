//! Board rows, Postgres backend. Mirrors [`super::super::boards`].

use super::super::boards::BOARD_COLS;
use super::super::codec::{json_of, ts_of};
use super::super::{Result, Store, StoreError};
use super::codec::{json_col, ts_col, uuid_of};
use crate::scheduler::BoardRecord;

fn board_of(row: &postgres::Row) -> Result<BoardRecord> {
    Ok(BoardRecord {
        id: uuid_of(row, 0)?,
        slug: row.get(1),
        version: row.get(2),
        display_name: row.get(3),
        enabled: row.get(4),
        trigger: json_col(row, 5)?,
        graph: json_col(row, 6)?,
        created_at: ts_col(row, 7)?,
    })
}

pub(crate) fn create_board(store: &Store, board: &BoardRecord) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO boards (id, slug, version, display_name, enabled, trigger, graph, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &board.id.to_string(),
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

pub(crate) fn next_board_version(store: &Store, slug: &str) -> Result<i64> {
    let mut client = store.postgres_conn()?;
    let row = client.query_one("SELECT MAX(version) FROM boards WHERE slug = $1", &[&slug])?;
    let max: Option<i64> = row.get(0);
    Ok(max.unwrap_or(0) + 1)
}

pub(crate) fn latest_boards(store: &Store) -> Result<Vec<BoardRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {BOARD_COLS} FROM boards b WHERE version = \
         (SELECT MAX(version) FROM boards WHERE slug = b.slug) ORDER BY slug"
    );
    let rows = client.query(sql.as_str(), &[])?;
    rows.iter().map(board_of).collect()
}

pub(crate) fn get_board(store: &Store, slug: &str) -> Result<BoardRecord> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {BOARD_COLS} FROM boards WHERE slug = $1 ORDER BY version DESC LIMIT 1"
    );
    let row = client
        .query_opt(sql.as_str(), &[&slug])?
        .ok_or(StoreError::NotFound("board"))?;
    board_of(&row)
}

pub(crate) fn delete_board(store: &Store, slug: &str) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM boards WHERE slug = $1", &[&slug])?;
    if n == 0 {
        return Err(StoreError::NotFound("board"));
    }
    Ok(())
}
