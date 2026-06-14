//! Durable node state, Postgres backend. Mirrors [`super::super::node_state`].
//! `value` is the node's JSON blob stored as TEXT. Cloud-feature only.

use serde_json::Value;

use super::super::error::StoreError;
use super::super::{Result, Store};

pub(in crate::store) fn get(
    store: &Store,
    board_id: &str,
    node: &str,
    key: &str,
) -> Result<Option<Value>> {
    let mut client = store.postgres_conn()?;
    let row = client.query_opt(
        "SELECT value FROM node_state WHERE board_id = $1 AND node = $2 AND key = $3",
        &[&board_id, &node, &key],
    )?;
    match row {
        Some(row) => {
            let text: String = row.get(0);
            Ok(Some(
                serde_json::from_str(&text).map_err(|e| StoreError::Db(e.into()))?,
            ))
        }
        None => Ok(None),
    }
}

pub(in crate::store) fn set(
    store: &Store,
    board_id: &str,
    node: &str,
    key: &str,
    value_text: &str,
) -> Result<()> {
    let mut client = store.postgres_conn()?;
    client.execute(
        "INSERT INTO node_state (board_id, node, key, value) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (board_id, node, key) DO UPDATE SET value = excluded.value",
        &[&board_id, &node, &key, &value_text],
    )?;
    Ok(())
}
