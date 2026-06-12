//! [`BoardAccess`] over the store: loads a board JSON into a reflow `Network`
//! backed by [`StorePointAccess`] and runs it once. Backs the agent `run_board`
//! tool with the same execution path as `POST /boards/run`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_flow::BoardGraph;
use rubix_tools::BoardAccess;

use crate::flow::StorePointAccess;
use crate::store::Store;

/// Store-backed board runner handed to the agent's `run_board` tool.
pub struct StoreBoardAccess {
    store: Store,
}

impl StoreBoardAccess {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

#[async_trait]
impl BoardAccess for StoreBoardAccess {
    async fn run_board(&self, board: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let graph: BoardGraph =
            serde_json::from_value(board).map_err(|e| anyhow::anyhow!("invalid board: {e}"))?;
        let access = Arc::new(StorePointAccess::new(self.store.clone()));
        let outputs = graph
            .run(access)
            .await
            .map_err(|e| anyhow::anyhow!("run board: {e}"))?;
        Ok(serde_json::json!({ "outputs": outputs }))
    }
}
