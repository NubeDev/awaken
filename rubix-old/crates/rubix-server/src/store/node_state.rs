//! Durable node state: a board-scoped key-value store backing
//! `rubix_flow::StatePolicy::Durable`, so a stateful flow node (a counter, an
//! integrator…) can retain state across a server restart. Keyed by
//! `(board_id, node, key)`; the value is the node's opaque JSON blob, stored as
//! text (like `boards.graph`). Backend dispatch mirrors the rest of the store.

use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use super::backend::Backend;
use super::error::StoreError;
use super::{Result, Store};

impl Store {
    /// Load a durable node-state blob, or `None` if unset.
    pub fn get_node_state(&self, board_id: &str, node: &str, key: &str) -> Result<Option<Value>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let text: Option<String> = conn
                    .query_row(
                        "SELECT value FROM node_state \
                         WHERE board_id = ?1 AND node = ?2 AND key = ?3",
                        params![board_id, node, key],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?;
                match text {
                    Some(s) => Ok(Some(
                        serde_json::from_str(&s).map_err(|e| StoreError::Db(e.into()))?,
                    )),
                    None => Ok(None),
                }
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::node_state::get(self, board_id, node, key),
        }
    }

    /// Insert or replace a durable node-state blob.
    pub fn set_node_state(
        &self,
        board_id: &str,
        node: &str,
        key: &str,
        value: &Value,
    ) -> Result<()> {
        let text = serde_json::to_string(value).map_err(|e| StoreError::Db(e.into()))?;
        match &self.backend {
            Backend::Sqlite(_) => {
                self.sqlite_conn()?.execute(
                    "INSERT INTO node_state (board_id, node, key, value) \
                     VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT(board_id, node, key) DO UPDATE SET value = excluded.value",
                    params![board_id, node, key, text],
                )?;
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::node_state::set(self, board_id, node, key, &text)
            }
        }
    }
}
